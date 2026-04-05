use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A single key-value variable within an environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub key: String,
    pub value: String,
}

/// A named environment containing variables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub variables: Vec<Variable>,
}

/// Persistent environment state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnvironmentData {
    pub environments: Vec<Environment>,
    pub active_id: Option<String>,
}

fn env_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let path = PathBuf::from(home).join(".config").join("frogbite");
    let _ = fs::create_dir_all(&path);
    path.join("environments.json")
}

/// Loads environments from disk.
pub fn load() -> EnvironmentData {
    let path = env_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

/// Persists environments to disk.
pub fn save(data: &EnvironmentData) {
    if let Ok(json) = serde_json::to_string_pretty(data) {
        let _ = fs::write(env_path(), json);
    }
}
