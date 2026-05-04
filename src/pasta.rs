use serde::{Deserialize, Serialize};
use std::fmt;
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pasta {
    pub slug: String,
    pub content: String,
    pub allow_edit: bool,
    pub created: DateTime<Utc>,
    pub expiration: Option<DateTime<Utc>>,
    pub last_read: DateTime<Utc>,
    pub read_count: u64,
    pub burn_after_reads: u64,
}

impl Pasta {
    pub fn slug(&self) -> &str {
        &self.slug
    }

    pub fn pasta_type(&self) -> &str {
        if crate::util::misc::is_valid_url(&self.content) {
            "URL"
        } else {
            "TEXT"
        }
    }

    pub fn total_size_as_string(&self) -> String {
        let bytes = self.content.len();
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        }
    }

    pub fn short_last_read_time_ago_as_string(&self) -> String {
        let now = Utc::now();
        let diff = (now - self.last_read).num_seconds();
        if diff < 60 {
            format!("{}s ago", diff)
        } else if diff < 3600 {
            format!("{}m ago", diff / 60)
        } else if diff < 86400 {
            format!("{}h ago", diff / 3600)
        } else {
            format!("{}d ago", diff / 86400)
        }
    }
}

impl fmt::Display for Pasta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.content)
    }
}
