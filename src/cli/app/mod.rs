mod clipboard;
mod kv_editor;
mod url_utils;

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

    pub fn save_collections(&self) {
        let data = CollectionData {
            folders: self.sidebar.folders.clone(),
            requests: self.sidebar.requests.clone(),
            active_request_id: self.sidebar.active_request_id.clone(),
        };
        collections::save(&data);
    }

    /// Builds a flat list of sidebar items: folders (with their requests) then ungrouped.
    pub fn sidebar_items(&self) -> Vec<SidebarItem> {
        let mut items = Vec::new();

        for folder in &self.sidebar.folders {
            items.push(SidebarItem::Folder(folder.clone()));
            if folder.expanded {
                for req in &self.sidebar.requests {
                    if req.folder_id.as_deref() == Some(&folder.id) {
                        items.push(SidebarItem::Request(Box::new(req.clone())));
                    }
                }
            }
        }

        for req in &self.sidebar.requests {
            if req.folder_id.is_none() {
                items.push(SidebarItem::Request(Box::new(req.clone())));
            }
        }

        items.push(SidebarItem::NewRequest);

        items
    }

    // -- Settings --

    pub fn settings_items(&self) -> Vec<(&str, bool)> {
        vec![
            ("Splash animation", self.ui.settings.splash_animation),
            ("Vim keys", self.ui.settings.vim_keys),
        ]
    }

    pub fn toggle_setting(&mut self) {
        match self.ui.settings_selected {
            0 => self.ui.settings.splash_animation = !self.ui.settings.splash_animation,
            1 => self.ui.settings.vim_keys = !self.ui.settings.vim_keys,
            _ => {}
        }
        settings::save(&self.ui.settings);
    }

    // -- Method popup --

    pub fn open_method_popup(&mut self) {
        self.method_popup.selected = Method::all()
            .iter()
            .position(|m| m == &self.request.method)
            .unwrap_or(0);
        self.method_popup.open = true;
    }

    pub fn confirm_method_popup(&mut self) {
        self.request.method = Method::all()[self.method_popup.selected].clone();
        self.method_popup.open = false;
        self.sync_to_collection();
    }

    // -- Sidebar navigation --

    pub fn sidebar_len(&self) -> usize {
        self.sidebar_items().len()
    }

    pub fn selected_sidebar_item(&self) -> Option<SidebarItem> {
        self.sidebar_items().get(self.sidebar.selected).cloned()
    }

    pub fn toggle_folder_at_cursor(&mut self) {
        if let Some(SidebarItem::Folder(f)) = self.selected_sidebar_item() {
            if let Some(folder) = self.sidebar.folders.iter_mut().find(|fo| fo.id == f.id) {
                folder.expanded = !folder.expanded;
            }
            self.save_collections();
        }
    }

    pub fn load_selected(&mut self) {
        match self.selected_sidebar_item() {
            Some(SidebarItem::Request(req)) => self.load_request_by_id(&req.id),
            Some(SidebarItem::Folder(_)) => self.toggle_folder_at_cursor(),
            Some(SidebarItem::NewRequest) => self.create_request_at_root(),
            None => {}
        }
    }

    fn load_request_by_id(&mut self, id: &str) {
        if let Some(req) = self.sidebar.requests.iter().find(|r| r.id == id) {
            self.request.method = Method::from_str(&req.method);
            self.request.url = req.url.clone();
            self.request.body = req.body.clone();
            self.request.headers = req.headers.clone();
            self.auth.config = req.auth.clone();
            self.request.body_type = req.body_type;
            self.request.content_type = req.content_type;
            self.request.form_editor.entries = req.form_data.clone();
            self.follow_redirects = req.follow_redirects;
            self.timeout.secs = req.timeout_secs;
            self.tls.verify = req.verify_tls;
            self.tls.ca_cert.clone_from(&req.ca_cert_path);
            self.tls.client_cert.clone_from(&req.client_cert_path);
            self.tls.client_key.clone_from(&req.client_key_path);
            self.tls.min_version.clone_from(&req.tls_min_version);
            self.extractors.editor.entries.clone_from(&req.extractors);
            self.extractors.editor.selected = 0;
            self.extractors.editor.editing = false;
            self.assertions.exprs.clone_from(&req.assertions);
            self.assertions
                .results
                .clone_from(&req.last_assertion_results);
            self.assertions.selected = 0;
            self.response.last = match (&req.last_response, &req.last_error) {
                (Some(r), _) => Some(Ok(r.clone())),
                (None, Some(e)) => Some(Err(e.clone())),
                _ => None,
            };
            self.request.form_editor.selected = 0;
            self.request.form_editor.editing = false;
            self.request.cursor_pos = self.request.url.len();
            self.request.body_row = 0;
            self.request.body_col = 0;
            self.response.scroll = 0;
            self.sidebar.active_request_id = Some(id.to_owned());
            self.sync_headers_from_map();
            self.parse_params_from_url();
            self.request.tab = RequestTab::Body;
            self.save_collections();
        }
    }

    fn load_request_at(&mut self, idx: usize) {
        if let Some(req) = self.sidebar.requests.get(idx) {
            let id = req.id.clone();
            self.load_request_by_id(&id);
        }
    }

    /// Syncs current editor state back to the collection.
    pub fn sync_to_collection(&mut self) {
        self.sync_headers_to_map();
        let Some(id) = self.sidebar.active_request_id.clone() else {
            return;
        };
        if let Some(req) = self.sidebar.requests.iter_mut().find(|r| r.id == id) {
            self.request.method.as_str().clone_into(&mut req.method);
            req.url.clone_from(&self.request.url);
            req.body.clone_from(&self.request.body);
            req.headers.clone_from(&self.request.headers);
            req.auth.clone_from(&self.auth.config);
            req.body_type = self.request.body_type;
            req.content_type = self.request.content_type;
            req.form_data.clone_from(&self.request.form_editor.entries);
            req.follow_redirects = self.follow_redirects;
            req.timeout_secs = self.timeout.secs;
            req.verify_tls = self.tls.verify;
            req.ca_cert_path.clone_from(&self.tls.ca_cert);
            req.client_cert_path.clone_from(&self.tls.client_cert);
            req.client_key_path.clone_from(&self.tls.client_key);
            req.tls_min_version.clone_from(&self.tls.min_version);
            req.extractors.clone_from(&self.extractors.editor.entries);
            req.assertions.clone_from(&self.assertions.exprs);
        }
        self.save_collections();
    }

    fn sync_headers_from_map(&mut self) {
        let mut entries: Vec<(String, String)> = self
            .request
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        self.request.header_editor.entries = entries;
        self.request.header_editor.selected = 0;
        self.request.header_editor.editing = false;
    }

    fn sync_headers_to_map(&mut self) {
        self.request.headers = self
            .request
            .header_editor
            .entries
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
    }

    pub fn parse_params_from_url(&mut self) {
        self.request.param_editor.entries.clear();
        let Some(query_start) = self.request.url.find('?') else {
            self.request.param_editor.selected = 0;
            return;
        };
        let query = &self.request.url[query_start + 1..];
        let query = query.split('#').next().unwrap_or(query);
        for pair in query.split('&') {
            if pair.is_empty() {
                continue;
            }
            let (key, value) = match pair.split_once('=') {
                Some((k, v)) => (
                    url_utils::simple_url_decode(k),
                    url_utils::simple_url_decode(v),
                ),
                None => (url_utils::simple_url_decode(pair), String::new()),
            };
            self.request.param_editor.entries.push((key, value));
        }
        self.request.param_editor.selected = 0;
    }

    pub fn sync_params_to_url(&mut self) {
        let base = self
            .request
            .url
            .split('?')
            .next()
            .unwrap_or(&self.request.url)
            .to_owned();
        if self.request.param_editor.entries.is_empty() {
            self.request.url = base;
        } else {
            let query: String = self
                .request
                .param_editor
                .entries
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}={}",
                        url_utils::simple_url_encode(k),
                        url_utils::simple_url_encode(v)
                    )
                })
                .collect::<Vec<_>>()
                .join("&");
            self.request.url = format!("{base}?{query}");
        }
        self.request.cursor_pos = self.request.url.len();
        self.sync_to_collection();
    }

    // -- Sidebar editing --

    pub fn start_editing_name(&mut self) {
        match self.selected_sidebar_item() {
            Some(SidebarItem::Request(req)) => {
                self.sidebar.edit_buffer = req.name;
                self.sidebar.editing_name = true;
            }
            Some(SidebarItem::Folder(f)) => {
                self.sidebar.edit_buffer = f.name;
                self.sidebar.editing_name = true;
            }
            Some(SidebarItem::NewRequest) | None => {}
        }
    }

    pub fn confirm_editing_name(&mut self) {
        match self.selected_sidebar_item() {
            Some(SidebarItem::Request(req)) => {
                if let Some(r) = self.sidebar.requests.iter_mut().find(|r| r.id == req.id) {
                    r.name.clone_from(&self.sidebar.edit_buffer);
                }
            }
            Some(SidebarItem::Folder(f)) => {
                if let Some(fo) = self.sidebar.folders.iter_mut().find(|fo| fo.id == f.id) {
                    fo.name.clone_from(&self.sidebar.edit_buffer);
                }
            }
            Some(SidebarItem::NewRequest) | None => {}
        }
        self.sidebar.editing_name = false;
        self.save_collections();
    }

    pub fn cycle_sidebar_method(&mut self) {
        if let Some(SidebarItem::Request(req)) = self.selected_sidebar_item() {
            if let Some(r) = self.sidebar.requests.iter_mut().find(|r| r.id == req.id) {
                let m = Method::from_str(&r.method).next();
                m.as_str().clone_into(&mut r.method);
                if self.sidebar.active_request_id.as_deref() == Some(&req.id) {
                    self.request.method = Method::from_str(&r.method);
                }
            }
            self.save_collections();
        }
    }

    // -- CRUD --

    pub fn create_request(&mut self) {
        let folder_id = match self.selected_sidebar_item() {
            Some(SidebarItem::Folder(f)) => Some(f.id),
            Some(SidebarItem::Request(r)) => r.folder_id,
            Some(SidebarItem::NewRequest) | None => None,
        };
        self.create_request_in(folder_id);
    }

    pub fn create_request_at_root(&mut self) {
        self.create_request_in(None);
    }

    fn create_request_in(&mut self, folder_id: Option<String>) {
        let req = SavedRequest {
            id: collections::new_id(),
            name: "New Request".into(),
            method: "GET".into(),
            url: "https://".into(),
            body: String::new(),
            headers: HashMap::new(),
            folder_id,
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
        };

        let id = req.id.clone();
        self.sidebar.requests.push(req);
        self.save_collections();
        self.load_request_by_id(&id);

        let items = self.sidebar_items();
        if let Some(idx) = items
            .iter()
            .position(|item| matches!(item, SidebarItem::Request(r) if r.id == id))
        {
            self.sidebar.selected = idx;
        }

        self.ui.focus = Focus::UrlBar;
    }

    pub fn create_folder(&mut self) {
        let folder = Folder {
            id: collections::new_id(),
            name: "New Folder".into(),
            expanded: true,
        };
        self.sidebar.folders.push(folder);
        self.save_collections();

        let items = self.sidebar_items();
        self.sidebar.selected = items.len().saturating_sub(1);
    }

    pub fn duplicate_request(&mut self) {
        if let Some(SidebarItem::Request(req)) = self.selected_sidebar_item() {
            let new_req = SavedRequest {
                id: collections::new_id(),
                name: format!("{} (copy)", req.name),
                method: req.method.clone(),
                url: req.url.clone(),
                body: req.body.clone(),
                headers: req.headers.clone(),
                folder_id: req.folder_id,
                auth: req.auth,
                body_type: req.body_type,
                content_type: req.content_type,
                form_data: req.form_data,
                follow_redirects: req.follow_redirects,
                timeout_secs: req.timeout_secs,
                verify_tls: req.verify_tls,
                ca_cert_path: req.ca_cert_path,
                client_cert_path: req.client_cert_path,
                client_key_path: req.client_key_path,
                tls_min_version: req.tls_min_version,
                extractors: req.extractors,
                assertions: req.assertions,
                last_response: req.last_response,
                last_error: req.last_error,
                last_assertion_results: req.last_assertion_results,
            };
            let id = new_req.id.clone();
            self.sidebar.requests.push(new_req);
            self.save_collections();
            self.load_request_by_id(&id);

            let items = self.sidebar_items();
            if let Some(idx) = items
                .iter()
                .position(|item| matches!(item, SidebarItem::Request(r) if r.id == id))
            {
                self.sidebar.selected = idx;
            }
        }
    }

    pub fn request_delete(&mut self) {
        if self.sidebar.confirm_delete {
            self.confirm_delete_action();
        } else {
            self.sidebar.confirm_delete = true;
        }
    }

    pub const fn cancel_delete(&mut self) {
        self.sidebar.confirm_delete = false;
    }

    fn confirm_delete_action(&mut self) {
        self.sidebar.confirm_delete = false;
        match self.selected_sidebar_item() {
            Some(SidebarItem::Request(req)) => {
                self.sidebar.requests.retain(|r| r.id != req.id);
                if self.sidebar.active_request_id.as_deref() == Some(&req.id) {
                    self.sidebar.active_request_id = None;
                    self.request.url.clear();
                    self.request.body.clear();
                    self.response.last = None;
                }
            }
            Some(SidebarItem::Folder(f)) => {
                self.sidebar
                    .requests
                    .retain(|r| r.folder_id.as_deref() != Some(&f.id));
                self.sidebar.folders.retain(|fo| fo.id != f.id);
                self.sidebar.active_request_id = None;
                self.request.url.clear();
                self.request.body.clear();
                self.response.last = None;
            }
            Some(SidebarItem::NewRequest) | None => return,
        }

        let max = self.sidebar_items().len().saturating_sub(1);
        if self.sidebar.selected > max {
            self.sidebar.selected = max;
        }

        self.save_collections();
    }

    // -- URL cursor editing --

    pub fn url_insert(&mut self, c: char) {
        self.request.url.insert(self.request.cursor_pos, c);
        self.request.cursor_pos += c.len_utf8();
    }

    pub fn url_backspace(&mut self) {
        if self.request.cursor_pos > 0 {
            let prev = self.request.url[..self.request.cursor_pos]
                .char_indices()
                .next_back()
                .map_or(0, |(i, _)| i);
            self.request.url.remove(prev);
            self.request.cursor_pos = prev;
        }
    }

    pub fn url_delete(&mut self) {
        if self.request.cursor_pos < self.request.url.len() {
            self.request.url.remove(self.request.cursor_pos);
        }
    }

    pub fn url_cursor_left(&mut self) {
        if self.request.cursor_pos > 0 {
            self.request.cursor_pos = self.request.url[..self.request.cursor_pos]
                .char_indices()
                .next_back()
                .map_or(0, |(i, _)| i);
        }
    }

    pub fn url_cursor_right(&mut self) {
        if self.request.cursor_pos < self.request.url.len() {
            self.request.cursor_pos += self.request.url[self.request.cursor_pos..]
                .chars()
                .next()
                .map_or(0, char::len_utf8);
        }
    }

    pub const fn url_cursor_home(&mut self) {
        self.request.cursor_pos = 0;
    }

    pub fn url_cursor_end(&mut self) {
        self.request.cursor_pos = self.request.url.len();
    }

    pub fn finish_url_edit(&mut self) {
        self.request.editing_url = false;
        self.parse_params_from_url();
        self.sync_to_collection();
    }

    // -- Body cursor editing --

    fn body_line_count(&self) -> usize {
        self.request.body.split('\n').count().max(1)
    }

    fn body_line_len(&self, row: usize) -> usize {
        self.request.body.split('\n').nth(row).map_or(0, str::len)
    }

    fn body_cursor_offset(&self) -> usize {
        let mut offset = 0;
        for (i, line) in self.request.body.split('\n').enumerate() {
            if i == self.request.body_row {
                return offset + self.request.body_col.min(line.len());
            }
            offset += line.len() + 1;
        }
        self.request.body.len()
    }

    pub fn body_insert(&mut self, c: char) {
        let offset = self.body_cursor_offset();
        self.request.body.insert(offset, c);
        self.request.body_col += c.len_utf8();
    }

    pub fn body_insert_newline(&mut self) {
        let offset = self.body_cursor_offset();
        self.request.body.insert(offset, '\n');
        self.request.body_row += 1;
        self.request.body_col = 0;
    }

    pub fn body_insert_tab(&mut self) {
        let offset = self.body_cursor_offset();
        self.request.body.insert_str(offset, "  ");
        self.request.body_col += 2;
    }

    pub fn body_backspace(&mut self) {
        if self.request.body_col > 0 {
            let offset = self.body_cursor_offset();
            self.request.body.remove(offset - 1);
            self.request.body_col -= 1;
        } else if self.request.body_row > 0 {
            let prev_len = self.body_line_len(self.request.body_row - 1);
            let offset = self.body_cursor_offset();
            self.request.body.remove(offset - 1);
            self.request.body_row -= 1;
            self.request.body_col = prev_len;
        }
    }

    pub fn body_delete(&mut self) {
        let offset = self.body_cursor_offset();
        if offset < self.request.body.len() {
            self.request.body.remove(offset);
        }
    }

    pub fn body_cursor_left(&mut self) {
        if self.request.body_col > 0 {
            self.request.body_col -= 1;
        } else if self.request.body_row > 0 {
            self.request.body_row -= 1;
            self.request.body_col = self.body_line_len(self.request.body_row);
        }
    }

    pub fn body_cursor_right(&mut self) {
        let line_len = self.body_line_len(self.request.body_row);
        if self.request.body_col < line_len {
            self.request.body_col += 1;
        } else if self.request.body_row + 1 < self.body_line_count() {
            self.request.body_row += 1;
            self.request.body_col = 0;
        }
    }

    pub fn body_cursor_up(&mut self) {
        if self.request.body_row > 0 {
            self.request.body_row -= 1;
            self.request.body_col = self
                .request
                .body_col
                .min(self.body_line_len(self.request.body_row));
        }
    }

    pub fn body_cursor_down(&mut self) {
        if self.request.body_row + 1 < self.body_line_count() {
            self.request.body_row += 1;
            self.request.body_col = self
                .request
                .body_col
                .min(self.body_line_len(self.request.body_row));
        }
    }

    pub const fn body_cursor_home(&mut self) {
        self.request.body_col = 0;
    }

    pub fn body_cursor_end(&mut self) {
        self.request.body_col = self.body_line_len(self.request.body_row);
    }

    pub fn enter_body_edit(&mut self) {
        self.request.editing_body = true;
        let count = self.body_line_count();
        self.request.body_row = count.saturating_sub(1);
        self.request.body_col = self.body_line_len(self.request.body_row);
    }

    pub fn finish_body_edit(&mut self) {
        self.request.editing_body = false;
        self.sync_to_collection();
    }

    // -- cURL import --

    pub fn open_curl_import(&mut self) {
        self.curl_io.import_buffer = String::new();
        self.curl_io.import_error = false;
        self.curl_io.import_open = true;
    }

    pub fn confirm_curl_import(&mut self) {
        let Some(parsed) = curl::parse(&self.curl_io.import_buffer) else {
            self.curl_io.import_error = true;
            return;
        };

        let folder_id = match self.selected_sidebar_item() {
            Some(SidebarItem::Folder(f)) => Some(f.id),
            Some(SidebarItem::Request(r)) => r.folder_id,
            Some(SidebarItem::NewRequest) | None => None,
        };

        let name = url_utils::name_from_url(&parsed.url);
        let req = SavedRequest {
            id: collections::new_id(),
            name,
            method: parsed.method,
            url: parsed.url,
            body: parsed.body,
            headers: parsed.headers,
            folder_id,
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
        };

        let id = req.id.clone();
        self.sidebar.requests.push(req);
        self.save_collections();
        self.load_request_by_id(&id);

        let items = self.sidebar_items();
        if let Some(idx) = items
            .iter()
            .position(|item| matches!(item, SidebarItem::Request(r) if r.id == id))
        {
            self.sidebar.selected = idx;
        }

        self.curl_io.import_open = false;
        self.ui.focus = Focus::UrlBar;
    }

    pub fn open_curl_export(&mut self) {
        self.curl_io.export_content = curl::export(
            self.request.method.as_str(),
            &self.request.url,
            &self.request.headers,
            &self.request.body,
        );
        self.curl_io.export_open = true;
    }

    // -- Postman import --

    pub fn open_postman_import(&mut self) {
        self.postman_io.buffer = String::new();
        self.postman_io.error = false;
        self.postman_io.open = true;
    }

    pub fn confirm_postman_import(&mut self) {
        let path = std::path::Path::new(self.postman_io.buffer.trim());
        let Some(result) = postman::import(path) else {
            self.postman_io.error = true;
            return;
        };

        self.sidebar.folders.extend(result.folders);
        self.sidebar.requests.extend(result.requests);
        self.save_collections();

        self.postman_io.open = false;
    }

    // -- History --

    pub fn open_history(&mut self) {
        self.history.entries = history::load();
        self.history.selected = 0;
        self.history.open = true;
    }

    pub fn load_from_history(&mut self) {
        if let Some(entry) = self.history.entries.iter().rev().nth(self.history.selected) {
            self.request.method = Method::from_str(&entry.method);
            self.request.url = entry.url.clone();
            self.request.body = entry.body.clone();
            self.request.cursor_pos = self.request.url.len();
            self.request.body_row = 0;
            self.request.body_col = 0;
            self.response.last = None;
            self.history.open = false;
        }
    }

    // -- Auth --

    pub const AUTH_TYPES: &'static [&'static str] = &["None", "Bearer", "Basic", "API Key"];

    pub const fn auth_type_index(&self) -> usize {
        match &self.auth.config {
            Auth::None => 0,
            Auth::Bearer { .. } => 1,
            Auth::Basic { .. } => 2,
            Auth::ApiKey { .. } => 3,
        }
    }

    pub fn select_auth_type(&mut self) {
        self.auth.selecting_type = false;
        let new_auth = match self.auth.type_selected {
            1 => {
                let token = if let Auth::Bearer { token } = &self.auth.config {
                    token.clone()
                } else {
                    String::new()
                };
                Auth::Bearer { token }
            }
            2 => {
                let (username, password) =
                    if let Auth::Basic { username, password } = &self.auth.config {
                        (username.clone(), password.clone())
                    } else {
                        (String::new(), String::new())
                    };
                Auth::Basic { username, password }
            }
            3 => {
                let (header, value) = if let Auth::ApiKey { header, value } = &self.auth.config {
                    (header.clone(), value.clone())
                } else {
                    ("X-API-Key".to_owned(), String::new())
                };
                Auth::ApiKey { header, value }
            }
            _ => Auth::None,
        };
        self.auth.config = new_auth;
        self.sync_to_collection();

        if self.auth.config != Auth::None {
            self.open_auth_edit();
        }
    }

    pub fn open_auth_edit(&mut self) {
        self.auth.field = 0;
        match &self.auth.config {
            Auth::Bearer { token } => {
                self.auth.buf_a = token.clone();
                self.auth.buf_b.clear();
            }
            Auth::Basic { username, password } => {
                self.auth.buf_a = username.clone();
                self.auth.buf_b = password.clone();
            }
            Auth::ApiKey { header, value } => {
                self.auth.buf_a = header.clone();
                self.auth.buf_b = value.clone();
            }
            Auth::None => return,
        }
        self.auth.editing = true;
    }

    pub fn confirm_auth_edit(&mut self) {
        match &self.auth.config {
            Auth::Bearer { .. } => {
                self.auth.config = Auth::Bearer {
                    token: self.auth.buf_a.clone(),
                };
            }
            Auth::Basic { .. } => {
                self.auth.config = Auth::Basic {
                    username: self.auth.buf_a.clone(),
                    password: self.auth.buf_b.clone(),
                };
            }
            Auth::ApiKey { .. } => {
                self.auth.config = Auth::ApiKey {
                    header: self.auth.buf_a.clone(),
                    value: self.auth.buf_b.clone(),
                };
            }
            Auth::None => {}
        }
        self.auth.editing = false;
        self.sync_to_collection();
    }

    fn apply_auth_headers(&self, headers: &mut HashMap<String, String>) {
        match &self.auth.config {
            Auth::None => {}
            Auth::Bearer { token } => {
                let resolved = self.resolve_variables(token);
                headers.insert("Authorization".to_owned(), format!("Bearer {resolved}"));
            }
            Auth::Basic { username, password } => {
                let resolved_user = self.resolve_variables(username);
                let resolved_pass = self.resolve_variables(password);
                let encoded =
                    crate::curl::base64(format!("{resolved_user}:{resolved_pass}").as_bytes());
                headers.insert("Authorization".to_owned(), format!("Basic {encoded}"));
            }
            Auth::ApiKey { header, value } => {
                let resolved_header = self.resolve_variables(header);
                let resolved_value = self.resolve_variables(value);
                headers.insert(resolved_header, resolved_value);
            }
        }
    }

    // -- Environments --

    pub fn save_environments(&self) {
        let data = environments::EnvironmentData {
            environments: self.env.environments.clone(),
            active_id: self.env.active_id.clone(),
        };
        environments::save(&data);
    }

    pub fn active_env_name(&self) -> Option<&str> {
        let id = self.env.active_id.as_ref()?;
        self.env
            .environments
            .iter()
            .find(|e| e.id == *id)
            .map(|e| e.name.as_str())
    }

    pub const fn open_env_popup(&mut self) {
        self.env.popup_selected = 0;
        self.env.popup_open = true;
    }

    pub fn env_popup_count(&self) -> usize {
        self.env.environments.len() + 1
    }

    pub fn select_env_from_popup(&mut self) {
        if self.env.popup_selected == 0 {
            self.env.active_id = None;
        } else {
            let idx = self.env.popup_selected - 1;
            if let Some(env) = self.env.environments.get(idx) {
                self.env.active_id = Some(env.id.clone());
            }
        }
        self.save_environments();
        self.env.popup_open = false;
    }

    pub fn create_environment(&mut self) {
        let env = Environment {
            id: collections::new_id(),
            name: String::new(),
            variables: Vec::new(),
        };
        self.env.environments.push(env);
        self.env.popup_selected = self.env.environments.len();
        self.env.name_buffer = String::new();
        self.env.renaming = true;
        self.save_environments();
    }

    pub fn delete_env_from_popup(&mut self) {
        if self.env.popup_selected == 0 {
            return;
        }
        let idx = self.env.popup_selected - 1;
        if idx < self.env.environments.len() {
            let removed_id = self.env.environments[idx].id.clone();
            self.env.environments.remove(idx);
            if self.env.active_id.as_deref() == Some(&removed_id) {
                self.env.active_id = None;
            }
            let max = self.env_popup_count().saturating_sub(1);
            if self.env.popup_selected > max {
                self.env.popup_selected = max;
            }
            self.save_environments();
        }
    }

    pub fn start_env_rename(&mut self) {
        if self.env.popup_selected == 0 {
            return;
        }
        let idx = self.env.popup_selected - 1;
        if let Some(env) = self.env.environments.get(idx) {
            self.env.name_buffer = env.name.clone();
            self.env.renaming = true;
        }
    }

    pub fn confirm_env_rename(&mut self) {
        if self.env.popup_selected == 0 {
            self.env.renaming = false;
            return;
        }
        let idx = self.env.popup_selected - 1;
        if self.env.name_buffer.trim().is_empty() {
            self.cancel_env_rename();
            return;
        }
        if let Some(env) = self.env.environments.get_mut(idx) {
            env.name.clone_from(&self.env.name_buffer);
        }
        self.env.renaming = false;
        self.save_environments();
    }

    pub fn cancel_env_rename(&mut self) {
        if self.env.popup_selected > 0 {
            let idx = self.env.popup_selected - 1;
            if idx < self.env.environments.len() && self.env.environments[idx].name.is_empty() {
                self.env.environments.remove(idx);
                let max = self.env_popup_count().saturating_sub(1);
                if self.env.popup_selected > max {
                    self.env.popup_selected = max;
                }
                self.save_environments();
            }
        }
        self.env.renaming = false;
    }

    pub fn open_env_import(&mut self) {
        self.env.import.buffer = String::new();
        self.env.import.error = false;
        self.env.import.open = true;
    }

    pub fn confirm_env_import(&mut self) {
        let path = std::path::Path::new(self.env.import.buffer.trim());
        let Some(vars) = environments::parse_dotenv(path) else {
            self.env.import.error = true;
            return;
        };

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("dotenv")
            .to_owned();

        let env = Environment {
            id: collections::new_id(),
            name,
            variables: vars,
        };
        self.env.environments.push(env);
        self.env.popup_selected = self.env.environments.len();
        self.save_environments();
        self.env.import.open = false;
    }

    pub fn open_env_editor(&mut self) {
        if self.env.popup_selected == 0 {
            return;
        }
        let idx = self.env.popup_selected - 1;
        if let Some(env) = self.env.environments.get(idx) {
            self.env.editor.id = env.id.clone();
            self.env.editor.selected = 0;
            self.env.popup_open = false;
            self.env.editor.open = true;
        }
    }

    fn edited_env(&self) -> Option<&Environment> {
        self.env
            .environments
            .iter()
            .find(|e| e.id == self.env.editor.id)
    }

    pub fn env_editor_count(&self) -> usize {
        self.edited_env().map_or(1, |e| e.variables.len() + 1)
    }

    pub fn start_add_var(&mut self) {
        self.env.editor.var_key_buffer.clear();
        self.env.editor.var_value_buffer.clear();
        self.env.editor.var_field = 0;
        self.env.editor.editing_var = true;
    }

    pub fn start_edit_var(&mut self) {
        let id = self.env.editor.id.clone();
        let Some(env) = self.env.environments.iter().find(|e| e.id == id) else {
            return;
        };
        if self.env.editor.selected >= env.variables.len() {
            self.start_add_var();
            return;
        }
        self.env.editor.var_key_buffer = env.variables[self.env.editor.selected].key.clone();
        self.env.editor.var_value_buffer = env.variables[self.env.editor.selected].value.clone();
        self.env.editor.var_field = 0;
        self.env.editor.editing_var = true;
    }

    pub fn confirm_var_edit(&mut self) {
        if self.env.editor.var_key_buffer.trim().is_empty() {
            self.env.editor.editing_var = false;
            return;
        }
        let id = self.env.editor.id.clone();
        let Some(env) = self.env.environments.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if self.env.editor.selected < env.variables.len() {
            let var = &mut env.variables[self.env.editor.selected];
            var.key.clone_from(&self.env.editor.var_key_buffer);
            var.value.clone_from(&self.env.editor.var_value_buffer);
        } else {
            env.variables.push(Variable {
                key: self.env.editor.var_key_buffer.clone(),
                value: self.env.editor.var_value_buffer.clone(),
                secret: false,
            });
        }
        self.env.editor.editing_var = false;
        self.save_environments();
    }

    pub fn delete_var(&mut self) {
        let id = self.env.editor.id.clone();
        let Some(env) = self.env.environments.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if self.env.editor.selected < env.variables.len() {
            env.variables.remove(self.env.editor.selected);
            let max = env.variables.len();
            if self.env.editor.selected > max {
                self.env.editor.selected = max;
            }
            self.save_environments();
        }
    }

    pub fn toggle_var_secret(&mut self) {
        let id = self.env.editor.id.clone();
        let Some(env) = self.env.environments.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if self.env.editor.selected < env.variables.len() {
            let var = &mut env.variables[self.env.editor.selected];
            var.secret = !var.secret;
            self.save_environments();
        }
    }

    fn resolve_variables(&self, input: &str) -> String {
        let mut result = self.resolve_chain_refs(input);
        for (k, v) in &self.extractors.extracted {
            let pattern = format!("{{{{{k}}}}}");
            result = result.replace(&pattern, v);
        }
        let Some(env_id) = &self.env.active_id else {
            return result;
        };
        let Some(env) = self.env.environments.iter().find(|e| e.id == *env_id) else {
            return result;
        };
        for var in &env.variables {
            let pattern = format!("{{{{{}}}}}", var.key);
            result = result.replace(&pattern, &var.value);
        }
        result
    }

    /// Expands `{{$res:RequestName.path.to.field}}` markers using the bodies
    /// of previously sent requests stored in `self.response.last_bodies`.
    fn resolve_chain_refs(&self, input: &str) -> String {
        const OPEN: &str = "{{$res:";
        const CLOSE: &str = "}}";
        let mut out = String::with_capacity(input.len());
        let mut rest = input;
        while let Some(start) = rest.find(OPEN) {
            out.push_str(&rest[..start]);
            let after = &rest[start + OPEN.len()..];
            let Some(end) = after.find(CLOSE) else {
                out.push_str(&rest[start..]);
                return out;
            };
            let expr = &after[..end];
            out.push_str(&self.lookup_chain_expr(expr));
            rest = &after[end + CLOSE.len()..];
        }
        out.push_str(rest);
        out
    }

    fn lookup_chain_expr(&self, expr: &str) -> String {
        let (name, path) = expr
            .split_once('.')
            .map_or((expr, ""), |(n, p)| (n.trim(), p.trim()));
        let Some(body) = self.response.last_bodies.get(name) else {
            return format!("{{{{$res:{expr}}}}}");
        };
        if path.is_empty() {
            return body.clone();
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
            return format!("{{{{$res:{expr}}}}}");
        };
        let mut current = &value;
        for segment in path.split('.') {
            let next = segment
                .parse::<usize>()
                .map_or_else(|_| current.get(segment), |idx| current.get(idx));
            let Some(next) = next else {
                return format!("{{{{$res:{expr}}}}}");
            };
            current = next;
        }
        match current {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        }
    }

    // -- Search --

    pub fn open_search(&mut self) {
        self.response.search_buf = self.response.search.clone();
        self.response.searching = true;
    }

    pub fn confirm_search(&mut self) {
        self.response.search = self.response.search_buf.clone();
        self.response.searching = false;
        self.response.match_idx = 0;
        self.scroll_to_match();
    }

    pub const fn cancel_search(&mut self) {
        self.response.searching = false;
    }

    pub fn clear_search(&mut self) {
        self.response.search.clear();
        self.response.searching = false;
    }

    pub fn next_match(&mut self) {
        if self.response.search.is_empty() {
            return;
        }
        self.response.match_idx += 1;
        self.scroll_to_match();
    }

    pub fn prev_match(&mut self) {
        if self.response.search.is_empty() {
            return;
        }
        self.response.match_idx = self.response.match_idx.saturating_sub(1);
        self.scroll_to_match();
    }

    fn scroll_to_match(&mut self) {
        let body = self.formatted_response_body();
        if self.response.search.is_empty() {
            return;
        }
        let needle = self.response.search.to_ascii_lowercase();
        let mut match_count = 0;
        for (i, line) in body.lines().enumerate() {
            if line.to_ascii_lowercase().contains(&needle) {
                if match_count == self.response.match_idx {
                    self.response.scroll = i as u16;
                    return;
                }
                match_count += 1;
            }
        }
        if match_count > 0 {
            self.response.match_idx = 0;
            self.scroll_to_match();
        }
    }

    fn resolve_form_entries(&self) -> Vec<(String, String)> {
        self.request
            .form_editor
            .entries
            .iter()
            .map(|(k, v)| (self.resolve_variables(k), self.resolve_variables(v)))
            .collect()
    }

    // -- Request --

    pub fn open_timeout_popup(&mut self) {
        self.timeout.buffer = self.timeout.secs.to_string();
        self.timeout.error = false;
        self.timeout.popup_open = true;
    }

    pub fn confirm_timeout_popup(&mut self) {
        match self.timeout.buffer.trim().parse::<u64>() {
            Ok(n) if n > 0 && n <= 3600 => {
                self.timeout.secs = n;
                self.timeout.popup_open = false;
                self.sync_to_collection();
            }
            _ => self.timeout.error = true,
        }
    }

    pub const TLS_FIELDS: usize = 5;

    pub fn open_tls_popup(&mut self) {
        self.tls.popup_selected = 0;
        self.tls.editing = false;
        self.tls.edit_buffer.clear();
        self.tls.popup_open = true;
    }

    pub const fn tls_popup_down(&mut self) {
        if self.tls.popup_selected + 1 < Self::TLS_FIELDS {
            self.tls.popup_selected += 1;
        }
    }

    pub const fn tls_popup_up(&mut self) {
        if self.tls.popup_selected > 0 {
            self.tls.popup_selected -= 1;
        }
    }

    pub fn tls_popup_activate(&mut self) {
        match self.tls.popup_selected {
            0 => {
                self.tls.verify = !self.tls.verify;
                self.sync_to_collection();
            }
            4 => {
                self.tls.min_version = match self.tls.min_version.as_str() {
                    "" => "1.2".to_owned(),
                    "1.2" => "1.3".to_owned(),
                    _ => String::new(),
                };
                self.sync_to_collection();
            }
            n => {
                self.tls.edit_buffer = match n {
                    1 => self.tls.ca_cert.clone(),
                    2 => self.tls.client_cert.clone(),
                    3 => self.tls.client_key.clone(),
                    _ => String::new(),
                };
                self.tls.editing = true;
            }
        }
    }

    pub fn tls_popup_clear_field(&mut self) {
        match self.tls.popup_selected {
            1 => self.tls.ca_cert.clear(),
            2 => self.tls.client_cert.clear(),
            3 => self.tls.client_key.clear(),
            _ => return,
        }
        self.sync_to_collection();
    }

    pub fn tls_confirm_edit(&mut self) {
        let val = self.tls.edit_buffer.trim().to_owned();
        match self.tls.popup_selected {
            1 => self.tls.ca_cert = val,
            2 => self.tls.client_cert = val,
            3 => self.tls.client_key = val,
            _ => {}
        }
        self.tls.editing = false;
        self.sync_to_collection();
    }

    pub const fn open_assertions_popup(&mut self) {
        self.assertions.popup_open = true;
        self.assertions.editing = false;
        self.assertions.selected = 0;
    }

    pub fn assertions_popup_down(&mut self) {
        let max = self.assertions.exprs.len();
        if self.assertions.selected < max {
            self.assertions.selected += 1;
        }
    }

    pub const fn assertions_popup_up(&mut self) {
        if self.assertions.selected > 0 {
            self.assertions.selected -= 1;
        }
    }

    pub fn assertions_start_add(&mut self) {
        self.assertions.edit_buffer.clear();
        self.assertions.editing_existing = false;
        self.assertions.editing = true;
    }

    pub fn assertions_start_edit(&mut self) {
        if self.assertions.selected >= self.assertions.exprs.len() {
            self.assertions_start_add();
            return;
        }
        self.assertions
            .edit_buffer
            .clone_from(&self.assertions.exprs[self.assertions.selected]);
        self.assertions.editing_existing = true;
        self.assertions.editing = true;
    }

    pub fn assertions_confirm_edit(&mut self) {
        let value = self.assertions.edit_buffer.trim().to_owned();
        if value.is_empty() {
            self.assertions.editing = false;
            return;
        }
        if self.assertions.editing_existing
            && self.assertions.selected < self.assertions.exprs.len()
        {
            self.assertions.exprs[self.assertions.selected] = value;
        } else {
            self.assertions.exprs.push(value);
            self.assertions.selected = self.assertions.exprs.len() - 1;
        }
        self.assertions.editing = false;
        self.sync_to_collection();
    }

    pub fn assertions_delete(&mut self) {
        if self.assertions.selected < self.assertions.exprs.len() {
            self.assertions.exprs.remove(self.assertions.selected);
            if self.assertions.selected > 0
                && self.assertions.selected >= self.assertions.exprs.len()
            {
                self.assertions.selected = self.assertions.exprs.len().saturating_sub(1);
            }
            self.sync_to_collection();
        }
    }

    pub const fn open_extractors_popup(&mut self) {
        self.extractors.editor.editing = false;
        self.extractors.editor.selected = 0;
        self.extractors.popup_open = true;
    }

    fn apply_extractors(&mut self, body: &str) {
        if self.extractors.editor.entries.is_empty() {
            return;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
            return;
        };
        for (name, path) in &self.extractors.editor.entries {
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            if let Some(extracted) = jsonpath_lookup(&value, path.trim()) {
                self.extractors.extracted.insert(name.to_owned(), extracted);
            }
        }
    }

    pub fn open_cookies_popup(&mut self) {
        self.cookies.store.purge_expired();
        self.cookies.popup_selected = 0;
        self.cookies.popup_open = true;
    }

    pub fn cookies_popup_down(&mut self) {
        let len = self.cookies.store.cookies.len();
        if len > 0 && self.cookies.popup_selected + 1 < len {
            self.cookies.popup_selected += 1;
        }
    }

    pub const fn cookies_popup_up(&mut self) {
        if self.cookies.popup_selected > 0 {
            self.cookies.popup_selected -= 1;
        }
    }

    pub fn cookies_popup_delete(&mut self) {
        if self.cookies.store.cookies.is_empty() {
            return;
        }
        self.cookies.store.remove(self.cookies.popup_selected);
        let len = self.cookies.store.cookies.len();
        if self.cookies.popup_selected >= len && len > 0 {
            self.cookies.popup_selected = len - 1;
        }
        cookies::save(&self.cookies.store);
    }

    pub fn cookies_popup_clear_all(&mut self) {
        self.cookies.store.clear();
        self.cookies.popup_selected = 0;
        cookies::save(&self.cookies.store);
    }

    pub fn toggle_follow_redirects(&mut self) {
        self.follow_redirects = !self.follow_redirects;
        self.sync_to_collection();
    }

    pub fn send_request(&mut self) {
        if self.pending.is_some() {
            return;
        }
        self.response.loading = true;
        self.response.last = None;
        self.response.scroll = 0;
        self.sync_to_collection();

        let resolved_url = self.resolve_variables(&self.request.url);
        let resolved_body = self.resolve_variables(&self.request.body);
        let mut resolved_headers: HashMap<String, String> = self
            .request
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), self.resolve_variables(v)))
            .collect();
        self.apply_auth_headers(&mut resolved_headers);

        let has_cookie_header = resolved_headers
            .keys()
            .any(|k| k.eq_ignore_ascii_case("cookie"));
        if !has_cookie_header {
            if let Some(value) = self.cookies.store.header_for(&resolved_url) {
                resolved_headers.insert("Cookie".to_owned(), value);
            }
        }

        if !resolved_headers.contains_key("Content-Type")
            && !resolved_headers.contains_key("content-type")
        {
            let ct = match self.request.body_type {
                BodyType::Raw => Some(self.request.content_type.mime()),
                BodyType::Form => Some("application/x-www-form-urlencoded"),
                BodyType::Multipart => None,
            };
            if let Some(ct) = ct {
                resolved_headers.insert("Content-Type".to_owned(), ct.to_owned());
            }
        }

        let body = match self.request.body_type {
            BodyType::Raw => {
                if resolved_body.is_empty() {
                    None
                } else {
                    Some(frogbite::core::http::RequestBody::Raw(resolved_body))
                }
            }
            BodyType::Form => {
                let pairs = self.resolve_form_entries();
                if pairs.is_empty() {
                    None
                } else {
                    Some(frogbite::core::http::RequestBody::Form(pairs))
                }
            }
            BodyType::Multipart => {
                let pairs = self.resolve_form_entries();
                if pairs.is_empty() {
                    None
                } else {
                    Some(frogbite::core::http::RequestBody::Multipart(pairs))
                }
            }
        };

        let opts = RequestOptions {
            method: self.request.method.as_str().to_owned(),
            url: resolved_url,
            headers: resolved_headers,
            body,
            follow_redirects: self.follow_redirects,
            timeout_secs: self.timeout.secs,
            verify_tls: self.tls.verify,
            ca_cert_path: self.tls.ca_cert.clone(),
            client_cert_path: self.tls.client_cert.clone(),
            client_key_path: self.tls.client_key.clone(),
            tls_min_version: self.tls.min_version.clone(),
        };

        let resolved_for_pending = opts.url.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = frogbite::core::http::send_request(&opts);
            let _ = tx.send(result);
        });

        let request_name = self.sidebar.active_request_id.as_ref().and_then(|id| {
            self.sidebar
                .requests
                .iter()
                .find(|r| &r.id == id)
                .map(|r| r.name.clone())
        });
        self.pending = Some(PendingRequest {
            rx,
            method: self.request.method.as_str().to_owned(),
            url: self.request.url.clone(),
            resolved_url: resolved_for_pending,
            body: self.request.body.clone(),
            request_name,
        });
    }

    fn persist_last_response(&mut self) {
        let Some(id) = self.sidebar.active_request_id.clone() else {
            return;
        };
        let (last_response, last_error) = match &self.response.last {
            Some(Ok(r)) => (Some(r.clone()), None),
            Some(Err(e)) => (None, Some(e.clone())),
            None => (None, None),
        };
        let last_assertion_results = self.assertions.results.clone();
        if let Some(req) = self.sidebar.requests.iter_mut().find(|r| r.id == id) {
            req.last_response = last_response;
            req.last_error = last_error;
            req.last_assertion_results = last_assertion_results;
        }
        self.save_collections();
    }

    pub fn poll_pending(&mut self) -> bool {
        let Some(pending) = self.pending.as_ref() else {
            return false;
        };
        let recv = pending.rx.try_recv();
        match recv {
            Ok(result) => {
                let Some(pending) = self.pending.take() else {
                    return false;
                };
                if let Ok(resp) = &result {
                    if !resp.set_cookies.is_empty() {
                        self.cookies
                            .store
                            .ingest(&pending.resolved_url, &resp.set_cookies);
                        self.cookies.store.purge_expired();
                        cookies::save(&self.cookies.store);
                    }
                    if let Some(name) = &pending.request_name {
                        self.response
                            .last_bodies
                            .insert(name.clone(), resp.body.clone());
                    }
                    let body = resp.body.clone();
                    self.apply_extractors(&body);
                    self.assertions.results =
                        assertions::evaluate_all(&self.assertions.exprs, resp);
                }
                if result.is_err() {
                    self.assertions.results.clear();
                }
                let entry = HistoryEntry {
                    method: pending.method.clone(),
                    url: pending.url.clone(),
                    body: pending.body.clone(),
                    status: result.as_ref().ok().map(|r| r.status),
                    duration_ms: result.as_ref().ok().map(|r| r.duration_ms),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |d| d.as_secs()),
                };
                history::append(entry);
                self.response.last = Some(result);
                self.response.loading = false;
                self.persist_last_response();
                true
            }
            Err(mpsc::TryRecvError::Empty) => false,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.response.last = Some(Err("request thread died".to_owned()));
                self.response.loading = false;
                self.pending = None;
                true
            }
        }
    }

    pub fn formatted_response_body(&self) -> String {
        match &self.response.last {
            Some(Ok(resp)) => serde_json::from_str::<serde_json::Value>(&resp.body)
                .ok()
                .and_then(|json| serde_json::to_string_pretty(&json).ok())
                .unwrap_or_else(|| resp.body.clone()),
            Some(Err(e)) => e.clone(),
            None => String::new(),
        }
    }

    pub fn copy_response_to_clipboard(&mut self) {
        let text = self.formatted_response_body();
        if text.is_empty() {
            self.response.clipboard_msg = Some("Nothing to copy".to_owned());
            return;
        }
        match clipboard::copy_to_clipboard(&text) {
            Ok(()) => self.response.clipboard_msg = Some("Copied to clipboard".to_owned()),
            Err(e) => self.response.clipboard_msg = Some(e),
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
fn jsonpath_lookup(value: &serde_json::Value, expr: &str) -> Option<String> {
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
