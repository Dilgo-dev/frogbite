use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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
