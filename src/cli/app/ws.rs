use std::collections::HashMap;

use frogbite::core::ws::{self, WsEvent, WsHandle};

use super::App;

fn parse_hex(s: &str) -> Result<Vec<u8>, String> {
    let cleaned: String = s
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ',')
        .collect();
    if cleaned.len() % 2 != 0 {
        return Err("odd number of hex digits".to_owned());
    }
    let mut out = Vec::with_capacity(cleaned.len() / 2);
    for pair in cleaned.as_bytes().chunks(2) {
        let hi = hex_digit(pair[0])?;
        let lo = hex_digit(pair[1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

const fn hex_digit(c: u8) -> Result<u8, String> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(String::new()),
    }
}

fn decode_base64(s: &str) -> Result<Vec<u8>, String> {
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = cleaned.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut buf = 0u32;
    let mut bits = 0u8;
    for &b in bytes {
        if b == b'=' {
            break;
        }
        let v = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return Err(format!("invalid base64 char: {}", b as char)),
        };
        buf = (buf << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xff) as u8);
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WsStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsDirection {
    Sent,
    Recv,
    Info,
    Error,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WsFormat {
    #[default]
    Text,
    Hex,
    Base64,
}

impl WsFormat {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Text => "TXT",
            Self::Hex => "HEX",
            Self::Base64 => "B64",
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::Text => Self::Hex,
            Self::Hex => Self::Base64,
            Self::Base64 => Self::Text,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WsMessage {
    pub direction: WsDirection,
    pub text: String,
    pub data: Option<Vec<u8>>,
}

#[derive(Default)]
pub struct WsState {
    pub status: WsStatus,
    pub messages: Vec<WsMessage>,
    pub input: String,
    pub input_editing: bool,
    pub input_format: WsFormat,
    pub view_format: WsFormat,
    pub scroll: u16,
    pub handle: Option<WsHandle>,
    pub upload_popup_open: bool,
    pub upload_buffer: String,
    pub upload_error: Option<String>,
}

impl WsState {
    pub fn push(&mut self, direction: WsDirection, text: String) {
        self.messages.push(WsMessage {
            direction,
            text,
            data: None,
        });
    }

    pub fn push_binary(&mut self, direction: WsDirection, data: Vec<u8>) {
        self.messages.push(WsMessage {
            direction,
            text: format!("<binary {} bytes>", data.len()),
            data: Some(data),
        });
    }
}

fn is_ws_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("ws://") || lower.starts_with("wss://")
}

impl App {
    pub fn request_url_is_ws(&self) -> bool {
        let resolved = self.resolve_variables(&self.request.url);
        is_ws_url(&resolved) || is_ws_url(&self.request.url)
    }

    pub const fn ws_active(&self) -> bool {
        !matches!(self.ws.status, WsStatus::Disconnected)
    }

    pub fn ws_connect(&mut self) {
        if matches!(self.ws.status, WsStatus::Connecting | WsStatus::Connected) {
            return;
        }
        let url = self.resolve_variables(&self.request.url);
        let headers: HashMap<String, String> = self
            .request
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), self.resolve_variables(v)))
            .collect();
        self.ws.messages.clear();
        self.ws.scroll = 0;
        self.ws.status = WsStatus::Connecting;
        self.ws
            .push(WsDirection::Info, format!("connecting to {url}"));
        self.ws.handle = Some(ws::connect(url, headers));
    }

    pub fn ws_disconnect(&self) {
        if let Some(handle) = &self.ws.handle {
            handle.close();
        }
    }

    pub fn ws_send_input(&mut self) {
        if !matches!(self.ws.status, WsStatus::Connected) {
            return;
        }
        let raw = std::mem::take(&mut self.ws.input);
        if raw.is_empty() {
            return;
        }
        let Some(handle) = &self.ws.handle else {
            return;
        };
        match self.ws.input_format {
            WsFormat::Text => {
                handle.send(raw.clone());
                self.ws.push(WsDirection::Sent, raw);
            }
            WsFormat::Hex => match parse_hex(&raw) {
                Ok(bytes) => {
                    handle.send_binary(bytes.clone());
                    self.ws.push_binary(WsDirection::Sent, bytes);
                }
                Err(e) => {
                    self.ws.input = raw;
                    self.ws.push(WsDirection::Error, format!("hex parse: {e}"));
                }
            },
            WsFormat::Base64 => match decode_base64(&raw) {
                Ok(bytes) => {
                    handle.send_binary(bytes.clone());
                    self.ws.push_binary(WsDirection::Sent, bytes);
                }
                Err(e) => {
                    self.ws.input = raw;
                    self.ws
                        .push(WsDirection::Error, format!("base64 parse: {e}"));
                }
            },
        }
    }

    pub const fn ws_cycle_input_format(&mut self) {
        self.ws.input_format = self.ws.input_format.next();
    }

    pub const fn ws_cycle_view_format(&mut self) {
        self.ws.view_format = self.ws.view_format.next();
    }

    pub fn ws_open_upload_popup(&mut self) {
        if !matches!(self.ws.status, WsStatus::Connected) {
            return;
        }
        self.ws.upload_buffer.clear();
        self.ws.upload_error = None;
        self.ws.upload_popup_open = true;
    }

    pub fn ws_confirm_upload(&mut self) {
        let path = self.ws.upload_buffer.trim().to_owned();
        if path.is_empty() {
            self.ws.upload_error = Some("path is empty".to_owned());
            return;
        }
        match std::fs::read(&path) {
            Ok(bytes) => {
                if let Some(handle) = &self.ws.handle {
                    handle.send_binary(bytes.clone());
                    self.ws.push_binary(WsDirection::Sent, bytes);
                    self.ws.upload_popup_open = false;
                    self.ws.upload_error = None;
                }
            }
            Err(e) => {
                self.ws.upload_error = Some(format!("read: {e}"));
            }
        }
    }

    pub fn ws_clear_stream(&mut self) {
        self.ws.messages.clear();
        self.ws.scroll = 0;
    }

    pub fn ws_reset(&mut self) {
        self.ws.handle = None;
        self.ws.status = WsStatus::Disconnected;
        self.ws.messages.clear();
        self.ws.input.clear();
        self.ws.input_editing = false;
        self.ws.scroll = 0;
    }

    pub fn poll_ws(&mut self) {
        if self.ws.handle.is_none() {
            return;
        }
        loop {
            let recv = self.ws.handle.as_ref().map(|h| h.event_rx.try_recv());
            let Some(recv) = recv else { break };
            match recv {
                Ok(WsEvent::Connected) => {
                    self.ws.status = WsStatus::Connected;
                    self.ws.push(WsDirection::Info, "connected".to_owned());
                }
                Ok(WsEvent::Message(text)) => {
                    self.ws.push(WsDirection::Recv, text);
                }
                Ok(WsEvent::Binary(data)) => {
                    self.ws.push_binary(WsDirection::Recv, data);
                }
                Ok(WsEvent::Error(e)) => {
                    self.ws.push(WsDirection::Error, e);
                }
                Ok(WsEvent::Closed) => {
                    self.ws.status = WsStatus::Closed;
                    self.ws.push(WsDirection::Info, "closed".to_owned());
                    self.ws.handle = None;
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.ws.status = WsStatus::Closed;
                    self.ws.handle = None;
                    break;
                }
            }
        }
    }
}
