use reqwest::blocking::{Client, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestOptions {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub duration_ms: u128,
}

pub fn send_request(opts: &RequestOptions) -> Result<HttpResponse, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
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

    if let Some(body) = &opts.body {
        req = req.body(body.clone());
    }

    let resp: Response = req.send().map_err(|e| format!("Request failed: {e}"))?;
    let duration_ms = start.elapsed().as_millis();

    let status = resp.status().as_u16();
    let status_text = resp.status().to_string();

    let mut headers = HashMap::new();
    for (k, v) in resp.headers() {
        if let Ok(val) = v.to_str() {
            headers.insert(k.to_string(), val.to_string());
        }
    }

    let body = resp.text().map_err(|e| format!("Failed to read body: {e}"))?;

    Ok(HttpResponse {
        status,
        status_text,
        headers,
        body,
        duration_ms,
    })
}
