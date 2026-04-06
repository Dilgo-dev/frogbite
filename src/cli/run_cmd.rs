use std::collections::HashMap;
use std::process::ExitCode;

use clap::{Args, Subcommand};
use frogbite::core::http::{RequestBody, RequestOptions, send_request};

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Send a single HTTP request and print the response to stdout
    Send(SendArgs),
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
    match cmd {
        Command::Send(args) => match run_send(args) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::from(1)
            }
        },
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
