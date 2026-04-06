use std::collections::HashMap;
use std::process::ExitCode;

use clap::{Args, Subcommand, ValueEnum};
use frogbite::core::http::{HttpResponse, RequestBody, RequestOptions, send_request};
use serde_json::json;

use crate::assertions;
use crate::collections::{self, Auth, BodyType, ContentType, SavedRequest};
use crate::curl::base64;
use crate::docs;
use crate::update;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Send a single HTTP request and print the response to stdout
    Send(SendArgs),
    /// Run the saved collection as a test suite
    Run(RunArgs),
    /// Generate a styled HTML reference from the saved collection
    Docs(DocsArgs),
    /// Check for and install the latest frogbite release
    Update,
}

#[derive(Debug, Args)]
pub struct DocsArgs {
    /// Output HTML file path
    #[arg(
        short = 'o',
        long = "out",
        value_name = "PATH",
        default_value = "frogbite-api.html"
    )]
    pub out: std::path::PathBuf,
    /// Override the document title
    #[arg(long, value_name = "TITLE")]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum OutputFormat {
    /// Human-readable, colored-ish output (default)
    #[default]
    Pretty,
    /// Machine-readable JSON
    Json,
    /// One-line minimal output
    Minimal,
}

#[derive(Debug, Args)]
pub struct RunArgs {
    /// Only run requests inside the folder with this name
    #[arg(long)]
    pub folder: Option<String>,
    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Pretty)]
    pub format: OutputFormat,
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
    /// Include response headers in the output (pretty format only)
    #[arg(short = 'i', long)]
    pub include: bool,
    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Pretty)]
    pub format: OutputFormat,
    /// Proxy URL (http://, https://, socks5://)
    #[arg(short = 'x', long, value_name = "URL")]
    pub proxy: Option<String>,
}

pub fn execute(cmd: &Command) -> ExitCode {
    let result = match cmd {
        Command::Send(args) => run_send(args),
        Command::Run(args) => run_collection(args),
        Command::Docs(args) => run_docs(args),
        Command::Update => run_update(),
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
        proxy_url: args.proxy.clone().unwrap_or_default(),
    };

    let resp = send_request(&opts)?;

    match args.format {
        OutputFormat::Pretty => {
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
        }
        OutputFormat::Json => {
            let payload = json!({
                "status": resp.status,
                "status_text": resp.status_text,
                "headers": resp.headers,
                "body": resp.body,
                "duration_ms": resp.duration_ms,
                "redirect_chain": resp.redirect_chain,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&payload)
                    .map_err(|e| format!("json encode failed: {e}"))?
            );
        }
        OutputFormat::Minimal => {
            println!("{}", resp.status);
        }
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
    let mut outcomes: Vec<RequestOutcome> = Vec::with_capacity(total);
    for req in &requests {
        match send_saved(req) {
            Ok(resp) => {
                let results = assertions::evaluate_all(&req.assertions, &resp);
                outcomes.push(RequestOutcome {
                    name: req.name.clone(),
                    method: req.method.clone(),
                    url: req.url.clone(),
                    response: Some(resp),
                    assertions: results,
                    error: None,
                });
            }
            Err(e) => outcomes.push(RequestOutcome {
                name: req.name.clone(),
                method: req.method.clone(),
                url: req.url.clone(),
                response: None,
                assertions: Vec::new(),
                error: Some(e),
            }),
        }
    }

    let failed_requests = outcomes.iter().filter(|o| o.failed()).count();
    let passed_requests = total - failed_requests;
    let total_assertions: usize = outcomes.iter().map(|o| o.assertions.len()).sum();
    let failed_assertions: usize = outcomes
        .iter()
        .flat_map(|o| &o.assertions)
        .filter(|a| !a.passed)
        .count();

    match args.format {
        OutputFormat::Pretty => render_run_pretty(
            &outcomes,
            total,
            passed_requests,
            failed_requests,
            total_assertions,
            failed_assertions,
        ),
        OutputFormat::Json => render_run_json(
            &outcomes,
            total,
            passed_requests,
            failed_requests,
            total_assertions,
            failed_assertions,
        )?,
        OutputFormat::Minimal => render_run_minimal(&outcomes),
    }

    if failed_requests > 0 {
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

struct RequestOutcome {
    name: String,
    method: String,
    url: String,
    response: Option<HttpResponse>,
    assertions: Vec<assertions::AssertionResult>,
    error: Option<String>,
}

impl RequestOutcome {
    fn failed(&self) -> bool {
        self.error.is_some() || self.assertions.iter().any(|a| !a.passed)
    }
}

fn render_run_pretty(
    outcomes: &[RequestOutcome],
    total: usize,
    passed_requests: usize,
    failed_requests: usize,
    total_assertions: usize,
    failed_assertions: usize,
) {
    println!("Running {total} request(s)\n");
    for (idx, o) in outcomes.iter().enumerate() {
        let label = format!("[{}/{}] {} {}", idx + 1, total, o.method, o.name);
        if let Some(err) = &o.error {
            println!("FAIL  {label}  -> request error: {err}");
            continue;
        }
        let resp = o.response.as_ref().unwrap_or_else(|| unreachable!());
        let marker = if o.failed() { "FAIL" } else { "PASS" };
        println!(
            "{marker}  {label}  -> {} {} ({}ms)",
            resp.status, resp.status_text, resp.duration_ms
        );
        for r in &o.assertions {
            if r.passed {
                println!("      ok    {}", r.message);
            } else {
                println!("      FAIL  {}", r.message);
            }
        }
    }
    println!(
        "\nSummary: {passed_requests} passed, {failed_requests} failed ({total} total) | \
         assertions: {} passed, {failed_assertions} failed ({total_assertions} total)",
        total_assertions - failed_assertions
    );
}

fn render_run_json(
    outcomes: &[RequestOutcome],
    total: usize,
    passed_requests: usize,
    failed_requests: usize,
    total_assertions: usize,
    failed_assertions: usize,
) -> Result<(), String> {
    let results: Vec<_> = outcomes
        .iter()
        .map(|o| {
            json!({
                "name": o.name,
                "method": o.method,
                "url": o.url,
                "passed": !o.failed(),
                "error": o.error,
                "status": o.response.as_ref().map(|r| r.status),
                "status_text": o.response.as_ref().map(|r| r.status_text.clone()),
                "duration_ms": o.response.as_ref().map(|r| r.duration_ms),
                "assertions": o.assertions.iter().map(|a| json!({
                    "passed": a.passed,
                    "message": a.message,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let payload = json!({
        "total": total,
        "passed_requests": passed_requests,
        "failed_requests": failed_requests,
        "total_assertions": total_assertions,
        "passed_assertions": total_assertions - failed_assertions,
        "failed_assertions": failed_assertions,
        "results": results,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).map_err(|e| format!("json encode failed: {e}"))?
    );
    Ok(())
}

fn render_run_minimal(outcomes: &[RequestOutcome]) {
    for o in outcomes {
        let marker = if o.failed() { "FAIL" } else { "PASS" };
        let status = o
            .response
            .as_ref()
            .map_or_else(|| "ERR".to_owned(), |r| r.status.to_string());
        println!("{marker} {status} {} {}", o.method, o.name);
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
        proxy_url: req.proxy_url.clone(),
    };
    send_request(&opts)
}

fn run_docs(args: &DocsArgs) -> Result<ExitCode, String> {
    let data = collections::load();
    let html = docs::render_html(&data, args.title.as_deref());
    std::fs::write(&args.out, html).map_err(|e| format!("write {}: {e}", args.out.display()))?;
    println!("wrote {}", args.out.display());
    Ok(ExitCode::SUCCESS)
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

fn run_update() -> Result<ExitCode, String> {
    let current = env!("CARGO_PKG_VERSION");
    println!("current version: {current}");
    let latest = update::fetch_latest_version()?;
    println!("latest release : {latest}");
    if !update::is_newer(&latest, current) {
        println!("already up to date");
        return Ok(ExitCode::SUCCESS);
    }
    println!("installing {latest}...");
    update::self_install()?;
    println!("update complete");
    Ok(ExitCode::SUCCESS)
}

const fn content_type_mime(ct: ContentType) -> &'static str {
    match ct {
        ContentType::Json => "application/json",
        ContentType::Text => "text/plain",
        ContentType::Xml => "application/xml",
        ContentType::Html => "text/html",
    }
}
