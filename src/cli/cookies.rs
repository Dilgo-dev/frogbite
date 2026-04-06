use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// A single stored cookie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    #[serde(default)]
    pub secure: bool,
    #[serde(default)]
    pub http_only: bool,
    #[serde(default)]
    pub expires_unix: Option<u64>,
}

/// Persistent cookie store.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CookieStore {
    pub cookies: Vec<Cookie>,
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn cookies_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let path = PathBuf::from(home).join(".config").join("frogbite");
    let _ = fs::create_dir_all(&path);
    path.join("cookies.json")
}

/// Loads the cookie store from disk.
pub fn load() -> CookieStore {
    fs::read_to_string(cookies_path())
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

/// Persists the cookie store to disk.
pub fn save(store: &CookieStore) {
    if let Ok(json) = serde_json::to_string_pretty(store) {
        let _ = fs::write(cookies_path(), json);
    }
}

impl CookieStore {
    pub fn purge_expired(&mut self) {
        let now = now_unix();
        self.cookies
            .retain(|c| c.expires_unix.is_none_or(|e| e > now));
    }

    pub fn clear(&mut self) {
        self.cookies.clear();
    }

    pub fn remove(&mut self, idx: usize) {
        if idx < self.cookies.len() {
            self.cookies.remove(idx);
        }
    }

    /// Builds the `Cookie:` header value for the given URL, or `None` if no
    /// cookies match.
    pub fn header_for(&self, url: &str) -> Option<String> {
        let (host, path, scheme) = parse_url(url)?;
        let now = now_unix();
        let pairs: Vec<String> = self
            .cookies
            .iter()
            .filter(|c| {
                if c.expires_unix.is_some_and(|e| e <= now) {
                    return false;
                }
                if c.secure && scheme != "https" {
                    return false;
                }
                domain_matches(&host, &c.domain) && path_matches(&path, &c.path)
            })
            .map(|c| format!("{}={}", c.name, c.value))
            .collect();
        if pairs.is_empty() {
            None
        } else {
            Some(pairs.join("; "))
        }
    }

    /// Merges `Set-Cookie` headers from a response into the store.
    pub fn ingest(&mut self, request_url: &str, set_cookies: &[String]) {
        let Some((host, _, _)) = parse_url(request_url) else {
            return;
        };
        for raw in set_cookies {
            if let Some(cookie) = parse_set_cookie(raw, &host) {
                self.upsert(cookie);
            }
        }
    }

    fn upsert(&mut self, cookie: Cookie) {
        self.cookies.retain(|c| {
            !(c.name == cookie.name && c.domain == cookie.domain && c.path == cookie.path)
        });
        self.cookies.push(cookie);
    }
}

fn parse_url(url: &str) -> Option<(String, String, String)> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let host = parsed.host_str()?.to_ascii_lowercase();
    let path = parsed.path().to_owned();
    let scheme = parsed.scheme().to_owned();
    Some((host, path, scheme))
}

fn domain_matches(host: &str, cookie_domain: &str) -> bool {
    let cookie_domain = cookie_domain.trim_start_matches('.').to_ascii_lowercase();
    host == cookie_domain || host.ends_with(&format!(".{cookie_domain}"))
}

fn path_matches(req_path: &str, cookie_path: &str) -> bool {
    if cookie_path.is_empty() || cookie_path == "/" {
        return true;
    }
    if req_path == cookie_path {
        return true;
    }
    if req_path.starts_with(cookie_path) {
        let next = req_path.as_bytes().get(cookie_path.len()).copied();
        return next == Some(b'/') || cookie_path.ends_with('/');
    }
    false
}

fn parse_set_cookie(header: &str, request_host: &str) -> Option<Cookie> {
    let mut parts = header.split(';');
    let first = parts.next()?.trim();
    let (name, value) = first.split_once('=')?;
    let mut cookie = Cookie {
        name: name.trim().to_owned(),
        value: value.trim().to_owned(),
        domain: request_host.to_owned(),
        path: "/".to_owned(),
        secure: false,
        http_only: false,
        expires_unix: None,
    };
    if cookie.name.is_empty() {
        return None;
    }
    for attr in parts {
        let attr = attr.trim();
        if attr.eq_ignore_ascii_case("Secure") {
            cookie.secure = true;
        } else if attr.eq_ignore_ascii_case("HttpOnly") {
            cookie.http_only = true;
        } else if let Some((k, v)) = attr.split_once('=') {
            let k = k.trim();
            let v = v.trim();
            if k.eq_ignore_ascii_case("Domain") {
                cookie.domain = v.trim_start_matches('.').to_ascii_lowercase();
            } else if k.eq_ignore_ascii_case("Path") {
                v.clone_into(&mut cookie.path);
            } else if k.eq_ignore_ascii_case("Max-Age") {
                if let Ok(secs) = v.parse::<i64>() {
                    if secs > 0 {
                        cookie.expires_unix = Some(now_unix() + secs as u64);
                    } else {
                        cookie.expires_unix = Some(0);
                    }
                }
            }
        }
    }
    Some(cookie)
}
