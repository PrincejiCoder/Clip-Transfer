use actix_web::{get, post, web, HttpResponse, Responder};
use crate::AppState;
use crate::pasta::Pasta;
use crate::args::{Args, ARGS};
use askama::Template;
use serde::Deserialize;
use regex::Regex;
use lazy_static::lazy_static;
use chrono::{DateTime, Utc, Duration};
use crate::util::misc::string_to_qr_svg;

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
    content: Option<String>,
}

#[derive(Template)]
#[template(path = "view.html")]
struct ViewTemplate<'a> {
    args: &'a Args,
    slug: String,
    pasta: &'a Pasta,
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

#[derive(Template)]
#[template(path = "offline.html")]
struct OfflineTemplate<'a> {
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

fn expiration_to_timestamp(expiration: &str, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    match expiration {
        "1hour" => Some(now + Duration::hours(1)),
        "24hour" => Some(now + Duration::days(1)),
        "1week" => Some(now + Duration::weeks(1)),
        "1month" => Some(now + Duration::days(30)),
        "never" => None,
        _ => Some(now + Duration::weeks(1)), // default 1 week
    }
}

fn apply_security_headers(mut res: HttpResponse) -> HttpResponse {
    res.headers_mut().insert(
        actix_web::http::header::X_CONTENT_TYPE_OPTIONS,
        actix_web::http::header::HeaderValue::from_static("nosniff"),
    );
    res.headers_mut().insert(
        actix_web::http::header::X_FRAME_OPTIONS,
        actix_web::http::header::HeaderValue::from_static("SAMEORIGIN"),
    );
    res
}

#[get("/")]
pub async fn homepage() -> impl Responder {
    let s = HomeTemplate { args: &ARGS }.render().unwrap();
    apply_security_headers(HttpResponse::Ok().content_type("text/html; charset=utf-8").body(s))
}

#[get("/{slug}")]
pub async fn get_slug(
    state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<QueryParams>,
) -> impl Responder {
    let raw_slug = path.into_inner();
    let slug = normalize_slug(&raw_slug);

    // Redirect to canonical URL if not normalized
    if raw_slug != slug {
        return HttpResponse::SeeOther()
            .append_header(("Location", format!("/{}", slug)))
            .finish();
    }

    if !SLUG_REGEX.is_match(&slug) {
        return apply_security_headers(HttpResponse::BadRequest().body("Invalid keyword format."));
    }

    let now = Utc::now();

    // 1. Share Mode (?created=true) - PURE UI
    if query.created.as_deref() == Some("true") {
        let pastas = state.pastas.read().unwrap_or_else(|e| e.into_inner());
        if let Some(_) = pastas.get(&slug) {
            let mut base_url = ARGS.public_path_as_str();
            if base_url.is_empty() {
                base_url = "http://localhost:8080".to_string();
            } else if !base_url.contains("://") {
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
            return apply_security_headers(HttpResponse::Ok().content_type("text/html; charset=utf-8").body(s));
        }
    }

    // Lock for both read/write to handle expiry and view count atomically
    let mut pastas = state.pastas.write().unwrap_or_else(|e| e.into_inner());
    
    if let Some(mut pasta) = pastas.remove(&slug) {
        // 2. Expired State (Expiry or Burn limit)
        let is_expired = pasta.expiration.map_or(false, |e| e <= now);
        let is_burned = pasta.burn_after_reads > 0 && pasta.read_count >= pasta.burn_after_reads;

        if is_expired || is_burned {
            let s = ExpiredTemplate { args: &ARGS }.render().unwrap();
            return apply_security_headers(HttpResponse::Gone().content_type("text/html; charset=utf-8").body(s));
        }

        // 3. Edit Mode
        if pasta.allow_edit {
            let existing_content = pasta.content.clone();
            pastas.insert(slug.clone(), pasta);
            let s = CreateTemplate { args: &ARGS, slug, content: Some(existing_content) }.render().unwrap();
            return apply_security_headers(HttpResponse::Ok().content_type("text/html; charset=utf-8").body(s));
        }

        // 4. View Mode
        pasta.read_count += 1;
        pasta.last_read = now;
        
        let s = ViewTemplate {
            args: &ARGS,
            slug: slug.clone(),
            pasta: &pasta,
            can_edit: pasta.allow_edit,
        }.render().unwrap();

        // If it reaches burn limit after this view, don't re-insert
        if pasta.burn_after_reads == 0 || pasta.read_count < pasta.burn_after_reads {
            pastas.insert(slug, pasta);
        }

        return apply_security_headers(HttpResponse::Ok().content_type("text/html; charset=utf-8").body(s));
    }

    // 5. Create Mode (Fallback)
    let s = CreateTemplate { args: &ARGS, slug, content: None }.render().unwrap();
    apply_security_headers(HttpResponse::Ok().content_type("text/html; charset=utf-8").body(s))
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
        return apply_security_headers(HttpResponse::Forbidden().body("Reserved keyword."));
    }

    if !SLUG_REGEX.is_match(&slug) {
        return apply_security_headers(HttpResponse::BadRequest().body("Invalid keyword format."));
    }

    // Enforce 1MB limit
    if form.content.len() > 1024 * 1024 {
        return apply_security_headers(HttpResponse::BadRequest().body("Content too large."));
    }

    let mut pastas = state.pastas.write().unwrap_or_else(|e| e.into_inner());
    let now = Utc::now();

    if let Some(existing) = pastas.get(&slug) {
        if !existing.allow_edit {
             return apply_security_headers(HttpResponse::Conflict().body("Slug exists and is read-only."));
        }
    }

    let pasta = Pasta {
        slug: slug.clone(),
        content: form.content.clone(),
        allow_edit: form.allow_edit.is_some(),
        created: now,
        expiration: expiration_to_timestamp(&form.expiration, now),
        last_read: now,
        read_count: 0,
        burn_after_reads: form.burn_after,
    };

    pastas.insert(slug.clone(), pasta);

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
    let pastas = state.pastas.read().unwrap_or_else(|e| e.into_inner());

    if let Some(pasta) = pastas.get(&slug) {
        return apply_security_headers(
            HttpResponse::Ok()
                .content_type("text/plain; charset=utf-8")
                .body(pasta.content.clone())
        );
    }

    apply_security_headers(HttpResponse::NotFound().finish())
}

#[get("/offline")]
pub async fn offline() -> impl Responder {
    let s = OfflineTemplate { args: &ARGS }.render().unwrap();
    apply_security_headers(HttpResponse::Ok().content_type("text/html; charset=utf-8").body(s))
}
