use actix_web::{get, post, web, HttpResponse, Responder};
use crate::AppState;
use crate::pasta::Pasta;
use crate::args::{Args, ARGS};
use askama::Template;
use serde::Deserialize;
use crate::util::db;
use regex::Regex;
use lazy_static::lazy_static;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::util::misc::{self, is_valid_url};

lazy_static! {
    static ref SLUG_REGEX: Regex = Regex::new(r"^[a-z0-9-_]{1,50}$").unwrap();
    static ref RESERVED_SLUGS: Vec<&'static str> = vec!["static", "public", "favicon.ico", "raw", "api", "admin"];
}

#[derive(Deserialize)]
pub struct QueryParams {
    pub created: Option<bool>,
}

#[derive(Deserialize)]
pub struct PastaForm {
    pub content: String,
    pub expiration: String,
    pub burn_after: u64,
    pub allow_edit: Option<String>,
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate<'a> {
    args: &'a Args,
}

#[derive(Template)]
#[template(path = "paste.html", escape = "none")]
struct PasteTemplate<'a> {
    args: &'a Args,
    slug: String,
    mode: String,
    pasta: Option<&'a Pasta>,
    error: Option<String>,
    qr: Option<String>,
}

fn normalize_slug(slug: &str) -> String {
    slug.to_lowercase()
        .trim()
        .replace(' ', "-")
        .trim_matches('-')
        .chars()
        .filter(|&c| c.is_alphanumeric() || c == '-' || c == '_')
        .collect::<String>()
}

fn expiration_to_timestamp(expiration: &str, timenow: i64) -> i64 {
    match expiration {
        "1min" => timenow + 60,
        "10min" => timenow + 60 * 10,
        "1hour" => timenow + 60 * 60,
        "24hour" => timenow + 60 * 60 * 24,
        "1week" => timenow + 60 * 60 * 24 * 7,
        "1month" => timenow + 60 * 60 * 24 * 30,
        "never" => 0,
        _ => timenow + 60 * 60 * 24 * 7, // default 1 week
    }
}

#[get("/")]
pub async fn homepage() -> impl Responder {
    HttpResponse::Ok().content_type("text/html").body(
        HomeTemplate { args: &ARGS }.render().unwrap()
    )
}

#[get("/{slug}")]
pub async fn get_slug(
    data: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<QueryParams>,
) -> impl Responder {
    let raw_slug = path.into_inner();
    let slug = normalize_slug(&raw_slug);
    
    if !SLUG_REGEX.is_match(&slug) {
        return HttpResponse::BadRequest().body("Invalid slug.");
    }
    
    let mut pastas = data.pastas.lock().unwrap();
    let timenow = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;

    if pastas.contains_key(&slug) {
        let is_expired;
        let is_burnt;
        
        {
            let pasta = pastas.get(&slug).unwrap();
            is_expired = pasta.expiration > 0 && pasta.expiration < timenow;
            is_burnt = pasta.burn_after_reads > 0 && pasta.read_count >= pasta.burn_after_reads;
        }

        if is_expired {
            pastas.remove(&slug);
            db::delete(&pastas, &slug);
            return HttpResponse::Ok().content_type("text/html").body(
                PasteTemplate {
                    args: &ARGS,
                    slug: slug.clone(),
                    mode: "create".to_string(),
                    pasta: None,
                    error: Some("This paste has expired and was deleted.".to_string()),
                    qr: None,
                }.render().unwrap()
            );
        }

        if is_burnt {
            pastas.remove(&slug);
            db::delete(&pastas, &slug);
             return HttpResponse::Ok().content_type("text/html").body(
                PasteTemplate {
                    args: &ARGS,
                    slug: slug.clone(),
                    mode: "create".to_string(),
                    pasta: None,
                    error: Some("This paste reached its view limit and was deleted.".to_string()),
                    qr: None,
                }.render().unwrap()
            );
        }

        // Update counts
        {
            let pasta = pastas.get_mut(&slug).unwrap();
            pasta.read_count += 1;
            pasta.last_read = timenow;
        }
        
        // Re-borrow immutably for database sync and rendering
        let pasta = pastas.get(&slug).unwrap();
        db::update(&pastas, pasta);
        
        let qr_svg = Some(misc::string_to_qr_svg(&format!("{}/{}", ARGS.public_path_as_str(), slug)));

        if query.created.unwrap_or(false) {
            return HttpResponse::Ok().content_type("text/html").body(
                PasteTemplate {
                    args: &ARGS,
                    slug: slug.clone(),
                    mode: "share".to_string(),
                    pasta: Some(pasta),
                    error: None,
                    qr: qr_svg,
                }.render().unwrap()
            );
        }
        
        let mode = if pasta.allow_edit { "edit" } else { "view" };
        return HttpResponse::Ok().content_type("text/html").body(
            PasteTemplate {
                args: &ARGS,
                slug: slug.clone(),
                mode: mode.to_string(),
                pasta: Some(pasta),
                error: None,
                qr: qr_svg,
            }.render().unwrap()
        );
    }
    
    HttpResponse::Ok().content_type("text/html").body(
        PasteTemplate {
            args: &ARGS,
            slug: slug.clone(),
            mode: "create".to_string(),
            pasta: None,
            error: None,
            qr: None,
        }.render().unwrap()
    )
}

#[post("/{slug}")]
pub async fn post_slug(
    data: web::Data<AppState>,
    path: web::Path<String>,
    form: web::Form<PastaForm>,
) -> impl Responder {
    let raw_slug = path.into_inner();
    let slug = normalize_slug(&raw_slug);
    
    if RESERVED_SLUGS.contains(&slug.as_str()) {
        return HttpResponse::Forbidden().body("This slug is reserved.");
    }
    
    if !SLUG_REGEX.is_match(&slug) {
        return HttpResponse::BadRequest().body("Invalid slug.");
    }

    let mut pastas = data.pastas.lock().unwrap();
    let timenow = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;

    if let Some(existing) = pastas.get(&slug) {
        if !existing.allow_edit {
            return HttpResponse::Forbidden().body("This paste is read-only.");
        }
    }

    let pasta_type = if is_valid_url(&form.content) { "url".to_string() } else { "text".to_string() };
    
    let new_pasta = Pasta {
        slug: slug.clone(),
        content: form.content.clone(),
        file: None,
        attachments: None,
        allow_edit: form.allow_edit.is_some(),
        burn_after_reads: form.burn_after,
        read_count: 0,
        expiration: expiration_to_timestamp(&form.expiration, timenow),
        created: timenow,
        last_read: timenow,
        pasta_type,
        extension: "txt".to_string(),
        private: false,
        readonly: !form.allow_edit.is_some(),
        encrypt_client: false,
        encrypt_server: false,
        encrypted_key: None,
    };

    let exists = pastas.contains_key(&slug);
    pastas.insert(slug.clone(), new_pasta.clone());
    
    if exists {
        db::update(&pastas, &new_pasta);
    } else {
        db::insert(&pastas, &new_pasta);
    }

    HttpResponse::Found().append_header(("Location", format!("/{}?created=true", slug))).finish()
}

#[get("/raw/{slug}")]
pub async fn get_raw(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let raw_slug = path.into_inner();
    let slug = normalize_slug(&raw_slug);
    let pastas = data.pastas.lock().unwrap();
    
    if let Some(pasta) = pastas.get(&slug) {
        return HttpResponse::Ok().content_type("text/plain").body(pasta.content.clone());
    }
    
    HttpResponse::NotFound().body("404 Not Found")
}
