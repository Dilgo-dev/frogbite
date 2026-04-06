use frogbite::core::http::HttpResponse;
use serde::{Deserialize, Serialize};

/// Result of evaluating a single assertion against a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionResult {
    #[allow(dead_code)]
    pub expression: String,
    pub passed: bool,
    pub message: String,
}

/// Evaluates each assertion expression against the response and returns the
/// individual results in the same order.
#[must_use]
pub fn evaluate_all(assertions: &[String], response: &HttpResponse) -> Vec<AssertionResult> {
    assertions
        .iter()
        .map(|raw| evaluate(raw, response))
        .collect()
}

fn evaluate(raw: &str, response: &HttpResponse) -> AssertionResult {
    let expr = raw.trim();
    let outcome = parse_and_run(expr, response);
    match outcome {
        Ok(message) => AssertionResult {
            expression: raw.to_owned(),
            passed: true,
            message,
        },
        Err(message) => AssertionResult {
            expression: raw.to_owned(),
            passed: false,
            message,
        },
    }
}

#[allow(clippy::too_many_lines)]
fn parse_and_run(expr: &str, response: &HttpResponse) -> Result<String, String> {
    let mut parts = expr.split_whitespace();
    let kind = parts.next().ok_or_else(|| "empty assertion".to_owned())?;
    match kind {
        "status" => {
            let op = parts.next().ok_or_else(|| "missing operator".to_owned())?;
            let want_str = parts.next().ok_or_else(|| "missing value".to_owned())?;
            let want: u16 = want_str
                .parse()
                .map_err(|_| format!("invalid status: {want_str}"))?;
            let got = response.status;
            let ok = match op {
                "==" | "=" => got == want,
                "!=" => got != want,
                "<" => got < want,
                "<=" => got <= want,
                ">" => got > want,
                ">=" => got >= want,
                _ => return Err(format!("unknown operator: {op}")),
            };
            if ok {
                Ok(format!("status {got} {op} {want}"))
            } else {
                Err(format!("status {got} (expected {op} {want})"))
            }
        }
        "body" => {
            let op = parts.next().ok_or_else(|| "missing operator".to_owned())?;
            let (negated, op) = if op == "not" {
                let real = parts.next().ok_or_else(|| "missing operator".to_owned())?;
                (true, real)
            } else {
                (false, op)
            };
            let needle = remainder(&parts.collect::<Vec<_>>());
            let needle = strip_quotes(&needle);
            match op {
                "contains" => {
                    let found = response.body.contains(&needle);
                    let ok = if negated { !found } else { found };
                    if ok {
                        Ok(format!(
                            "body {} \"{needle}\"",
                            if negated { "not contains" } else { "contains" }
                        ))
                    } else {
                        Err(format!(
                            "body {} \"{needle}\"",
                            if negated {
                                "contains"
                            } else {
                                "does not contain"
                            }
                        ))
                    }
                }
                "==" | "=" => {
                    let ok = response.body.trim() == needle;
                    let ok = if negated { !ok } else { ok };
                    if ok {
                        Ok("body matches".to_owned())
                    } else {
                        Err("body does not match".to_owned())
                    }
                }
                _ => Err(format!("unknown body operator: {op}")),
            }
        }
        "header" => {
            let name = parts
                .next()
                .ok_or_else(|| "missing header name".to_owned())?;
            let op = parts.next().ok_or_else(|| "missing operator".to_owned())?;
            let value = response
                .headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.clone());
            match op {
                "exists" => value
                    .map(|_| format!("header {name} exists"))
                    .ok_or_else(|| format!("header {name} missing")),
                "missing" => {
                    if value.is_none() {
                        Ok(format!("header {name} missing"))
                    } else {
                        Err(format!("header {name} unexpectedly present"))
                    }
                }
                "==" | "=" | "contains" => {
                    let want = strip_quotes(&remainder(&parts.collect::<Vec<_>>()));
                    let Some(got) = value else {
                        return Err(format!("header {name} missing"));
                    };
                    let ok = if op == "contains" {
                        got.contains(&want)
                    } else {
                        got == want
                    };
                    if ok {
                        Ok(format!("header {name} {op} \"{want}\""))
                    } else {
                        Err(format!(
                            "header {name} = \"{got}\" (expected {op} \"{want}\")"
                        ))
                    }
                }
                _ => Err(format!("unknown header operator: {op}")),
            }
        }
        "json" => {
            let path = parts.next().ok_or_else(|| "missing json path".to_owned())?;
            let op = parts.next().ok_or_else(|| "missing operator".to_owned())?;
            let value: serde_json::Value = serde_json::from_str(&response.body)
                .map_err(|e| format!("body is not JSON: {e}"))?;
            let extracted = json_lookup(&value, path);
            match op {
                "exists" => extracted
                    .map(|v| format!("json {path} = {v}"))
                    .ok_or_else(|| format!("json {path} not found")),
                "missing" => {
                    if extracted.is_none() {
                        Ok(format!("json {path} missing"))
                    } else {
                        Err(format!("json {path} unexpectedly present"))
                    }
                }
                "==" | "=" | "!=" | "contains" => {
                    let want = strip_quotes(&remainder(&parts.collect::<Vec<_>>()));
                    let Some(got) = extracted else {
                        return Err(format!("json {path} not found"));
                    };
                    let ok = match op {
                        "contains" => got.contains(&want),
                        "!=" => got != want,
                        _ => got == want,
                    };
                    if ok {
                        Ok(format!("json {path} {op} \"{want}\""))
                    } else {
                        Err(format!(
                            "json {path} = \"{got}\" (expected {op} \"{want}\")"
                        ))
                    }
                }
                _ => Err(format!("unknown json operator: {op}")),
            }
        }
        _ => Err(format!(
            "unknown assertion kind: {kind} (try: status, body, header, json)"
        )),
    }
}

fn remainder(parts: &[&str]) -> String {
    parts.join(" ")
}

fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"') && s.len() >= 2)
        || (s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2)
    {
        s[1..s.len() - 1].to_owned()
    } else {
        s.to_owned()
    }
}

fn json_lookup(value: &serde_json::Value, expr: &str) -> Option<String> {
    let mut expr = expr.trim();
    if let Some(rest) = expr.strip_prefix('$') {
        expr = rest;
    }
    let expr = expr.trim_start_matches('.');

    let mut current = value;
    let mut buf = String::new();
    let mut chars = expr.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c == '.' {
            if !buf.is_empty() {
                current = current.get(buf.as_str())?;
                buf.clear();
            }
            chars.next();
        } else if c == '[' {
            if !buf.is_empty() {
                current = current.get(buf.as_str())?;
                buf.clear();
            }
            chars.next();
            let mut idx_str = String::new();
            while let Some(&c) = chars.peek() {
                if c == ']' {
                    chars.next();
                    break;
                }
                idx_str.push(c);
                chars.next();
            }
            let idx: usize = idx_str.trim().parse().ok()?;
            current = current.get(idx)?;
        } else {
            buf.push(c);
            chars.next();
        }
    }
    if !buf.is_empty() {
        current = current.get(buf.as_str())?;
    }
    Some(match current {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    })
}
