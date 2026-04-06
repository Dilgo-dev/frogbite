//! Subprocess plugin runner.
//!
//! A plugin is just an executable on disk (any language). Frogbite invokes
//! it, writes a JSON envelope on stdin, then reads a JSON envelope on
//! stdout. Two hook points are supported: `request` (called before sending,
//! plugin can rewrite the outgoing request) and `response` (called after
//! receiving, plugin can rewrite the response shown to the user).
//!
//! Envelope schema:
//!
//! ```text
//! // stdin
//! {"hook":"request","request":{"method":"GET","url":"...","headers":{...},"body":"..."}}
//! {"hook":"response","response":{"status":200,"status_text":"OK","headers":{...},"body":"..."}}
//!
//! // stdout
//! {"request":{...}}      or {"response":{...}}      or {"error":"..."}
//! ```

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

const PLUGIN_TIMEOUT_SECS: u64 = 10;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginRequest {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginResponse {
    pub status: u16,
    #[serde(default)]
    pub status_text: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub body: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "hook", rename_all = "lowercase")]
enum Envelope<'a> {
    Request { request: &'a PluginRequest },
    Response { response: &'a PluginResponse },
}

#[derive(Debug, Deserialize)]
struct PluginReply {
    #[serde(default)]
    request: Option<PluginRequest>,
    #[serde(default)]
    response: Option<PluginResponse>,
    #[serde(default)]
    error: Option<String>,
}

/// Returns the directory frogbite expects plugin executables to live in.
pub fn plugins_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home)
        .join(".config")
        .join("frogbite")
        .join("plugins")
}

/// Resolves a plugin name to an executable path.
fn resolve(name: &str) -> Result<PathBuf, String> {
    let dir = plugins_dir();
    let direct = dir.join(name);
    if direct.is_file() {
        return Ok(direct);
    }
    Err(format!("plugin '{name}' not found in {}", dir.display()))
}

/// Runs a plugin against a `PluginRequest` envelope and returns the
/// transformed request.
pub fn run_request_hook(name: &str, req: &PluginRequest) -> Result<PluginRequest, String> {
    let envelope = Envelope::Request { request: req };
    let raw = serde_json::to_vec(&envelope).map_err(|e| format!("serialize: {e}"))?;
    let reply = invoke(name, &raw)?;
    if let Some(err) = reply.error {
        return Err(format!("plugin '{name}': {err}"));
    }
    reply
        .request
        .ok_or_else(|| format!("plugin '{name}' returned no 'request' field"))
}

/// Runs a plugin against a `PluginResponse` envelope and returns the
/// transformed response.
pub fn run_response_hook(name: &str, resp: &PluginResponse) -> Result<PluginResponse, String> {
    let envelope = Envelope::Response { response: resp };
    let raw = serde_json::to_vec(&envelope).map_err(|e| format!("serialize: {e}"))?;
    let reply = invoke(name, &raw)?;
    if let Some(err) = reply.error {
        return Err(format!("plugin '{name}': {err}"));
    }
    reply
        .response
        .ok_or_else(|| format!("plugin '{name}' returned no 'response' field"))
}

fn invoke(name: &str, stdin_bytes: &[u8]) -> Result<PluginReply, String> {
    let exe = resolve(name)?;

    let mut child = Command::new(&exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("plugin '{name}' spawn failed: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(stdin_bytes)
            .map_err(|e| format!("plugin '{name}' stdin write: {e}"))?;
    }

    let timeout = Duration::from_secs(PLUGIN_TIMEOUT_SECS);
    let started = Instant::now();
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| format!("plugin '{name}' wait: {e}"))?
        {
            let mut stdout = String::new();
            if let Some(mut out) = child.stdout.take() {
                out.read_to_string(&mut stdout).ok();
            }
            let mut stderr = String::new();
            if let Some(mut err) = child.stderr.take() {
                err.read_to_string(&mut stderr).ok();
            }
            if !status.success() {
                let trimmed = stderr.trim();
                let detail = if trimmed.is_empty() {
                    format!("exit {status}")
                } else {
                    format!("exit {status}: {trimmed}")
                };
                return Err(format!("plugin '{name}' failed: {detail}"));
            }
            return serde_json::from_str(&stdout)
                .map_err(|e| format!("plugin '{name}' bad json: {e}"));
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            return Err(format!(
                "plugin '{name}' timed out after {PLUGIN_TIMEOUT_SECS}s"
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
