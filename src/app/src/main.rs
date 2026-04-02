#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Deserialize)]
struct RequestOptions {
    method: String,
    url: String,
    headers: HashMap<String, String>,
    body: Option<String>,
}

#[derive(Debug, Serialize)]
struct HttpResponse {
    status: u16,
    status_text: String,
    headers: HashMap<String, String>,
    body: String,
    duration_ms: u128,
    error: Option<String>,
}

#[tauri::command]
async fn send_request(opts: RequestOptions) -> HttpResponse {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build();

    let client = match client {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse {
                status: 0,
                status_text: String::new(),
                headers: HashMap::new(),
                body: String::new(),
                duration_ms: 0,
                error: Some(format!("Failed to create client: {e}")),
            }
        }
    };

    let start = Instant::now();

    let method = match opts.method.parse::<reqwest::Method>() {
        Ok(m) => m,
        Err(e) => {
            return HttpResponse {
                status: 0,
                status_text: String::new(),
                headers: HashMap::new(),
                body: String::new(),
                duration_ms: 0,
                error: Some(format!("Invalid method: {e}")),
            }
        }
    };

    let mut req = client.request(method, &opts.url);

    for (k, v) in &opts.headers {
        req = req.header(k.as_str(), v.as_str());
    }

    if let Some(body) = &opts.body {
        req = req.body(body.clone());
    }

    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse {
                status: 0,
                status_text: String::new(),
                headers: HashMap::new(),
                body: String::new(),
                duration_ms: start.elapsed().as_millis(),
                error: Some(format!("Request failed: {e}")),
            }
        }
    };

    let duration_ms = start.elapsed().as_millis();
    let status = resp.status().as_u16();
    let status_text = resp.status().to_string();

    let mut headers = HashMap::new();
    for (k, v) in resp.headers() {
        if let Ok(val) = v.to_str() {
            headers.insert(k.to_string(), val.to_string());
        }
    }

    let body = resp.text().await.unwrap_or_default();

    HttpResponse {
        status,
        status_text,
        headers,
        body,
        duration_ms,
        error: None,
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![send_request])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
