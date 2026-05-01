extern crate core;

use crate::args::ARGS;
use crate::endpoints::{
    errors, static_resources,
};
use crate::pasta::Pasta;
use crate::util::db::read_all;
use crate::util::telemetry::start_telemetry_thread;
use actix_web::{middleware, web, App, HttpServer};
use chrono::Local;
use env_logger::Builder;
use log::LevelFilter;
use std::fs;
use std::io::Write;
use std::collections::HashMap;
use std::sync::Mutex;

use std::thread;
use std::time::Duration;

pub mod args;
pub mod pasta;

pub mod util {
    pub mod animalnumbers;
    pub mod auth;
    pub mod db;
    pub mod db_json;
    #[cfg(feature = "default")]
    pub mod db_sqlite;
    pub mod hashids;
    pub mod misc;
    pub mod syntaxhighlighter;
    pub mod telemetry;
    pub mod version;
    pub mod http_client;
}

pub mod endpoints {
    pub mod errors;
    pub mod static_resources;
    pub mod core_routes;
}

pub struct AppState {
    pub pastas: Mutex<HashMap<String, Pasta>>,
}

fn start_cleanup_thread(state: web::Data<AppState>) {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(60));
            if let Ok(mut pastas) = state.pastas.lock() {
                crate::util::misc::remove_expired(&mut pastas);
            }
        }
    });
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .filter(None, LevelFilter::Info)
        .init();

    log::info!(
        "LinkDrop starting on http://{}:{} (Public URL: {})",
        ARGS.bind.to_string(),
        ARGS.port.to_string(),
        if ARGS.public_path.is_some() { ARGS.public_path_as_str() } else { "None".to_string() }
    );

    match fs::create_dir_all(format!("{}/public", ARGS.data_dir)) {
        Ok(dir) => dir,
        Err(error) => {
            log::error!(
                "Couldn't create data directory {}/attachments/: {:?}",
                ARGS.data_dir,
                error
            );
            panic!(
                "Couldn't create data directory {}/attachments/: {:?}",
                ARGS.data_dir, error
            );
        }
    };

    let data = web::Data::new(AppState {
        pastas: Mutex::new(read_all()),
    });

    if !ARGS.disable_telemetry {
        start_telemetry_thread();
    }

    start_cleanup_thread(data.clone());

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .app_data(web::PayloadConfig::new(1024 * 1024)) // 1MB Limit
            .wrap(middleware::NormalizePath::trim())
            .wrap(
                middleware::Logger::new(r#"%{r}a "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T"#)
            )
            // Conditional / Public Services
            // Core URL-Driven Routes
            .service(endpoints::core_routes::homepage)
            .service(endpoints::core_routes::get_raw) // /raw/{slug} priority
            .service(static_resources::static_resources) // /static priority
            .service(endpoints::core_routes::get_slug) // /{slug} last
            .service(endpoints::core_routes::post_slug)
            .default_service(web::route().to(errors::not_found))
    })
    .bind((ARGS.bind, ARGS.port))?
    .workers(ARGS.threads as usize)
    .run()
    .await
}
