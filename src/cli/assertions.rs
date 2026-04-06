use frogbite::core::http::HttpResponse;
use serde::{Deserialize, Serialize};

/// Result of evaluating a single assertion against a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionResult {
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
    match parse_and_run(expr, response) {
        Ok(message) => AssertionResult {
            passed: true,
            message,
        },
        Err(message) => AssertionResult {
            passed: false,
            message,
        },
    }
}

fn parse_and_run(expr: &str, response: &HttpResponse) -> Result<String, String> {
    let mut parts = expr.split_whitespace();
    let kind = parts.next().ok_or_else(|| "empty assertion".to_owned())?;
    match kind {
        "status" => eval_status(&mut parts, response),
        "body" => eval_body(&mut parts, response),
        "header" => eval_header(&mut parts, response),
        "json" => eval_json(&mut parts, response),
        _ => Err(format!(
            "unknown assertion kind: {kind} (try: status, body, header, json)"
        )),
    }
}

fn eval_status(
    parts: &mut std::str::SplitWhitespace,
    response: &HttpResponse,
) -> Result<String, String> {
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

fn eval_body(
    parts: &mut std::str::SplitWhitespace,
    response: &HttpResponse,
) -> Result<String, String> {
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

fn eval_header(
    parts: &mut std::str::SplitWhitespace,
    response: &HttpResponse,
) -> Result<String, String> {
    let name = parts
        .next()
        .ok_or_else(|| "missing header name".to_owned())?
        .to_owned();
    let op = parts
        .next()
        .ok_or_else(|| "missing operator".to_owned())?
        .to_owned();
    let value = response
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(&name))
        .map(|(_, v)| v.clone());
    match op.as_str() {
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

fn eval_json(
    parts: &mut std::str::SplitWhitespace,
    response: &HttpResponse,
) -> Result<String, String> {
    let path = parts
        .next()
        .ok_or_else(|| "missing json path".to_owned())?
        .to_owned();
    let op = parts
        .next()
        .ok_or_else(|| "missing operator".to_owned())?
        .to_owned();
    let value: serde_json::Value =
        serde_json::from_str(&response.body).map_err(|e| format!("body is not JSON: {e}"))?;
    let extracted = crate::app::jsonpath_lookup(&value, &path);
    match op.as_str() {
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
            let ok = match op.as_str() {
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
