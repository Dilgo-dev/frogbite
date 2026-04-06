use std::process::Command;
use std::time::Duration;

const REPO: &str = "Dilgo-dev/frogbite";
const USER_AGENT: &str = concat!("frogbite/", env!("CARGO_PKG_VERSION"));

/// Fetches the tag of the latest GitHub release (e.g. `v0.2.0`).
pub fn fetch_latest_version() -> Result<String, String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| format!("client build failed: {e}"))?;
    let resp = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("github API returned {}", resp.status()));
    }
    let value: serde_json::Value = resp.json().map_err(|e| format!("json parse failed: {e}"))?;
    value
        .get("tag_name")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "missing tag_name in response".to_owned())
}

/// Returns `true` if `latest` represents a newer version than `current`.
/// Both strings may carry a leading `v`. Falls back to a lexicographic compare
/// when components are not pure numbers.
pub fn is_newer(latest: &str, current: &str) -> bool {
    let l = latest.trim_start_matches('v');
    let c = current.trim_start_matches('v');
    if l == c {
        return false;
    }
    let lp: Vec<u64> = l.split('.').filter_map(|s| s.parse().ok()).collect();
    let cp: Vec<u64> = c.split('.').filter_map(|s| s.parse().ok()).collect();
    if lp.is_empty() || cp.is_empty() {
        return l > c;
    }
    for i in 0..lp.len().max(cp.len()) {
        let a = lp.get(i).copied().unwrap_or(0);
        let b = cp.get(i).copied().unwrap_or(0);
        if a != b {
            return a > b;
        }
    }
    false
}

/// Re-runs the upstream install script targeted at the directory holding the
/// current executable, so the running binary is overwritten in place.
pub fn self_install() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe failed: {e}"))?;
    let dir = exe
        .parent()
        .ok_or_else(|| "current exe has no parent dir".to_owned())?;
    let dir_str = dir
        .to_str()
        .ok_or_else(|| "non-utf8 install dir".to_owned())?;

    #[cfg(unix)]
    let status = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "curl -fsSL https://raw.githubusercontent.com/{REPO}/main/scripts/install.sh | sh"
        ))
        .env("FROGBITE_INSTALL_DIR", dir_str)
        .status()
        .map_err(|e| format!("failed to invoke installer: {e}"))?;

    #[cfg(windows)]
    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &format!(
                "$env:FROGBITE_INSTALL_DIR='{dir_str}'; \
                 irm https://raw.githubusercontent.com/{REPO}/main/scripts/install.ps1 | iex"
            ),
        ])
        .status()
        .map_err(|e| format!("failed to invoke installer: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("installer exited with {status}"))
    }
}
