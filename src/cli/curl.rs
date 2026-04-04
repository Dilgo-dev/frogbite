use std::collections::HashMap;

pub struct ParsedCurl {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

pub fn parse(input: &str) -> Option<ParsedCurl> {
    let tokens = tokenize(input);
    if tokens.is_empty() {
        return None;
    }

    let mut method: Option<String> = None;
    let mut url: Option<String> = None;
    let mut headers = HashMap::new();
    let mut body = String::new();
    let mut has_data = false;

    let mut i = 0;
    while i < tokens.len() {
        let token = &tokens[i];
        match token.as_str() {
            "curl" => {}
            "-X" | "--request" => {
                i += 1;
                if let Some(m) = tokens.get(i) {
                    method = Some(m.to_uppercase());
                }
            }
            "-H" | "--header" => {
                i += 1;
                if let Some(h) = tokens.get(i) {
                    if let Some((key, value)) = h.split_once(':') {
                        headers.insert(key.trim().to_owned(), value.trim().to_owned());
                    }
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" => {
                i += 1;
                if let Some(d) = tokens.get(i) {
                    body.clone_from(d);
                    has_data = true;
                }
            }
            "-u" | "--user" => {
                i += 1;
                if let Some(creds) = tokens.get(i) {
                    headers.insert(
                        "Authorization".to_owned(),
                        format!("Basic {}", base64(creds.as_bytes())),
                    );
                }
            }
            t if t.starts_with('-') => {
                // Skip unknown flags; consume next token if it doesn't look like a flag
                if let Some(next) = tokens.get(i + 1) {
                    if !next.starts_with('-') {
                        i += 1;
                    }
                }
            }
            _ if url.is_none() => {
                url = Some(token.clone());
            }
            _ => {}
        }
        i += 1;
    }

    let url = url?;
    let method = method.unwrap_or_else(|| if has_data { "POST" } else { "GET" }.to_owned());

    Some(ParsedCurl {
        method,
        url,
        headers,
        body,
    })
}

fn tokenize(input: &str) -> Vec<String> {
    let normalized = input.replace("\\\n", " ").replace("\\\r\n", " ");
    let chars: Vec<char> = normalized.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            c if c.is_whitespace() => i += 1,
            '\'' => {
                i += 1;
                let mut token = String::new();
                while i < chars.len() && chars[i] != '\'' {
                    token.push(chars[i]);
                    i += 1;
                }
                i += 1; // closing quote
                tokens.push(token);
            }
            '"' => {
                i += 1;
                let mut token = String::new();
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 1;
                    }
                    token.push(chars[i]);
                    i += 1;
                }
                i += 1; // closing quote
                tokens.push(token);
            }
            _ => {
                let mut token = String::new();
                while i < chars.len() && !chars[i].is_whitespace() {
                    token.push(chars[i]);
                    i += 1;
                }
                tokens.push(token);
            }
        }
    }

    tokens
}

fn base64(input: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;

    while i < input.len() {
        let b0 = u32::from(input[i]);
        let b1 = if i + 1 < input.len() {
            u32::from(input[i + 1])
        } else {
            0
        };
        let b2 = if i + 2 < input.len() {
            u32::from(input[i + 2])
        } else {
            0
        };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        out.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);

        if i + 1 < input.len() {
            out.push(TABLE[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }

        if i + 2 < input.len() {
            out.push(TABLE[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }

        i += 3;
    }

    out
}
