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

/// Parses a .env file into a list of variables.
pub fn parse_dotenv(path: &std::path::Path) -> Option<Vec<Variable>> {
    let content = fs::read_to_string(path).ok()?;
    let mut vars = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, raw_value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key
            .trim()
            .strip_prefix("export ")
            .unwrap_or_else(|| key.trim());
        if key.is_empty() {
            continue;
        }
        let value = unquote(raw_value.trim());
        vars.push(Variable {
            key: key.to_owned(),
            value,
        });
    }

    Some(vars)
}

fn unquote(s: &str) -> String {
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_owned()
    } else {
        s.to_owned()
    }
}
