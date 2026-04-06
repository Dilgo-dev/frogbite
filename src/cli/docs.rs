//! Generates a single self-contained HTML reference from a frogbite
//! collection. Uses the brand palette (lime on near-black) but a layout
//! that mirrors a technical specification document, not the marketing
//! landing page: sticky sidebar nav, numbered sections, dotted leaders.

#![allow(
    clippy::too_many_lines,
    clippy::cast_possible_wrap,
    clippy::write_with_newline,
    clippy::uninlined_format_args
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

    let mut nav = String::new();
    let mut sections = String::new();

    let mut groups: Vec<(String, Vec<&SavedRequest>)> = Vec::new();
    for folder in &data.folders {
        let in_folder: Vec<&SavedRequest> = data
            .requests
            .iter()
            .filter(|r| r.folder_id.as_deref() == Some(folder.id.as_str()))
            .collect();
        if !in_folder.is_empty() {
            groups.push((folder.name.clone(), in_folder));
        }
    }
    let ungrouped: Vec<&SavedRequest> = data
        .requests
        .iter()
        .filter(|r| r.folder_id.is_none())
        .collect();
    if !ungrouped.is_empty() {
        groups.push(("Ungrouped".to_owned(), ungrouped));
    }

    for (folder_idx, (label, requests)) in groups.iter().enumerate() {
        let folder_no = folder_idx + 1;
        let folder_anchor = slug(label);

        let _ = write!(
            nav,
            "  <div class=\"nav-folder\">{:02} / {}</div>\n  <nav class=\"nav-list\">\n",
            folder_no,
            esc(label)
        );

        let _ = write!(
            sections,
            "  <section class=\"folder\" id=\"folder-{folder_anchor}\">\n    <div class=\"folder-marker\">\n      <span class=\"folder-tag\">SECTION {folder_no:02}</span>\n      <h2 class=\"folder-title\">{name}</h2>\n      <span class=\"folder-count\">{count} requests</span>\n    </div>\n",
            folder_anchor = folder_anchor,
            folder_no = folder_no,
            name = esc(label),
            count = requests.len(),
        );

        for (req_idx, req) in requests.iter().enumerate() {
            let req_no = req_idx + 1;
            let section_no = format!("{folder_no:02}.{req_no:02}");
            let anchor = format!("req-{}-{}", folder_anchor, slug(&req.name));
            let m = esc(&req.method);
            let name = esc(&req.name);
            let url = esc(&req.url);

            let _ = write!(
                nav,
                "    <a class=\"nav-item\" href=\"#{anchor}\">\n      <span class=\"nav-num\">{section_no}</span>\n      <span class=\"nav-pill m-{m}\">{m}</span>\n      <span class=\"nav-name\">{name}</span>\n    </a>\n",
                anchor = anchor,
                section_no = section_no,
                m = m,
                name = name,
            );

            let _ = write!(
                sections,
                "    <article class=\"request\" id=\"{anchor}\">\n      <div class=\"request-head\">\n        <span class=\"request-num\">{section_no}</span>\n        <span class=\"method m-{m}\">{m}</span>\n        <h3 class=\"request-name\">{name}</h3>\n        <code class=\"request-url\">{url}</code>\n",
                anchor = anchor,
                section_no = section_no,
                m = m,
                name = name,
                url = url,
            );

            for badge in request_badges(req) {
                let _ = write!(
                    sections,
                    "        <span class=\"badge\">{}</span>\n",
                    esc(&badge)
                );
            }

            sections.push_str("      </div>\n      <div class=\"request-body\">\n");

            let mut sub_no = 0usize;

            if !matches!(req.auth, Auth::None) {
                sub_no += 1;
                emit_spec_open(&mut sections, sub_no, "Auth");
                let (kind, fields) = describe_auth(&req.auth);
                sections.push_str("          <div class=\"kv\">\n");
                let _ = write!(
                    sections,
                    "            <div class=\"kv-row\"><div class=\"kv-key\">type</div><div class=\"kv-val\">{}</div></div>\n",
                    esc(kind)
                );
                for (k, v) in fields {
                    let _ = write!(
                        sections,
                        "            <div class=\"kv-row\"><div class=\"kv-key\">{}</div><div class=\"kv-val\">{}</div></div>\n",
                        esc(&k),
                        esc(&v)
                    );
                }
                sections.push_str("          </div>\n");
                emit_spec_close(&mut sections);
            }

            if !req.headers.is_empty() {
                sub_no += 1;
                emit_spec_open(&mut sections, sub_no, "Headers");
                let mut entries: Vec<(&String, &String)> = req.headers.iter().collect();
                entries.sort_by(|a, b| a.0.cmp(b.0));
                sections.push_str("          <div class=\"kv\">\n");
                for (k, v) in entries {
                    let _ = write!(
                        sections,
                        "            <div class=\"kv-row\"><div class=\"kv-key\">{}</div><div class=\"kv-val\">{}</div></div>\n",
                        esc(k),
                        esc(v)
                    );
                }
                sections.push_str("          </div>\n");
                emit_spec_close(&mut sections);
            }

            match req.body_type {
                BodyType::Raw if !req.body.is_empty() => {
                    sub_no += 1;
                    let label = format!("Body / {}", req.content_type.label());
                    emit_spec_open(&mut sections, sub_no, &label);
                    let lang = req.content_type.label().to_lowercase();
                    let _ = write!(
                        sections,
                        "          <pre class=\"code\" data-lang=\"{}\">{}</pre>\n",
                        esc(&lang),
                        esc(&req.body)
                    );
                    emit_spec_close(&mut sections);
                }
                BodyType::Form | BodyType::Multipart if !req.form_data.is_empty() => {
                    sub_no += 1;
                    let label = if matches!(req.body_type, BodyType::Form) {
                        "Body / form-urlencoded"
                    } else {
                        "Body / multipart"
                    };
                    emit_spec_open(&mut sections, sub_no, label);
                    sections.push_str("          <div class=\"kv\">\n");
                    for (k, v) in &req.form_data {
                        let _ = write!(
                            sections,
                            "            <div class=\"kv-row\"><div class=\"kv-key\">{}</div><div class=\"kv-val\">{}</div></div>\n",
                            esc(k),
                            esc(v)
                        );
                    }
                    sections.push_str("          </div>\n");
                    emit_spec_close(&mut sections);
                }
                _ => {}
            }

            if req.method == "GRPC" && (!req.proto_path.is_empty() || !req.grpc_method.is_empty()) {
                sub_no += 1;
                emit_spec_open(&mut sections, sub_no, "gRPC");
                sections.push_str("          <div class=\"kv\">\n");
                if !req.proto_path.is_empty() {
                    let _ = write!(
                        sections,
                        "            <div class=\"kv-row\"><div class=\"kv-key\">proto</div><div class=\"kv-val\">{}</div></div>\n",
                        esc(&req.proto_path)
                    );
                }
                if !req.grpc_method.is_empty() {
                    let _ = write!(
                        sections,
                        "            <div class=\"kv-row\"><div class=\"kv-key\">method</div><div class=\"kv-val\">{}</div></div>\n",
                        esc(&req.grpc_method)
                    );
                }
                sections.push_str("          </div>\n");
                emit_spec_close(&mut sections);
            }

            if req.method == "GQL" && !req.gql_variables.is_empty() {
                sub_no += 1;
                emit_spec_open(&mut sections, sub_no, "Variables");
                let _ = write!(
                    sections,
                    "          <pre class=\"code\" data-lang=\"json\">{}</pre>\n",
                    esc(&req.gql_variables)
                );
                emit_spec_close(&mut sections);
            }

            if let Some(resp) = &req.last_response {
                if !resp.body.is_empty() {
                    sub_no += 1;
                    emit_spec_open(&mut sections, sub_no, "Sample response");
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
                    let _ = write!(
                        sections,
                        "          <div class=\"resp-meta\"><strong>{}</strong> {} / {} ms</div>\n",
                        resp.status,
                        esc(&resp.status_text),
                        resp.duration_ms,
                    );
                    let _ = write!(
                        sections,
                        "          <pre class=\"code\" data-lang=\"json\">{}</pre>\n",
                        esc(&truncated)
                    );
                    emit_spec_close(&mut sections);
                }
            }

            if sub_no == 0 {
                sections.push_str("        <div class=\"spec-section\"><div class=\"spec-section-marker\"><span class=\"roman\">-</span>EMPTY</div><div class=\"spec-section-body\" style=\"color:var(--fg-dim);font-style:italic\">No additional metadata.</div></div>\n");
            }

            sections.push_str("      </div>\n    </article>\n");
        }

        sections.push_str("  </section>\n");
        nav.push_str("  </nav>\n");
    }

    let template = include_str!("docs_template.html");
    template
        .replace("{TITLE}", &esc(title))
        .replace("{ASCII}", &esc(ASCII))
        .replace("{REQUEST_COUNT}", &request_count.to_string())
        .replace("{FOLDER_COUNT}", &folder_count.to_string())
        .replace("{METHOD_COUNT}", &method_count.to_string())
        .replace("{GENERATED_AT}", &generated_at)
        .replace("{NAV}", &nav)
        .replace("{SECTIONS}", &sections)
}

fn emit_spec_open(out: &mut String, n: usize, label: &str) {
    let _ = write!(
        out,
        "        <div class=\"spec-section\">\n          <div class=\"spec-section-marker\"><span class=\"roman\">{}.</span>{}</div>\n          <div class=\"spec-section-body\">\n",
        roman_numeral(n),
        esc(label),
    );
}

fn emit_spec_close(out: &mut String) {
    out.push_str("          </div>\n        </div>\n");
}

const fn roman_numeral(n: usize) -> &'static str {
    match n {
        1 => "I",
        2 => "II",
        3 => "III",
        4 => "IV",
        5 => "V",
        6 => "VI",
        7 => "VII",
        8 => "VIII",
        _ => "IX",
    }
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
        "*".repeat(s.len())
    } else {
        format!("{}...{}", &s[..3], "*".repeat(8))
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
