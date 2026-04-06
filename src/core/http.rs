use reqwest::blocking::{Client, Response};
use reqwest::tls::Version;
use reqwest::{Certificate, Identity, redirect};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Body payload for an outgoing HTTP request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequestBody {
    Raw(String),
    Form(Vec<(String, String)>),
    Multipart(Vec<(String, String)>),
}

/// Configuration for an outgoing HTTP request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestOptions {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<RequestBody>,
    #[serde(default)]
    pub follow_redirects: bool,
    #[serde(default)]
    pub timeout_secs: u64,
    #[serde(default)]
    pub verify_tls: bool,
    #[serde(default)]
    pub ca_cert_path: String,
    #[serde(default)]
    pub client_cert_path: String,
    #[serde(default)]
    pub client_key_path: String,
    #[serde(default)]
    pub tls_min_version: String,
}

/// Parsed HTTP response with status, headers, body and timing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub duration_ms: u128,
    #[serde(default)]
    pub redirect_chain: Vec<String>,
}

/// Sends a blocking HTTP request and returns the parsed response.
#[allow(clippy::too_many_lines)]
pub fn send_request(opts: &RequestOptions) -> Result<HttpResponse, String> {
    let chain: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    let policy = if opts.follow_redirects {
        let chain_c = Arc::clone(&chain);
        redirect::Policy::custom(move |attempt| {
            if attempt.previous().len() >= 10 {
                return attempt.error("too many redirects");
            }
            if let Ok(mut c) = chain_c.lock() {
                c.push(attempt.url().to_string());
            }
            attempt.follow()
        })
    } else {
        redirect::Policy::none()
    };

    let timeout_secs = if opts.timeout_secs == 0 {
        30
    } else {
        opts.timeout_secs
    };
    let mut builder = Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .redirect(policy);

    if !opts.verify_tls {
        builder = builder.danger_accept_invalid_certs(true);
    }

    if !opts.ca_cert_path.is_empty() {
        let pem = fs::read(&opts.ca_cert_path)
            .map_err(|e| format!("CA cert read failed ({}): {e}", opts.ca_cert_path))?;
        let cert = Certificate::from_pem(&pem).map_err(|e| format!("CA cert parse failed: {e}"))?;
        builder = builder.add_root_certificate(cert);
    }

    if !opts.client_cert_path.is_empty() {
        let mut combined = fs::read(&opts.client_cert_path)
            .map_err(|e| format!("Client cert read failed ({}): {e}", opts.client_cert_path))?;
        if !opts.client_key_path.is_empty() {
            let mut key = fs::read(&opts.client_key_path)
                .map_err(|e| format!("Client key read failed ({}): {e}", opts.client_key_path))?;
            combined.push(b'\n');
            combined.append(&mut key);
        }
        let id = Identity::from_pem(&combined)
            .map_err(|e| format!("Client identity parse failed: {e}"))?;
        builder = builder.identity(id);
    }

    match opts.tls_min_version.as_str() {
        "1.2" => builder = builder.min_tls_version(Version::TLS_1_2),
        "1.3" => builder = builder.min_tls_version(Version::TLS_1_3),
        _ => {}
    }

    let client = builder
        .build()
        .map_err(|e| format!("Failed to create client: {e}"))?;

    let start = Instant::now();

    let method = opts
        .method
        .parse::<reqwest::Method>()
        .map_err(|e| format!("Invalid method: {e}"))?;

    let mut req = client.request(method, &opts.url);

    for (k, v) in &opts.headers {
        req = req.header(k.as_str(), v.as_str());
    }

    match &opts.body {
        Some(RequestBody::Raw(text)) => {
            req = req.body(text.clone());
        }
        Some(RequestBody::Form(pairs)) => {
            req = req.form(pairs);
        }
        Some(RequestBody::Multipart(pairs)) => {
            let mut form = reqwest::blocking::multipart::Form::new();
            for (k, v) in pairs {
                form = form.text(k.clone(), v.clone());
            }
            req = req.multipart(form);
        }
        None => {}
    }

    let resp: Response = req.send().map_err(|e| {
        if e.is_timeout() {
            format!("Request timed out after {timeout_secs}s")
        } else {
            format!("Request failed: {e}")
        }
    })?;
    let duration_ms = start.elapsed().as_millis();

    let status = resp.status().as_u16();
    let status_text = resp.status().to_string();

    let mut headers = HashMap::new();
    for (k, v) in resp.headers() {
        if let Ok(val) = v.to_str() {
            headers.insert(k.to_string(), val.to_owned());
        }
    }

    let body = resp
        .text()
        .map_err(|e| format!("Failed to read body: {e}"))?;

    let redirect_chain = chain.lock().map(|c| c.clone()).unwrap_or_default();

    Ok(HttpResponse {
        status,
        status_text,
        headers,
        body,
        duration_ms,
        redirect_chain,
    })
}
