use reqwest::blocking::Client;
use reqwest::tls::Version;
use reqwest::{Certificate, Identity, Method, Url, redirect};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
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
    #[serde(default)]
    pub set_cookies: Vec<String>,
}

/// Sends a blocking HTTP request and returns the parsed response.
fn error_chain(err: &dyn std::error::Error) -> String {
    let mut parts = vec![err.to_string()];
    let mut src = err.source();
    while let Some(e) = src {
        parts.push(e.to_string());
        src = e.source();
    }
    parts.join(" -> ")
}

#[allow(clippy::too_many_lines)]
pub fn send_request(opts: &RequestOptions) -> Result<HttpResponse, String> {
    let timeout_secs = if opts.timeout_secs == 0 {
        30
    } else {
        opts.timeout_secs
    };
    // We follow redirects manually so we can capture Set-Cookie headers and
    // the URL chain at every hop.
    let mut builder = Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .redirect(redirect::Policy::none());

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

    let mut current_method = opts
        .method
        .parse::<Method>()
        .map_err(|e| format!("Invalid method: {e}"))?;
    let mut current_url = opts.url.clone();
    let mut current_body = opts.body.clone();
    let mut redirect_chain: Vec<String> = Vec::new();
    let mut set_cookies: Vec<String> = Vec::new();
    let mut hops: u32 = 0;

    let (final_status, final_status_text, headers, body) = loop {
        let mut req = client.request(current_method.clone(), &current_url);
        for (k, v) in &opts.headers {
            req = req.header(k.as_str(), v.as_str());
        }
        match &current_body {
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

        let resp = req.send().map_err(|e| classify_error(&e, timeout_secs))?;
        let status = resp.status().as_u16();
        let status_text = resp.status().to_string();

        let mut headers_map = HashMap::new();
        for (k, v) in resp.headers() {
            if let Ok(val) = v.to_str() {
                if k.as_str().eq_ignore_ascii_case("set-cookie") {
                    set_cookies.push(val.to_owned());
                }
                headers_map.insert(k.to_string(), val.to_owned());
            }
        }
        let location = headers_map
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("location"))
            .map(|(_, v)| v.clone());

        let is_redirect = (300..400).contains(&status) && status != 304;
        if !opts.follow_redirects || !is_redirect {
            let body_text = resp
                .text()
                .map_err(|e| format!("Failed to read body: {e}"))?;
            break (status, status_text, headers_map, body_text);
        }

        let Some(loc) = location else {
            let body_text = resp
                .text()
                .map_err(|e| format!("Failed to read body: {e}"))?;
            break (status, status_text, headers_map, body_text);
        };

        // Drop the in-flight body so the next hop can be issued.
        drop(resp);

        hops += 1;
        if hops > 10 {
            return Err("Too many redirects (>10)".to_owned());
        }

        let base = Url::parse(&current_url).map_err(|e| format!("Invalid URL: {e}"))?;
        let next = base
            .join(&loc)
            .map_err(|e| format!("Invalid redirect target: {e}"))?;
        current_url = next.to_string();
        redirect_chain.push(current_url.clone());

        if matches!(status, 301..=303) && current_method != Method::HEAD {
            current_method = Method::GET;
            current_body = None;
        }
    };
    let duration_ms = start.elapsed().as_millis();

    Ok(HttpResponse {
        status: final_status,
        status_text: final_status_text,
        headers,
        body,
        duration_ms,
        redirect_chain,
        set_cookies,
    })
}

fn classify_error(e: &reqwest::Error, timeout_secs: u64) -> String {
    if e.is_timeout() {
        return format!("Request timed out after {timeout_secs}s");
    }
    let chain = error_chain(e);
    if chain.contains("UnknownIssuer")
        || chain.contains("self-signed")
        || chain.contains("self signed")
    {
        return format!(
            "TLS error: untrusted/self-signed certificate. \
             Disable 'Verify TLS' in the TLS popup (S) or add the CA cert. \
             ({chain})"
        );
    }
    if chain.contains("CertificateExpired") || chain.contains("Expired") {
        return format!("TLS error: certificate expired ({chain})");
    }
    if chain.contains("NotValidForName") || chain.contains("hostname") {
        return format!("TLS error: certificate hostname mismatch ({chain})");
    }
    if chain.to_lowercase().contains("certificate") || chain.to_lowercase().contains("tls") {
        return format!("TLS error: {chain}");
    }
    format!("Request failed: {chain}")
}
