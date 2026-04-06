use reqwest::blocking::{Client, Response};
use reqwest::redirect;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    let client = Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .redirect(policy)
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

    let resp: Response = req.send().map_err(|e| format!("Request failed: {e}"))?;
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
