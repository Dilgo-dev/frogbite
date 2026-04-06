//! Generates a single self-contained HTML reference from a frogbite
//! collection. The visual style mirrors the frogbite-cloud landing page:
//! near-black canvas, lime accent, `Cabinet Grotesk` + `Satoshi` +
//! `JetBrains Mono`, grid background and noise overlay.

#![allow(
    clippy::too_many_lines,
    clippy::cast_possible_wrap,
    clippy::write_with_newline
)]

use std::collections::BTreeSet;
use std::fmt::Write as _;

use crate::collections::{Auth, BodyType, CollectionData, SavedRequest};

const ASCII: &str = r"___________                   __________.__  __
\_   _____/______  ____   ____\______   \__|/  |_  ____
 |    __) \_  __ \/  _ \ / ___\|    |  _/  \   __\/ __ \
 |     \   |  | \(  <_> ) /_/  >    |   \  ||  | \  ___/
 \___  /   |__|   \____/\___  /|______  /__||__|  \___  >
     \/                /_____/        \/              \/ ";

pub fn render_html(data: &CollectionData, title_override: Option<&str>) -> String {
    let title = title_override.unwrap_or("Frogbite API Reference");
    let request_count = data.requests.len();
    let folder_count = data.folders.len();
    let methods: BTreeSet<&str> = data.requests.iter().map(|r| r.method.as_str()).collect();
    let method_count = methods.len();
    let generated_at = current_date();

    let mut toc = String::new();
    let mut sections = String::new();

    let mut idx = 0usize;
    let mut emit_section = |label: &str, requests: &[&SavedRequest], idx: &mut usize| {
        let folder_anchor = slug(label);
        sections.push_str("<section class=\"folder\" id=\"folder-");
        sections.push_str(&folder_anchor);
        sections.push_str("\">\n  <div class=\"folder-header\">\n    <span class=\"folder-tag\">// FOLDER</span>\n    <h2 class=\"folder-name\">");
        sections.push_str(&esc(label));
        sections.push_str(
            "</h2>\n    <span class=\"folder-line\"></span>\n    <span class=\"folder-tag\">",
        );
        let _ = write!(sections, "{} REQ", requests.len());
        sections.push_str("</span>\n  </div>\n");

        for req in requests {
            *idx += 1;
            let anchor = format!("req-{}-{}", folder_anchor, slug(&req.name));
            let num = format!("[{:02}]", *idx);
            let m = esc(&req.method);
            let name = esc(&req.name);
            let url = esc(&req.url);

            // TOC entry
            toc.push_str("<li class=\"toc-item\">\n  <span class=\"toc-num\">");
            toc.push_str(&num);
            toc.push_str("</span>\n  <span class=\"toc-method m-");
            toc.push_str(&m);
            toc.push_str("\">");
            toc.push_str(&m);
            toc.push_str("</span>\n  <a class=\"toc-link\" href=\"#");
            toc.push_str(&anchor);
            toc.push_str("\">");
            toc.push_str(&name);
            toc.push_str("</a>\n</li>\n");

            // Section card
            sections.push_str("  <article class=\"request\" id=\"");
            sections.push_str(&anchor);
            sections
                .push_str("\">\n    <div class=\"request-head\">\n      <span class=\"method m-");
            sections.push_str(&m);
            sections.push_str("\">");
            sections.push_str(&m);
            sections.push_str("</span>\n      <h3 class=\"request-name\">");
            sections.push_str(&name);
            sections.push_str("</h3>\n      <code class=\"request-url\">");
            sections.push_str(&url);
            sections.push_str("</code>\n");

            for badge in request_badges(req) {
                sections.push_str("      <span class=\"badge\">");
                sections.push_str(&esc(&badge));
                sections.push_str("</span>\n");
            }

            sections.push_str("    </div>\n    <div class=\"request-body\">\n");

            // Auth
            if !matches!(req.auth, Auth::None) {
                sections.push_str(
                    "      <div class=\"section\">\n        <div class=\"section-label\">Auth</div>\n",
                );
                let (kind, fields) = describe_auth(&req.auth);
                sections.push_str("        <div class=\"kv\">\n");
                write!(
                    sections,
                    "          <div class=\"kv-key\">type</div><div class=\"kv-val\">{}</div>\n",
                    esc(kind)
                )
                .ok();
                for (k, v) in fields {
                    write!(
                        sections,
                        "          <div class=\"kv-key\">{}</div><div class=\"kv-val\">{}</div>\n",
                        esc(&k),
                        esc(&v)
                    )
                    .ok();
                }
                sections.push_str("        </div>\n      </div>\n");
            }

            // Headers
            if !req.headers.is_empty() {
                let mut entries: Vec<(&String, &String)> = req.headers.iter().collect();
                entries.sort_by(|a, b| a.0.cmp(b.0));
                sections.push_str(
                    "      <div class=\"section\">\n        <div class=\"section-label\">Headers</div>\n        <div class=\"kv\">\n",
                );
                for (k, v) in entries {
                    write!(
                        sections,
                        "          <div class=\"kv-key\">{}</div><div class=\"kv-val\">{}</div>\n",
                        esc(k),
                        esc(v)
                    )
                    .ok();
                }
                sections.push_str("        </div>\n      </div>\n");
            }

            // Body
            let body_label = match req.body_type {
                BodyType::Raw => format!("Body · raw / {}", req.content_type.label()),
                BodyType::Form => "Body · form-urlencoded".to_owned(),
                BodyType::Multipart => "Body · multipart".to_owned(),
            };
            match req.body_type {
                BodyType::Raw if !req.body.is_empty() => {
                    sections.push_str("      <div class=\"section\">\n");
                    write!(
                        sections,
                        "        <div class=\"section-label\">{}</div>\n",
                        esc(&body_label)
                    )
                    .ok();
                    write!(
                        sections,
                        "        <pre class=\"code\">{}</pre>\n      </div>\n",
                        esc(&req.body)
                    )
                    .ok();
                }
                BodyType::Form | BodyType::Multipart if !req.form_data.is_empty() => {
                    sections.push_str("      <div class=\"section\">\n");
                    write!(
                        sections,
                        "        <div class=\"section-label\">{}</div>\n        <div class=\"kv\">\n",
                        esc(&body_label)
                    )
                    .ok();
                    for (k, v) in &req.form_data {
                        write!(
                            sections,
                            "          <div class=\"kv-key\">{}</div><div class=\"kv-val\">{}</div>\n",
                            esc(k),
                            esc(v)
                        )
                        .ok();
                    }
                    sections.push_str("        </div>\n      </div>\n");
                }
                _ => {}
            }

            // gRPC details
            if req.method == "GRPC" && (!req.proto_path.is_empty() || !req.grpc_method.is_empty()) {
                sections.push_str(
                    "      <div class=\"section\">\n        <div class=\"section-label\">gRPC</div>\n        <div class=\"kv\">\n",
                );
                if !req.proto_path.is_empty() {
                    write!(
                        sections,
                        "          <div class=\"kv-key\">proto</div><div class=\"kv-val\">{}</div>\n",
                        esc(&req.proto_path)
                    )
                    .ok();
                }
                if !req.grpc_method.is_empty() {
                    write!(
                        sections,
                        "          <div class=\"kv-key\">method</div><div class=\"kv-val\">{}</div>\n",
                        esc(&req.grpc_method)
                    )
                    .ok();
                }
                sections.push_str("        </div>\n      </div>\n");
            }

            // GraphQL details
            if req.method == "GQL" && !req.gql_variables.is_empty() {
                sections.push_str(
                    "      <div class=\"section\">\n        <div class=\"section-label\">Variables</div>\n",
                );
                write!(
                    sections,
                    "        <pre class=\"code\">{}</pre>\n      </div>\n",
                    esc(&req.gql_variables)
                )
                .ok();
            }

            // Last response sample
            if let Some(resp) = &req.last_response {
                if !resp.body.is_empty() {
                    sections.push_str(
                        "      <div class=\"section\">\n        <div class=\"section-label\">Sample response</div>\n",
                    );
                    let pretty = serde_json::from_str::<serde_json::Value>(&resp.body)
                        .ok()
                        .and_then(|v| serde_json::to_string_pretty(&v).ok())
                        .unwrap_or_else(|| resp.body.clone());
                    let truncated = if pretty.len() > 4000 {
                        format!(
                            "{}\n\n... [truncated {} bytes]",
                            &pretty[..4000],
                            pretty.len() - 4000
                        )
                    } else {
                        pretty
                    };
                    write!(
                        sections,
                        "        <div class=\"section-label\" style=\"margin-top:8px\">{} {} · {} ms</div>\n",
                        resp.status,
                        esc(&resp.status_text),
                        resp.duration_ms,
                    )
                    .ok();
                    write!(
                        sections,
                        "        <pre class=\"code\">{}</pre>\n      </div>\n",
                        esc(&truncated)
                    )
                    .ok();
                }
            }

            sections.push_str("    </div>\n  </article>\n");
        }

        sections.push_str("</section>\n");
    };

    // Group: each folder, then ungrouped
    for folder in &data.folders {
        let in_folder: Vec<&SavedRequest> = data
            .requests
            .iter()
            .filter(|r| r.folder_id.as_deref() == Some(folder.id.as_str()))
            .collect();
        if in_folder.is_empty() {
            continue;
        }
        emit_section(&folder.name, &in_folder, &mut idx);
    }
    let ungrouped: Vec<&SavedRequest> = data
        .requests
        .iter()
        .filter(|r| r.folder_id.is_none())
        .collect();
    if !ungrouped.is_empty() {
        emit_section("Ungrouped", &ungrouped, &mut idx);
    }
    let template = include_str!("docs_template.html");
    template
        .replace("{TITLE}", &esc(title))
        .replace("{ASCII}", &esc(ASCII))
        .replace("{REQUEST_COUNT}", &request_count.to_string())
        .replace("{FOLDER_COUNT}", &folder_count.to_string())
        .replace("{METHOD_COUNT}", &method_count.to_string())
        .replace("{GENERATED_AT}", &generated_at)
        .replace("{TOC}", &toc)
        .replace("{SECTIONS}", &sections)
}

fn request_badges(req: &SavedRequest) -> Vec<String> {
    let mut out = Vec::new();
    if !req.proxy_url.is_empty() {
        out.push("PROXY".to_owned());
    }
    if !req.verify_tls {
        out.push("TLS INSECURE".to_owned());
    }
    if !req.client_cert_path.is_empty() {
        out.push("MTLS".to_owned());
    }
    if !req.assertions.is_empty() {
        out.push(format!("{} ASSERT", req.assertions.len()));
    }
    if !req.extractors.is_empty() {
        out.push(format!("{} EXTRACT", req.extractors.len()));
    }
    out
}

fn describe_auth(auth: &Auth) -> (&'static str, Vec<(String, String)>) {
    match auth {
        Auth::None => ("none", Vec::new()),
        Auth::Bearer { token } => ("bearer", vec![("token".into(), redact(token))]),
        Auth::Basic { username, password } => (
            "basic",
            vec![
                ("username".into(), username.clone()),
                ("password".into(), redact(password)),
            ],
        ),
        Auth::ApiKey { header, value } => (
            "api-key",
            vec![
                ("header".into(), header.clone()),
                ("value".into(), redact(value)),
            ],
        ),
    }
}

fn redact(s: &str) -> String {
    if s.len() <= 6 {
        "•".repeat(s.len())
    } else {
        format!("{}…{}", &s[..3], "•".repeat(8))
    }
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

fn slug(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned()
}

fn current_date() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let days = secs / 86400;
    let (y, m, d) = epoch_days_to_ymd(days as i64);
    format!("{y:04}.{m:02}.{d:02}")
}

/// Converts days since 1970-01-01 to (year, month, day). Civil-from-days
/// algorithm by Howard Hinnant.
const fn epoch_days_to_ymd(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}
