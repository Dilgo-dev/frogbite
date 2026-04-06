use std::collections::HashMap;
use std::process::ExitCode;

use clap::{Args, Subcommand};
use frogbite::core::http::{HttpResponse, RequestBody, RequestOptions, send_request};

use crate::assertions;
use crate::collections::{self, Auth, BodyType, ContentType, SavedRequest};
use crate::curl::base64;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Send a single HTTP request and print the response to stdout
    Send(SendArgs),
    /// Run the saved collection as a test suite
    Run(RunArgs),
}

#[derive(Debug, Args)]
pub struct RunArgs {
    /// Only run requests inside the folder with this name
    #[arg(long)]
    pub folder: Option<String>,
}

#[derive(Debug, Args)]
pub struct SendArgs {
    /// HTTP method (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)
    pub method: String,
    /// Request URL
    pub url: String,
    /// Header in `Key: Value` form, repeatable
    #[arg(short = 'H', long = "header", value_name = "HEADER")]
    pub headers: Vec<String>,
    /// Raw request body
    #[arg(short = 'd', long = "data", value_name = "BODY")]
    pub data: Option<String>,
    /// Timeout in seconds (default 30)
    #[arg(long, default_value_t = 0)]
    pub timeout: u64,
    /// Follow redirects
    #[arg(short = 'L', long)]
    pub follow: bool,
    /// Skip TLS certificate verification
    #[arg(short = 'k', long)]
    pub insecure: bool,
    /// Include response headers in the output
    #[arg(short = 'i', long)]
    pub include: bool,
}

pub fn execute(cmd: &Command) -> ExitCode {
    let result = match cmd {
        Command::Send(args) => run_send(args),
        Command::Run(args) => run_collection(args),
    };
    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn run_send(args: &SendArgs) -> Result<ExitCode, String> {
    let mut headers = HashMap::new();
    for h in &args.headers {
        let (k, v) = h
            .split_once(':')
            .ok_or_else(|| format!("invalid header (expected 'Key: Value'): {h}"))?;
        headers.insert(k.trim().to_owned(), v.trim().to_owned());
    }

    let opts = RequestOptions {
        method: args.method.to_uppercase(),
        url: args.url.clone(),
        headers,
        body: args.data.clone().map(RequestBody::Raw),
        follow_redirects: args.follow,
        timeout_secs: args.timeout,
        verify_tls: !args.insecure,
        ca_cert_path: String::new(),
        client_cert_path: String::new(),
        client_key_path: String::new(),
        tls_min_version: String::new(),
    };

    let resp = send_request(&opts)?;

    if args.include {
        println!("HTTP {} {}", resp.status, resp.status_text);
        for (k, v) in &resp.headers {
            println!("{k}: {v}");
        }
        println!();
    }
    print!("{}", resp.body);
    if !resp.body.ends_with('\n') {
        println!();
    }

    Ok(ExitCode::SUCCESS)
}

fn run_collection(args: &RunArgs) -> Result<ExitCode, String> {
    let data = collections::load();
    if data.requests.is_empty() {
        return Err("no saved requests in collection".to_owned());
    }

    let folder_id = if let Some(name) = &args.folder {
        let id = data
            .folders
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(name))
            .map(|f| f.id.clone())
            .ok_or_else(|| format!("folder not found: {name}"))?;
        Some(id)
    } else {
        None
    };

    let requests: Vec<&SavedRequest> = data
        .requests
        .iter()
        .filter(|r| {
            folder_id
                .as_ref()
                .is_none_or(|id| r.folder_id.as_ref() == Some(id))
        })
        .collect();

    if requests.is_empty() {
        return Err("no requests match the given filter".to_owned());
    }

    let total = requests.len();
    let mut passed_requests = 0usize;
    let mut failed_requests = 0usize;
    let mut total_assertions = 0usize;
    let mut failed_assertions = 0usize;

    println!("Running {total} request(s)\n");

    for (idx, req) in requests.iter().enumerate() {
        let label = format!("[{}/{}] {} {}", idx + 1, total, req.method, req.name);
        match send_saved(req) {
            Ok(resp) => {
                let results = assertions::evaluate_all(&req.assertions, &resp);
                let req_failed = results.iter().any(|r| !r.passed);
                let status_marker = if req_failed { "FAIL" } else { "PASS" };
                println!(
                    "{status_marker}  {label}  -> {} {} ({}ms)",
                    resp.status, resp.status_text, resp.duration_ms
                );
                for r in &results {
                    total_assertions += 1;
                    if r.passed {
                        println!("      ok    {}", r.message);
                    } else {
                        failed_assertions += 1;
                        println!("      FAIL  {}", r.message);
                    }
                }
                if req_failed {
                    failed_requests += 1;
                } else {
                    passed_requests += 1;
                }
            }
            Err(e) => {
                failed_requests += 1;
                println!("FAIL  {label}  -> request error: {e}");
            }
        }
    }

    println!(
        "\nSummary: {passed_requests} passed, {failed_requests} failed ({total} total) | \
         assertions: {} passed, {failed_assertions} failed ({total_assertions} total)",
        total_assertions - failed_assertions
    );

    if failed_requests > 0 {
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn send_saved(req: &SavedRequest) -> Result<HttpResponse, String> {
    let mut headers = req.headers.clone();
    apply_auth(&req.auth, &mut headers);

    let body = match req.body_type {
        BodyType::Raw => {
            if !headers
                .keys()
                .any(|k| k.eq_ignore_ascii_case("content-type"))
            {
                headers.insert(
                    "Content-Type".to_owned(),
                    content_type_mime(req.content_type).to_owned(),
                );
            }
            if req.body.is_empty() {
                None
            } else {
                Some(RequestBody::Raw(req.body.clone()))
            }
        }
        BodyType::Form => {
            if req.form_data.is_empty() {
                None
            } else {
                Some(RequestBody::Form(req.form_data.clone()))
            }
        }
        BodyType::Multipart => {
            if req.form_data.is_empty() {
                None
            } else {
                Some(RequestBody::Multipart(req.form_data.clone()))
            }
        }
    };

    let opts = RequestOptions {
        method: req.method.clone(),
        url: req.url.clone(),
        headers,
        body,
        follow_redirects: req.follow_redirects,
        timeout_secs: req.timeout_secs,
        verify_tls: req.verify_tls,
        ca_cert_path: req.ca_cert_path.clone(),
        client_cert_path: req.client_cert_path.clone(),
        client_key_path: req.client_key_path.clone(),
        tls_min_version: req.tls_min_version.clone(),
    };
    send_request(&opts)
}

fn apply_auth(auth: &Auth, headers: &mut HashMap<String, String>) {
    match auth {
        Auth::None => {}
        Auth::Bearer { token } => {
            headers.insert("Authorization".to_owned(), format!("Bearer {token}"));
        }
        Auth::Basic { username, password } => {
            let encoded = base64(format!("{username}:{password}").as_bytes());
            headers.insert("Authorization".to_owned(), format!("Basic {encoded}"));
        }
        Auth::ApiKey { header, value } => {
            headers.insert(header.clone(), value.clone());
        }
    }
}

const fn content_type_mime(ct: ContentType) -> &'static str {
    match ct {
        ContentType::Json => "application/json",
        ContentType::Text => "text/plain",
        ContentType::Xml => "application/xml",
        ContentType::Html => "text/html",
    }
}
