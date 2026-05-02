use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pasta {
    pub slug: String,
    pub content: String,
    pub allow_edit: bool,
    pub created: i64,
    pub expiration: i64,
    pub last_read: i64,
    pub read_count: u64,
    pub burn_after_reads: u64,
}

impl Pasta {
    pub fn slug(\u0026self) -\u003e \u0026str {
        \u0026self.slug
    }

    pub fn pasta_type(\u0026self) -\u003e \u0026str {
        if crate::util::misc::is_valid_url(\u0026self.content) {
            \"URL\"
        } else {
            \"TEXT\"
        }
    }

    pub fn total_size_as_string(\u0026self) -\u003e String {
        let bytes = self.content.len();
        if bytes \u003c 1024 {
            format!(\"{} B\", bytes)
        } else if bytes \u003c 1024 * 1024 {
            format!(\"{:.1} KB\", bytes as f64 / 1024.0)
        } else {
            format!(\"{:.1} MB\", bytes as f64 / (1024.0 * 1024.0))
        }
    }

    pub fn short_last_read_time_ago_as_string(\u0026self) -\u003e String {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        let diff = now - self.last_read;
        if diff \u003c 60 {
            format!(\"{}s ago\", diff)
        } else if diff \u003c 3600 {
            format!(\"{}m ago\", diff / 60)
        } else if diff \u003c 86400 {
            format!(\"{}h ago\", diff / 3600)
        } else {
            format!(\"{}d ago\", diff / 86400)
        }
    }

    pub fn content_syntax_highlighted(\u0026self) -\u003e String {
        html_escape::encode_text(\u0026self.content).to_string()
    }
}

impl fmt::Display for Pasta {
    fn fmt(\u0026self, f: \u0026mut fmt::Formatter\u003c'_\u003e) -\u003e fmt::Result {
        write!(f, \"{}\", self.content)
    }
}
