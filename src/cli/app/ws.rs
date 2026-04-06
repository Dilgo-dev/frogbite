use std::collections::HashMap;

use frogbite::core::ws::{self, WsEvent, WsHandle};

use super::App;

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

#[derive(Debug, Clone)]
pub struct WsMessage {
    pub direction: WsDirection,
    pub text: String,
}

#[derive(Default)]
pub struct WsState {
    pub status: WsStatus,
    pub messages: Vec<WsMessage>,
    pub input: String,
    pub input_editing: bool,
    pub scroll: u16,
    pub handle: Option<WsHandle>,
}

impl WsState {
    pub fn push(&mut self, direction: WsDirection, text: String) {
        self.messages.push(WsMessage { direction, text });
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
        let text = std::mem::take(&mut self.ws.input);
        if text.is_empty() {
            return;
        }
        if let Some(handle) = &self.ws.handle {
            handle.send(text.clone());
            self.ws.push(WsDirection::Sent, text);
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
