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
use crate::util::misc::{is_valid_url, string_to_qr_svg};

lazy_static! {
    static ref SLUG_REGEX: Regex = Regex::new(r"^[a-z0-9-_]{3,50}$").unwrap();
    static ref RESERVED_SLUGS: Vec<&'static str> = vec!["static", "public", "favicon.ico", "raw", "api", "admin"];
}

#[derive(Deserialize)]
pub struct PastaForm {
    pub content: String,
    pub expiration: String,
    pub burn_after: u64,
    pub allow_edit: Option<String>,
}

#[derive(Deserialize)]
pub struct QueryParams {
    pub created: Option<String>,
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate<'a> {
    args: &'a Args,
}

#[derive(Template)]
#[template(path = "create.html")]
struct CreateTemplate<'a> {
    args: &'a Args,
    slug: String,
    content: String,
}

#[derive(Template)]
#[template(path = "view.html", escape = "none")]
struct ViewTemplate<'a> {
    args: &'a Args,
    slug: String,
    pasta: &'a Pasta,
    highlighted_content: String,
    can_edit: bool,
}

#[derive(Template)]
#[template(path = "share.html", escape = "none")]
struct ShareTemplate<'a> {
    args: &'a Args,
    slug: String,
    full_url: String,
    qr_svg: String,
}

#[derive(Template)]
#[template(path = "expired.html")]
struct ExpiredTemplate<'a> {
    args: &'a Args,
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
        "1hour" => timenow + 3600,
        "24hour" => timenow + 86400,
        "1week" => timenow + 604800,
        "1month" => timenow + 2592000,
        "never" => 0,
        _ => timenow + 604800, // default 1 week
    }
}

#[get("/")]
pub async fn homepage() -> impl Responder {
    let s = HomeTemplate { args: &ARGS }.render().unwrap();
    HttpResponse::Ok().content_type("text/html").body(s)
}

#[get("/{slug}")]
pub async fn get_slug(
    state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<QueryParams>,
) -> impl Responder {
    let raw_slug = path.into_inner();
    let slug = normalize_slug(&raw_slug);

    if raw_slug != slug {
        return HttpResponse::SeeOther()
            .append_header(("Location", format!("/{}", slug)))
            .finish();
    }

    if !SLUG_REGEX.is_match(&slug) {
        return HttpResponse::BadRequest().body("Invalid keyword format.");
    }

    let mut pastas = state.pastas.lock().unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // A. Share Mode (?created=true) - PURE UI
    if query.created.as_deref() == Some("true") {
        if let Some(pasta) = pastas.get(&slug) {
            let mut base_url = ARGS.public_path_as_str();
            
            if base_url.is_empty() {
                base_url = "http://localhost:8080".to_string();
            } else if !base_url.contains("://") {
                // If protocol is missing, default to https for production/ngrok
                base_url = format!("https://{}", base_url);
            }

            let base_url = base_url.trim_end_matches('/');
            let full_url = format!("{}/{}", base_url, slug);

            let qr_svg = string_to_qr_svg(&full_url);

            let s = ShareTemplate {
                args: &ARGS,
                slug,
                full_url,
                qr_svg,
            }.render().unwrap();
            return HttpResponse::Ok().content_type("text/html").body(s);
        }
    }

    // B. Check Existence
    if let Some(mut pasta) = pastas.remove(&slug) {
        // C. Expired State
        if pasta.expiration > 0 && pasta.expiration < now {
            db::delete(&pastas, &slug);
            let s = ExpiredTemplate { args: &ARGS }.render().unwrap();
            return HttpResponse::Gone().content_type("text/html").body(s);
        }

        // D. Burn-after-read Logic
        if pasta.burn_after_reads > 0 && pasta.read_count >= pasta.burn_after_reads {
            db::delete(&pastas, &slug);
            let s = ExpiredTemplate { args: &ARGS }.render().unwrap();
            return HttpResponse::Gone().content_type("text/html").body(s);
        }

        // E. Edit Mode
        if pasta.allow_edit {
             let existing_content = pasta.content.clone();
             pastas.insert(slug.clone(), pasta.clone());
             let s = CreateTemplate { args: &ARGS, slug, content: existing_content }.render().unwrap();
             return HttpResponse::Ok().content_type("text/html").body(s);
        }

        // F. View Mode
        pasta.read_count += 1;
        pasta.last_read = now;
        
        let highlighted = pasta.content_syntax_highlighted();
        let s = ViewTemplate {
            args: &ARGS,
            slug: slug.clone(),
            pasta: &pasta,
            highlighted_content: highlighted,
            can_edit: pasta.allow_edit,
        }.render().unwrap();

        if pasta.burn_after_reads > 0 && pasta.read_count >= pasta.burn_after_reads {
            db::delete(&pastas, &slug);
        } else {
            db::update(&pastas, &pasta);
            pastas.insert(slug, pasta);
        }

        return HttpResponse::Ok().content_type("text/html").body(s);
    }

    // G. Create Mode
    let s = CreateTemplate { args: &ARGS, slug, content: String::new() }.render().unwrap();
    HttpResponse::Ok().content_type("text/html").body(s)
}

#[post("/{slug}")]
pub async fn post_slug(
    state: web::Data<AppState>,
    path: web::Path<String>,
    form: web::Form<PastaForm>,
) -> impl Responder {
    let raw_slug = path.into_inner();
    let slug = normalize_slug(&raw_slug);

    if RESERVED_SLUGS.contains(&slug.as_str()) {
        return HttpResponse::Forbidden().body("Reserved keyword.");
    }

    if !SLUG_REGEX.is_match(&slug) {
        return HttpResponse::BadRequest().body("Invalid keyword format.");
    }

    if form.content.len() > 1024 * 1024 {
        return HttpResponse::BadRequest().body("Content too large.");
    }

    let mut pastas = state.pastas.lock().unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    if let Some(existing) = pastas.get(&slug) {
        if !existing.allow_edit {
             return HttpResponse::Conflict().body("Slug exists and is read-only.");
        }
    }

    let pasta_type = if is_valid_url(&form.content) { "url".to_string() } else { "text".to_string() };

    let pasta = Pasta {
        slug: slug.clone(),
        content: form.content.clone(),
        file: None,
        attachments: None,
        extension: "txt".to_string(),
        private: false,
        readonly: form.allow_edit.is_none(),
        allow_edit: form.allow_edit.is_some(),
        encrypt_server: false,
        encrypt_client: false,
        encrypted_key: None,
        created: now,
        expiration: expiration_to_timestamp(&form.expiration, now),
        last_read: now,
        read_count: 0,
        burn_after_reads: form.burn_after,
        pasta_type,
    };

    let exists = pastas.contains_key(&slug);
    pastas.insert(slug.clone(), pasta.clone());
    
    if exists {
        db::update(&pastas, &pasta);
    } else {
        db::insert(&pastas, &pasta);
    }

    HttpResponse::SeeOther()
        .append_header(("Location", format!("/{}?created=true", slug)))
        .finish()
}

#[get("/raw/{slug}")]
pub async fn get_raw(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let slug = normalize_slug(&path.into_inner());
    let pastas = state.pastas.lock().unwrap();

    if let Some(pasta) = pastas.get(&slug) {
        return HttpResponse::Ok()
            .content_type("text/plain; charset=utf-8")
            .body(pasta.content.clone());
    }

    HttpResponse::NotFound().finish()
}
