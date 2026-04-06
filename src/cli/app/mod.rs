mod auth_env;
mod clipboard;
mod editing;
mod import;
mod kv_editor;
mod popups_send;
mod sidebar;
mod url_utils;
mod vars_search;

pub use kv_editor::KvEditorState;

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver};

use frogbite::core::http::{HttpResponse, RequestOptions};

struct PendingRequest {
    rx: Receiver<Result<HttpResponse, String>>,
    method: String,
    url: String,
    resolved_url: String,
    body: String,
    request_name: Option<String>,
}

use crate::assertions::{self, AssertionResult};
use crate::collections::{self, Auth, BodyType, CollectionData, ContentType, Folder, SavedRequest};
use crate::cookies::{self, CookieStore};
use crate::curl;
use crate::environments::{self, Environment, Variable};
use crate::history::{self, HistoryEntry};
use crate::postman;
use crate::settings::{self, Settings};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Method {
    #[default]
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl Method {
    pub const fn as_str(&self) -> &str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }

    pub const fn all() -> &'static [Self] {
        &[
            Self::Get,
            Self::Post,
            Self::Put,
            Self::Patch,
            Self::Delete,
            Self::Head,
            Self::Options,
        ]
    }

    pub fn next(&self) -> Self {
        let all = Self::all();
        let idx = all.iter().position(|m| m == self).unwrap_or(0);
        all[(idx + 1) % all.len()].clone()
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "POST" => Self::Post,
            "PUT" => Self::Put,
            "PATCH" => Self::Patch,
            "DELETE" => Self::Delete,
            "HEAD" => Self::Head,
            "OPTIONS" => Self::Options,
            _ => Self::Get,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum View {
    #[default]
    Main,
    Settings,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Focus {
    #[default]
    Sidebar,
    UrlBar,
    Body,
    Response,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ResponseTab {
    #[default]
    Body,
    Headers,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RequestTab {
    #[default]
    Body,
    Headers,
    Auth,
    Params,
}

#[derive(Default)]
pub struct RequestState {
    pub method: Method,
    pub url: String,
    pub body: String,
    pub headers: HashMap<String, String>,
    pub cursor_pos: usize,
    pub body_row: usize,
    pub body_col: usize,
    pub editing_url: bool,
    pub editing_body: bool,
    pub tab: RequestTab,
    pub body_type: BodyType,
    pub content_type: ContentType,
    pub form_editor: KvEditorState,
    pub header_editor: KvEditorState,
    pub param_editor: KvEditorState,
}

#[derive(Default)]
pub struct SidebarState {
    pub folders: Vec<Folder>,
    pub requests: Vec<SavedRequest>,
    pub selected: usize,
    pub active_request_id: Option<String>,
    pub editing_name: bool,
    pub edit_buffer: String,
    pub confirm_delete: bool,
}

#[derive(Default)]
pub struct ResponseState {
    pub last: Option<Result<HttpResponse, String>>,
    pub tab: ResponseTab,
    pub scroll: u16,
    pub search: String,
    pub searching: bool,
    pub search_buf: String,
    pub match_idx: usize,
    pub loading: bool,
    pub last_bodies: HashMap<String, String>,
    pub clipboard_msg: Option<String>,
}

#[derive(Default)]
pub struct AuthState {
    pub config: Auth,
    pub selecting_type: bool,
    pub type_selected: usize,
    pub editing: bool,
    pub field: usize,
    pub buf_a: String,
    pub buf_b: String,
}

#[derive(Default)]
pub struct EnvState {
    pub environments: Vec<Environment>,
    pub active_id: Option<String>,
    pub popup_open: bool,
    pub popup_selected: usize,
    pub renaming: bool,
    pub name_buffer: String,
    pub editor: EnvEditorState,
    pub import: EnvImportState,
}

#[derive(Default)]
pub struct EnvEditorState {
    pub open: bool,
    pub id: String,
    pub selected: usize,
    pub editing_var: bool,
    pub var_key_buffer: String,
    pub var_value_buffer: String,
    pub var_field: usize,
}

#[derive(Default)]
pub struct EnvImportState {
    pub open: bool,
    pub buffer: String,
    pub error: bool,
}

#[derive(Default)]
pub struct HistoryState {
    pub entries: Vec<HistoryEntry>,
    pub open: bool,
    pub selected: usize,
}

#[derive(Default)]
pub struct CurlIoState {
    pub import_open: bool,
    pub import_buffer: String,
    pub import_error: bool,
    pub export_open: bool,
    pub export_content: String,
}

#[derive(Default)]
pub struct PostmanIoState {
    pub open: bool,
    pub buffer: String,
    pub error: bool,
}

#[derive(Default)]
pub struct MethodPopupState {
    pub open: bool,
    pub selected: usize,
}

pub struct TimeoutState {
    pub secs: u64,
    pub popup_open: bool,
    pub buffer: String,
    pub error: bool,
}

impl Default for TimeoutState {
    fn default() -> Self {
        Self {
            secs: collections::default_timeout(),
            popup_open: false,
            buffer: String::new(),
            error: false,
        }
    }
}

pub struct TlsState {
    pub verify: bool,
    pub ca_cert: String,
    pub client_cert: String,
    pub client_key: String,
    pub min_version: String,
    pub popup_open: bool,
    pub popup_selected: usize,
    pub editing: bool,
    pub edit_buffer: String,
}

impl Default for TlsState {
    fn default() -> Self {
        Self {
            verify: true,
            ca_cert: String::new(),
            client_cert: String::new(),
            client_key: String::new(),
            min_version: String::new(),
            popup_open: false,
            popup_selected: 0,
            editing: false,
            edit_buffer: String::new(),
        }
    }
}

#[derive(Default)]
pub struct CookiesState {
    pub store: CookieStore,
    pub popup_open: bool,
    pub popup_selected: usize,
}

#[derive(Default)]
pub struct ExtractorsState {
    pub editor: KvEditorState,
    pub popup_open: bool,
    pub extracted: HashMap<String, String>,
}

#[derive(Default)]
pub struct AssertionsState {
    pub exprs: Vec<String>,
    pub results: Vec<AssertionResult>,
    pub popup_open: bool,
    pub selected: usize,
    pub editing: bool,
    pub edit_buffer: String,
    pub editing_existing: bool,
}

#[derive(Default)]
pub struct SettingsView {
    pub view: View,
    pub settings: Settings,
    pub settings_selected: usize,
    pub focus: Focus,
}

#[derive(Default)]
pub struct App {
    pub request: RequestState,
    pub sidebar: SidebarState,
    pub response: ResponseState,
    pub auth: AuthState,
    pub env: EnvState,
    pub history: HistoryState,
    pub curl_io: CurlIoState,
    pub postman_io: PostmanIoState,
    pub method_popup: MethodPopupState,
    pub timeout: TimeoutState,
    pub tls: TlsState,
    pub cookies: CookiesState,
    pub extractors: ExtractorsState,
    pub assertions: AssertionsState,
    pub ui: SettingsView,
    pub follow_redirects: bool,
    pub update_available: Option<String>,
    update_check_rx: Option<Receiver<String>>,
    pending: Option<PendingRequest>,
}

impl App {
    pub fn new() -> Self {
        let settings = settings::load();
        let data = collections::load();
        let env_data = environments::load();

        let (folders, requests, active_id) = if data.requests.is_empty() {
            let defaults = default_collection();
            collections::save(&defaults);
            (
                defaults.folders,
                defaults.requests,
                defaults.active_request_id,
            )
        } else {
            (data.folders, data.requests, data.active_request_id)
        };

        let mut app = Self {
            sidebar: SidebarState {
                folders,
                requests,
                active_request_id: active_id,
                ..Default::default()
            },
            env: EnvState {
                environments: env_data.environments,
                active_id: env_data.active_id,
                ..Default::default()
            },
            history: HistoryState {
                entries: history::load(),
                ..Default::default()
            },
            cookies: CookiesState {
                store: cookies::load(),
                ..Default::default()
            },
            ui: SettingsView {
                settings,
                ..Default::default()
            },
            follow_redirects: true,
            ..Self::default()
        };

        if let Some(id) = app.sidebar.active_request_id.clone() {
            if let Some(idx) = app.sidebar.requests.iter().position(|r| r.id == id) {
                app.sidebar.selected = idx;
                app.load_request_by_id(&id);
            }
        } else if !app.sidebar.requests.is_empty() {
            app.load_request_at(0);
        }

        app
    }

    pub fn set_update_check_rx(&mut self, rx: Receiver<String>) {
        self.update_check_rx = Some(rx);
    }

    pub fn poll_update_check(&mut self) {
        let Some(rx) = self.update_check_rx.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok(version) => {
                self.update_available = Some(version);
                self.update_check_rx = None;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.update_check_rx = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }
}

#[derive(Debug, Clone)]
pub enum SidebarItem {
    Folder(Folder),
    Request(Box<SavedRequest>),
    NewRequest,
}

/// Resolves a JSONPath-lite expression against a JSON value and returns the
/// extracted value as a string.
///
/// Supports `$` root, `.field` object access, `[N]` array index, and chains
/// thereof: `$.data.users[0].name`, `data.items[2].id`, `$.token`.
pub fn jsonpath_lookup(value: &serde_json::Value, expr: &str) -> Option<String> {
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

fn default_collection() -> CollectionData {
    let folder_id = collections::new_id();
    CollectionData {
        folders: vec![Folder {
            id: folder_id.clone(),
            name: "Examples".into(),
            expanded: true,
        }],
        requests: vec![
            SavedRequest {
                id: collections::new_id(),
                name: "Get User".into(),
                method: "GET".into(),
                url: "https://jsonplaceholder.typicode.com/users/1".into(),
                body: String::new(),
                headers: HashMap::new(),
                folder_id: Some(folder_id.clone()),
                auth: Auth::None,
                body_type: BodyType::Raw,
                content_type: ContentType::Json,
                form_data: Vec::new(),
                follow_redirects: true,
                timeout_secs: collections::default_timeout(),
                verify_tls: true,
                ca_cert_path: String::new(),
                client_cert_path: String::new(),
                client_key_path: String::new(),
                tls_min_version: String::new(),
                extractors: Vec::new(),
                assertions: Vec::new(),
                last_response: None,
                last_error: None,
                last_assertion_results: Vec::new(),
            },
            SavedRequest {
                id: collections::new_id(),
                name: "Create User".into(),
                method: "POST".into(),
                url: "https://jsonplaceholder.typicode.com/users".into(),
                body: "{\n  \"name\": \"Ada Lovelace\",\n  \"email\": \"ada@example.com\"\n}"
                    .into(),
                headers: HashMap::new(),
                folder_id: Some(folder_id),
                auth: Auth::None,
                body_type: BodyType::Raw,
                content_type: ContentType::Json,
                form_data: Vec::new(),
                follow_redirects: true,
                timeout_secs: collections::default_timeout(),
                verify_tls: true,
                ca_cert_path: String::new(),
                client_cert_path: String::new(),
                client_key_path: String::new(),
                tls_min_version: String::new(),
                extractors: Vec::new(),
                assertions: Vec::new(),
                last_response: None,
                last_error: None,
                last_assertion_results: Vec::new(),
            },
        ],
        active_request_id: None,
    }
}
