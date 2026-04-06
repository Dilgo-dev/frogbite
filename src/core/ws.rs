//! Minimal WebSocket client used by the TUI.
//!
//! Spawns a background thread running a single-threaded tokio runtime driving
//! a `tokio-tungstenite` connection, and exchanges commands and events with
//! the synchronous UI thread via `std::sync::mpsc` channels.

use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderName;
use tokio_tungstenite::tungstenite::protocol::Message;

/// Commands the UI sends to the background WebSocket task.
#[derive(Debug, Clone)]
pub enum WsCommand {
    Send(String),
    SendBinary(Vec<u8>),
    SendPing(Vec<u8>),
    Close,
}

/// Events emitted by the background WebSocket task.
#[derive(Debug, Clone)]
pub enum WsEvent {
    Connected,
    Message(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Error(String),
    Closed,
}

/// Handle to a live WebSocket session held by the UI thread.
pub struct WsHandle {
    pub cmd_tx: Sender<WsCommand>,
    pub event_rx: Receiver<WsEvent>,
}

impl WsHandle {
    pub fn send(&self, text: String) {
        let _ = self.cmd_tx.send(WsCommand::Send(text));
    }

    pub fn send_binary(&self, data: Vec<u8>) {
        let _ = self.cmd_tx.send(WsCommand::SendBinary(data));
    }

    pub fn send_ping(&self, payload: Vec<u8>) {
        let _ = self.cmd_tx.send(WsCommand::SendPing(payload));
    }

    pub fn close(&self) {
        let _ = self.cmd_tx.send(WsCommand::Close);
    }
}

/// Opens a WebSocket connection in a background thread and returns a handle.
///
/// The returned handle is usable immediately; callers should poll
/// `event_rx` for `Connected`, `Message`, `Error`, and `Closed` events.
pub fn connect<S: BuildHasher + Send + 'static>(
    url: String,
    headers: HashMap<String, String, S>,
) -> WsHandle {
    let (cmd_tx, cmd_rx) = mpsc::channel::<WsCommand>();
    let (event_tx, event_rx) = mpsc::channel::<WsEvent>();

    thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                let _ = event_tx.send(WsEvent::Error(format!("runtime: {e}")));
                let _ = event_tx.send(WsEvent::Closed);
                return;
            }
        };
        runtime.block_on(run_session(url, headers, cmd_rx, event_tx));
    });

    WsHandle { cmd_tx, event_rx }
}

async fn run_session<S: BuildHasher + Send + 'static>(
    url: String,
    headers: HashMap<String, String, S>,
    cmd_rx: Receiver<WsCommand>,
    event_tx: Sender<WsEvent>,
) {
    let request = match build_request(&url, &headers) {
        Ok(req) => req,
        Err(e) => {
            let _ = event_tx.send(WsEvent::Error(e));
            let _ = event_tx.send(WsEvent::Closed);
            return;
        }
    };

    let stream = match tokio_tungstenite::connect_async(request).await {
        Ok((stream, _)) => stream,
        Err(e) => {
            let _ = event_tx.send(WsEvent::Error(format!("connect: {e}")));
            let _ = event_tx.send(WsEvent::Closed);
            return;
        }
    };

    let _ = event_tx.send(WsEvent::Connected);

    let (mut sink, mut reader) = stream.split();
    let (async_cmd_tx, mut async_cmd_rx) = tokio::sync::mpsc::unbounded_channel::<WsCommand>();
    let bridge = thread::spawn(move || {
        while let Ok(cmd) = cmd_rx.recv() {
            let stop = matches!(cmd, WsCommand::Close);
            if async_cmd_tx.send(cmd).is_err() {
                return;
            }
            if stop {
                return;
            }
        }
    });

    loop {
        tokio::select! {
            cmd = async_cmd_rx.recv() => {
                match cmd {
                    Some(WsCommand::Send(text)) => {
                        if let Err(e) = sink.send(Message::Text(text)).await {
                            let _ = event_tx.send(WsEvent::Error(format!("send: {e}")));
                            break;
                        }
                    }
                    Some(WsCommand::SendBinary(bytes)) => {
                        if let Err(e) = sink.send(Message::Binary(bytes)).await {
                            let _ = event_tx.send(WsEvent::Error(format!("send: {e}")));
                            break;
                        }
                    }
                    Some(WsCommand::SendPing(payload)) => {
                        if let Err(e) = sink.send(Message::Ping(payload)).await {
                            let _ = event_tx.send(WsEvent::Error(format!("send ping: {e}")));
                            break;
                        }
                    }
                    Some(WsCommand::Close) | None => {
                        let _ = sink.send(Message::Close(None)).await;
                        break;
                    }
                }
            }
            msg = reader.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let _ = event_tx.send(WsEvent::Message(text.as_str().to_owned()));
                    }
                    Some(Ok(Message::Binary(bytes))) => {
                        let _ = event_tx.send(WsEvent::Binary(bytes.clone()));
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        let _ = event_tx.send(WsEvent::Ping(payload.clone()));
                    }
                    Some(Ok(Message::Pong(payload))) => {
                        let _ = event_tx.send(WsEvent::Pong(payload.clone()));
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        let _ = event_tx.send(WsEvent::Error(format!("recv: {e}")));
                        break;
                    }
                }
            }
        }
    }

    let _ = event_tx.send(WsEvent::Closed);
    drop(bridge);
}

fn build_request<S: BuildHasher + Send + 'static>(
    url: &str,
    headers: &HashMap<String, String, S>,
) -> Result<tokio_tungstenite::tungstenite::handshake::client::Request, String> {
    let mut request = url
        .into_client_request()
        .map_err(|e| format!("invalid url: {e}"))?;
    let h = request.headers_mut();
    for (k, v) in headers {
        let name = HeaderName::try_from(k.as_str())
            .map_err(|e| format!("invalid header name {k}: {e}"))?;
        let value = v
            .parse()
            .map_err(|e| format!("invalid header value for {k}: {e}"))?;
        h.insert(name, value);
    }
    Ok(request)
}
