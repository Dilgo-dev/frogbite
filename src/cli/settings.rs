use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// User preferences persisted in ~/.config/frogbite/settings.json.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub splash_animation: bool,
    pub vim_keys: bool,
    #[serde(default = "default_true")]
    pub update_check: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub ws_auto_reconnect: bool,
}

const fn default_true() -> bool {
    true
}

fn default_theme() -> String {
    "scooby".to_owned()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            splash_animation: true,
            vim_keys: true,
            update_check: true,
            theme: default_theme(),
            ws_auto_reconnect: false,
        }
    }
}

fn settings_path() -> PathBuf {
    let config_dir = dirs_path();
    config_dir.join("settings.json")
}

fn dirs_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let path = PathBuf::from(home).join(".config").join("frogbite");
    let _ = fs::create_dir_all(&path);
    path
}

/// Loads settings from disk, falling back to defaults.
pub fn load() -> Settings {
    let path = settings_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

/// Persists settings to disk as JSON.
pub fn save(settings: &Settings) {
    let path = settings_path();
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = fs::write(path, json);
    }
}
