use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use frogbite::core::http::HttpResponse;
use serde::{Deserialize, Serialize};

use crate::assertions::AssertionResult;

/// Authentication configuration for a request.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum Auth {
    #[default]
    None,
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
    ApiKey {
        header: String,
        value: String,
    },
}

/// Body content type for a request.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum BodyType {
    #[default]
    Raw,
    Form,
    Multipart,
}

impl BodyType {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Raw => "Raw",
            Self::Form => "Form",
            Self::Multipart => "Multipart",
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::Raw => Self::Form,
            Self::Form => Self::Multipart,
            Self::Multipart => Self::Raw,
        }
    }
}

/// Content type for raw body mode.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum ContentType {
    #[default]
    Json,
    Text,
    Xml,
    Html,
}

impl ContentType {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Json => "JSON",
            Self::Text => "Text",
            Self::Xml => "XML",
            Self::Html => "HTML",
        }
    }

    pub const fn mime(self) -> &'static str {
        match self {
            Self::Json => "application/json",
            Self::Text => "text/plain",
            Self::Xml => "application/xml",
            Self::Html => "text/html",
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::Json => Self::Text,
            Self::Text => Self::Xml,
            Self::Xml => Self::Html,
            Self::Html => Self::Json,
        }
    }
}

/// A saved request in a collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedRequest {
    pub id: String,
    pub name: String,
    pub method: String,
    pub url: String,
    pub body: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub folder_id: Option<String>,
    #[serde(default)]
    pub auth: Auth,
    #[serde(default)]
    pub body_type: BodyType,
    #[serde(default)]
    pub content_type: ContentType,
    #[serde(default)]
    pub form_data: Vec<(String, String)>,
    #[serde(default = "default_true")]
    pub follow_redirects: bool,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default = "default_true")]
    pub verify_tls: bool,
    #[serde(default)]
    pub ca_cert_path: String,
    #[serde(default)]
    pub client_cert_path: String,
    #[serde(default)]
    pub client_key_path: String,
    #[serde(default)]
    pub tls_min_version: String,
    #[serde(default)]
    pub extractors: Vec<(String, String)>,
    #[serde(default)]
    pub assertions: Vec<String>,
    #[serde(default)]
    pub last_response: Option<HttpResponse>,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub last_assertion_results: Vec<AssertionResult>,
}

pub const fn default_timeout() -> u64 {
    30
}

const fn default_true() -> bool {
    true
}

/// A folder grouping requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub expanded: bool,
}

/// Persistent collection state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CollectionData {
    pub folders: Vec<Folder>,
    pub requests: Vec<SavedRequest>,
    pub active_request_id: Option<String>,
}

fn collections_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let path = PathBuf::from(home).join(".config").join("frogbite");
    let _ = fs::create_dir_all(&path);
    path.join("collections.json")
}

/// Loads collections from disk.
pub fn load() -> CollectionData {
    let path = collections_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

/// Persists collections to disk.
pub fn save(data: &CollectionData) {
    if let Ok(json) = serde_json::to_string_pretty(data) {
        let _ = fs::write(collections_path(), json);
    }
}

/// Generates a short unique ID.
pub fn new_id() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_micros());
    format!("{ts:x}")
}
