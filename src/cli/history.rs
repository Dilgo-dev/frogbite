use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const MAX_HISTORY: usize = 50;

/// A recorded request with its result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub method: String,
    pub url: String,
    pub body: String,
    pub status: Option<u16>,
    pub duration_ms: Option<u128>,
    pub timestamp: u64,
}

fn history_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let path = PathBuf::from(home).join(".config").join("frogbite");
    let _ = fs::create_dir_all(&path);
    path.join("history.json")
}

/// Loads history from disk.
pub fn load() -> Vec<HistoryEntry> {
    let path = history_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

/// Appends an entry and persists, keeping at most `MAX_HISTORY` items.
pub fn append(entry: HistoryEntry) {
    let mut entries = load();
    entries.push(entry);
    if entries.len() > MAX_HISTORY {
        entries.drain(..entries.len() - MAX_HISTORY);
    }
    if let Ok(json) = serde_json::to_string_pretty(&entries) {
        let _ = fs::write(history_path(), json);
    }
}
