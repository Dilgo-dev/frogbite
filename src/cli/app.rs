use std::collections::HashMap;

use frogbite::core::http::{HttpResponse, RequestOptions};

use crate::collections::{self, CollectionData, Folder, SavedRequest};
use crate::curl;
use crate::environments::{self, Environment, Variable};
use crate::history::{self, HistoryEntry};
use crate::postman;
use crate::settings::{self, Settings};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Method {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Main,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sidebar,
    UrlBar,
    Body,
    Response,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseTab {
    Body,
    Headers,
}

#[allow(clippy::struct_excessive_bools)]
pub struct App {
    pub method: Method,
    pub url: String,
    pub body: String,
    pub headers: HashMap<String, String>,
    pub cursor_pos: usize,
    pub body_row: usize,
    pub body_col: usize,
    pub response: Option<Result<HttpResponse, String>>,
    pub loading: bool,
    pub view: View,
    pub focus: Focus,
    pub response_tab: ResponseTab,
    pub response_scroll: u16,
    pub folders: Vec<Folder>,
    pub requests: Vec<SavedRequest>,
    pub sidebar_selected: usize,
    pub active_request_id: Option<String>,
    pub editing_url: bool,
    pub editing_body: bool,
    pub settings: Settings,
    pub settings_selected: usize,
    pub method_popup: bool,
    pub method_popup_selected: usize,
    pub editing_sidebar_name: bool,
    pub sidebar_edit_buffer: String,
    pub history: Vec<HistoryEntry>,
    pub history_open: bool,
    pub history_selected: usize,
    pub confirm_delete: bool,
    pub curl_import_open: bool,
    pub curl_import_buffer: String,
    pub curl_import_error: bool,
    pub curl_export_open: bool,
    pub curl_export_content: String,
    pub postman_import_open: bool,
    pub postman_import_buffer: String,
    pub postman_import_error: bool,
    pub environments: Vec<Environment>,
    pub active_env_id: Option<String>,
    pub env_popup_open: bool,
    pub env_popup_selected: usize,
    pub env_editor_open: bool,
    pub env_editor_id: String,
    pub env_editor_selected: usize,
    pub env_editing_var: bool,
    pub env_var_key_buffer: String,
    pub env_var_value_buffer: String,
    pub env_var_field: usize,
    pub env_renaming: bool,
    pub env_name_buffer: String,
    pub env_import_open: bool,
    pub env_import_buffer: String,
    pub env_import_error: bool,
}

impl App {
    pub fn new() -> Self {
        let settings = settings::load();
        let hist = history::load();
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
            method: Method::Get,
            url: String::new(),
            body: String::new(),
            headers: HashMap::new(),
            cursor_pos: 0,
            body_row: 0,
            body_col: 0,
            response: None,
            loading: false,
            view: View::Main,
            focus: Focus::Sidebar,
            response_tab: ResponseTab::Body,
            response_scroll: 0,
            folders,
            requests,
            sidebar_selected: 0,
            active_request_id: active_id,
            editing_url: false,
            editing_body: false,
            settings,
            settings_selected: 0,
            method_popup: false,
            method_popup_selected: 0,
            editing_sidebar_name: false,
            sidebar_edit_buffer: String::new(),
            history: hist,
            history_open: false,
            history_selected: 0,
            confirm_delete: false,
            curl_import_open: false,
            curl_import_buffer: String::new(),
            curl_import_error: false,
            curl_export_open: false,
            curl_export_content: String::new(),
            postman_import_open: false,
            postman_import_buffer: String::new(),
            postman_import_error: false,
            environments: env_data.environments,
            active_env_id: env_data.active_id,
            env_popup_open: false,
            env_popup_selected: 0,
            env_editor_open: false,
            env_editor_id: String::new(),
            env_editor_selected: 0,
            env_editing_var: false,
            env_var_key_buffer: String::new(),
            env_var_value_buffer: String::new(),
            env_var_field: 0,
            env_renaming: false,
            env_name_buffer: String::new(),
            env_import_open: false,
            env_import_buffer: String::new(),
            env_import_error: false,
        };

        if let Some(id) = &app.active_request_id.clone() {
            if let Some(idx) = app.requests.iter().position(|r| &r.id == id) {
                app.sidebar_selected = idx;
                app.load_request_by_id(id);
            }
        } else if !app.requests.is_empty() {
            app.load_request_at(0);
        }

        app
    }

    pub fn save_collections(&self) {
        let data = CollectionData {
            folders: self.folders.clone(),
            requests: self.requests.clone(),
            active_request_id: self.active_request_id.clone(),
        };
        collections::save(&data);
    }

    /// Builds a flat list of sidebar items: folders (with their requests) then ungrouped.
    pub fn sidebar_items(&self) -> Vec<SidebarItem> {
        let mut items = Vec::new();

        for folder in &self.folders {
            items.push(SidebarItem::Folder(folder.clone()));
            if folder.expanded {
                for req in &self.requests {
                    if req.folder_id.as_deref() == Some(&folder.id) {
                        items.push(SidebarItem::Request(req.clone()));
                    }
                }
            }
        }

        for req in &self.requests {
            if req.folder_id.is_none() {
                items.push(SidebarItem::Request(req.clone()));
            }
        }

        items.push(SidebarItem::NewRequest);

        items
    }

    // -- Settings --

    pub fn settings_items(&self) -> Vec<(&str, bool)> {
        vec![
            ("Splash animation", self.settings.splash_animation),
            ("Vim keys", self.settings.vim_keys),
        ]
    }

    pub fn toggle_setting(&mut self) {
        match self.settings_selected {
            0 => self.settings.splash_animation = !self.settings.splash_animation,
            1 => self.settings.vim_keys = !self.settings.vim_keys,
            _ => {}
        }
        settings::save(&self.settings);
    }

    // -- Method popup --

    pub fn open_method_popup(&mut self) {
        self.method_popup_selected = Method::all()
            .iter()
            .position(|m| m == &self.method)
            .unwrap_or(0);
        self.method_popup = true;
    }

    pub fn confirm_method_popup(&mut self) {
        self.method = Method::all()[self.method_popup_selected].clone();
        self.method_popup = false;
        self.sync_to_collection();
    }

    // -- Sidebar navigation --

    pub fn sidebar_len(&self) -> usize {
        self.sidebar_items().len()
    }

    pub fn selected_sidebar_item(&self) -> Option<SidebarItem> {
        self.sidebar_items().get(self.sidebar_selected).cloned()
    }

    pub fn toggle_folder_at_cursor(&mut self) {
        if let Some(SidebarItem::Folder(f)) = self.selected_sidebar_item() {
            if let Some(folder) = self.folders.iter_mut().find(|fo| fo.id == f.id) {
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
        if let Some(req) = self.requests.iter().find(|r| r.id == id) {
            self.method = Method::from_str(&req.method);
            self.url = req.url.clone();
            self.body = req.body.clone();
            self.headers = req.headers.clone();
            self.cursor_pos = self.url.len();
            self.body_row = 0;
            self.body_col = 0;
            self.response = None;
            self.response_scroll = 0;
            self.active_request_id = Some(id.to_owned());
            self.save_collections();
        }
    }

    fn load_request_at(&mut self, idx: usize) {
        if let Some(req) = self.requests.get(idx) {
            let id = req.id.clone();
            self.load_request_by_id(&id);
        }
    }

    /// Syncs current editor state back to the collection.
    pub fn sync_to_collection(&mut self) {
        let Some(id) = self.active_request_id.clone() else {
            return;
        };
        if let Some(req) = self.requests.iter_mut().find(|r| r.id == id) {
            self.method.as_str().clone_into(&mut req.method);
            req.url.clone_from(&self.url);
            req.body.clone_from(&self.body);
            req.headers.clone_from(&self.headers);
        }
        self.save_collections();
    }

    // -- Sidebar editing --

    pub fn start_editing_name(&mut self) {
        match self.selected_sidebar_item() {
            Some(SidebarItem::Request(req)) => {
                self.sidebar_edit_buffer = req.name;
                self.editing_sidebar_name = true;
            }
            Some(SidebarItem::Folder(f)) => {
                self.sidebar_edit_buffer = f.name;
                self.editing_sidebar_name = true;
            }
            Some(SidebarItem::NewRequest) | None => {}
        }
    }

    pub fn confirm_editing_name(&mut self) {
        match self.selected_sidebar_item() {
            Some(SidebarItem::Request(req)) => {
                if let Some(r) = self.requests.iter_mut().find(|r| r.id == req.id) {
                    r.name.clone_from(&self.sidebar_edit_buffer);
                }
            }
            Some(SidebarItem::Folder(f)) => {
                if let Some(fo) = self.folders.iter_mut().find(|fo| fo.id == f.id) {
                    fo.name.clone_from(&self.sidebar_edit_buffer);
                }
            }
            Some(SidebarItem::NewRequest) | None => {}
        }
        self.editing_sidebar_name = false;
        self.save_collections();
    }

    pub fn cycle_sidebar_method(&mut self) {
        if let Some(SidebarItem::Request(req)) = self.selected_sidebar_item() {
            if let Some(r) = self.requests.iter_mut().find(|r| r.id == req.id) {
                let m = Method::from_str(&r.method).next();
                m.as_str().clone_into(&mut r.method);
                if self.active_request_id.as_deref() == Some(&req.id) {
                    self.method = Method::from_str(&r.method);
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
        };

        let id = req.id.clone();
        self.requests.push(req);
        self.save_collections();
        self.load_request_by_id(&id);

        let items = self.sidebar_items();
        if let Some(idx) = items
            .iter()
            .position(|item| matches!(item, SidebarItem::Request(r) if r.id == id))
        {
            self.sidebar_selected = idx;
        }

        self.focus = Focus::UrlBar;
    }

    pub fn create_folder(&mut self) {
        let folder = Folder {
            id: collections::new_id(),
            name: "New Folder".into(),
            expanded: true,
        };
        self.folders.push(folder);
        self.save_collections();

        let items = self.sidebar_items();
        self.sidebar_selected = items.len().saturating_sub(1);
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
            };
            let id = new_req.id.clone();
            self.requests.push(new_req);
            self.save_collections();
            self.load_request_by_id(&id);

            let items = self.sidebar_items();
            if let Some(idx) = items
                .iter()
                .position(|item| matches!(item, SidebarItem::Request(r) if r.id == id))
            {
                self.sidebar_selected = idx;
            }
        }
    }

    pub fn request_delete(&mut self) {
        if self.confirm_delete {
            self.confirm_delete_action();
        } else {
            self.confirm_delete = true;
        }
    }

    pub const fn cancel_delete(&mut self) {
        self.confirm_delete = false;
    }

    fn confirm_delete_action(&mut self) {
        self.confirm_delete = false;
        match self.selected_sidebar_item() {
            Some(SidebarItem::Request(req)) => {
                self.requests.retain(|r| r.id != req.id);
                if self.active_request_id.as_deref() == Some(&req.id) {
                    self.active_request_id = None;
                    self.url.clear();
                    self.body.clear();
                    self.response = None;
                }
            }
            Some(SidebarItem::Folder(f)) => {
                self.requests
                    .retain(|r| r.folder_id.as_deref() != Some(&f.id));
                self.folders.retain(|fo| fo.id != f.id);
                self.active_request_id = None;
                self.url.clear();
                self.body.clear();
                self.response = None;
            }
            Some(SidebarItem::NewRequest) | None => return,
        }

        let max = self.sidebar_items().len().saturating_sub(1);
        if self.sidebar_selected > max {
            self.sidebar_selected = max;
        }

        self.save_collections();
    }

    // -- URL cursor editing --

    pub fn url_insert(&mut self, c: char) {
        self.url.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    pub fn url_backspace(&mut self) {
        if self.cursor_pos > 0 {
            let prev = self.url[..self.cursor_pos]
                .char_indices()
                .next_back()
                .map_or(0, |(i, _)| i);
            self.url.remove(prev);
            self.cursor_pos = prev;
        }
    }

    pub fn url_delete(&mut self) {
        if self.cursor_pos < self.url.len() {
            self.url.remove(self.cursor_pos);
        }
    }

    pub fn url_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos = self.url[..self.cursor_pos]
                .char_indices()
                .next_back()
                .map_or(0, |(i, _)| i);
        }
    }

    pub fn url_cursor_right(&mut self) {
        if self.cursor_pos < self.url.len() {
            self.cursor_pos += self.url[self.cursor_pos..]
                .chars()
                .next()
                .map_or(0, char::len_utf8);
        }
    }

    pub const fn url_cursor_home(&mut self) {
        self.cursor_pos = 0;
    }

    pub fn url_cursor_end(&mut self) {
        self.cursor_pos = self.url.len();
    }

    pub fn finish_url_edit(&mut self) {
        self.editing_url = false;
        self.sync_to_collection();
    }

    // -- Body cursor editing --

    fn body_line_count(&self) -> usize {
        self.body.split('\n').count().max(1)
    }

    fn body_line_len(&self, row: usize) -> usize {
        self.body.split('\n').nth(row).map_or(0, str::len)
    }

    fn body_cursor_offset(&self) -> usize {
        let mut offset = 0;
        for (i, line) in self.body.split('\n').enumerate() {
            if i == self.body_row {
                return offset + self.body_col.min(line.len());
            }
            offset += line.len() + 1;
        }
        self.body.len()
    }

    pub fn body_insert(&mut self, c: char) {
        let offset = self.body_cursor_offset();
        self.body.insert(offset, c);
        self.body_col += c.len_utf8();
    }

    pub fn body_insert_newline(&mut self) {
        let offset = self.body_cursor_offset();
        self.body.insert(offset, '\n');
        self.body_row += 1;
        self.body_col = 0;
    }

    pub fn body_insert_tab(&mut self) {
        let offset = self.body_cursor_offset();
        self.body.insert_str(offset, "  ");
        self.body_col += 2;
    }

    pub fn body_backspace(&mut self) {
        if self.body_col > 0 {
            let offset = self.body_cursor_offset();
            self.body.remove(offset - 1);
            self.body_col -= 1;
        } else if self.body_row > 0 {
            let prev_len = self.body_line_len(self.body_row - 1);
            let offset = self.body_cursor_offset();
            self.body.remove(offset - 1);
            self.body_row -= 1;
            self.body_col = prev_len;
        }
    }

    pub fn body_delete(&mut self) {
        let offset = self.body_cursor_offset();
        if offset < self.body.len() {
            self.body.remove(offset);
        }
    }

    pub fn body_cursor_left(&mut self) {
        if self.body_col > 0 {
            self.body_col -= 1;
        } else if self.body_row > 0 {
            self.body_row -= 1;
            self.body_col = self.body_line_len(self.body_row);
        }
    }

    pub fn body_cursor_right(&mut self) {
        let line_len = self.body_line_len(self.body_row);
        if self.body_col < line_len {
            self.body_col += 1;
        } else if self.body_row + 1 < self.body_line_count() {
            self.body_row += 1;
            self.body_col = 0;
        }
    }

    pub fn body_cursor_up(&mut self) {
        if self.body_row > 0 {
            self.body_row -= 1;
            self.body_col = self.body_col.min(self.body_line_len(self.body_row));
        }
    }

    pub fn body_cursor_down(&mut self) {
        if self.body_row + 1 < self.body_line_count() {
            self.body_row += 1;
            self.body_col = self.body_col.min(self.body_line_len(self.body_row));
        }
    }

    pub const fn body_cursor_home(&mut self) {
        self.body_col = 0;
    }

    pub fn body_cursor_end(&mut self) {
        self.body_col = self.body_line_len(self.body_row);
    }

    pub fn enter_body_edit(&mut self) {
        self.editing_body = true;
        let count = self.body_line_count();
        self.body_row = count.saturating_sub(1);
        self.body_col = self.body_line_len(self.body_row);
    }

    pub fn finish_body_edit(&mut self) {
        self.editing_body = false;
        self.sync_to_collection();
    }

    // -- cURL import --

    pub fn open_curl_import(&mut self) {
        self.curl_import_buffer = String::new();
        self.curl_import_error = false;
        self.curl_import_open = true;
    }

    pub fn confirm_curl_import(&mut self) {
        let Some(parsed) = curl::parse(&self.curl_import_buffer) else {
            self.curl_import_error = true;
            return;
        };

        let folder_id = match self.selected_sidebar_item() {
            Some(SidebarItem::Folder(f)) => Some(f.id),
            Some(SidebarItem::Request(r)) => r.folder_id,
            Some(SidebarItem::NewRequest) | None => None,
        };

        let name = name_from_url(&parsed.url);
        let req = SavedRequest {
            id: collections::new_id(),
            name,
            method: parsed.method,
            url: parsed.url,
            body: parsed.body,
            headers: parsed.headers,
            folder_id,
        };

        let id = req.id.clone();
        self.requests.push(req);
        self.save_collections();
        self.load_request_by_id(&id);

        let items = self.sidebar_items();
        if let Some(idx) = items
            .iter()
            .position(|item| matches!(item, SidebarItem::Request(r) if r.id == id))
        {
            self.sidebar_selected = idx;
        }

        self.curl_import_open = false;
        self.focus = Focus::UrlBar;
    }

    pub fn open_curl_export(&mut self) {
        self.curl_export_content =
            curl::export(self.method.as_str(), &self.url, &self.headers, &self.body);
        self.curl_export_open = true;
    }

    // -- Postman import --

    pub fn open_postman_import(&mut self) {
        self.postman_import_buffer = String::new();
        self.postman_import_error = false;
        self.postman_import_open = true;
    }

    pub fn confirm_postman_import(&mut self) {
        let path = std::path::Path::new(self.postman_import_buffer.trim());
        let Some(result) = postman::import(path) else {
            self.postman_import_error = true;
            return;
        };

        self.folders.extend(result.folders);
        self.requests.extend(result.requests);
        self.save_collections();

        self.postman_import_open = false;
    }

    // -- History --

    pub fn open_history(&mut self) {
        self.history = history::load();
        self.history_selected = 0;
        self.history_open = true;
    }

    pub fn load_from_history(&mut self) {
        if let Some(entry) = self.history.iter().rev().nth(self.history_selected) {
            self.method = Method::from_str(&entry.method);
            self.url = entry.url.clone();
            self.body = entry.body.clone();
            self.cursor_pos = self.url.len();
            self.body_row = 0;
            self.body_col = 0;
            self.response = None;
            self.history_open = false;
        }
    }

    // -- Environments --

    pub fn save_environments(&self) {
        let data = environments::EnvironmentData {
            environments: self.environments.clone(),
            active_id: self.active_env_id.clone(),
        };
        environments::save(&data);
    }

    pub fn active_env_name(&self) -> Option<&str> {
        let id = self.active_env_id.as_ref()?;
        self.environments
            .iter()
            .find(|e| e.id == *id)
            .map(|e| e.name.as_str())
    }

    pub const fn open_env_popup(&mut self) {
        self.env_popup_selected = 0;
        self.env_popup_open = true;
    }

    pub fn env_popup_count(&self) -> usize {
        self.environments.len() + 1
    }

    pub fn select_env_from_popup(&mut self) {
        if self.env_popup_selected == 0 {
            self.active_env_id = None;
        } else {
            let idx = self.env_popup_selected - 1;
            if let Some(env) = self.environments.get(idx) {
                self.active_env_id = Some(env.id.clone());
            }
        }
        self.save_environments();
        self.env_popup_open = false;
    }

    pub fn create_environment(&mut self) {
        let env = Environment {
            id: collections::new_id(),
            name: String::new(),
            variables: Vec::new(),
        };
        self.environments.push(env);
        self.env_popup_selected = self.environments.len();
        self.env_name_buffer = String::new();
        self.env_renaming = true;
        self.save_environments();
    }

    pub fn delete_env_from_popup(&mut self) {
        if self.env_popup_selected == 0 {
            return;
        }
        let idx = self.env_popup_selected - 1;
        if idx < self.environments.len() {
            let removed_id = self.environments[idx].id.clone();
            self.environments.remove(idx);
            if self.active_env_id.as_deref() == Some(&removed_id) {
                self.active_env_id = None;
            }
            let max = self.env_popup_count().saturating_sub(1);
            if self.env_popup_selected > max {
                self.env_popup_selected = max;
            }
            self.save_environments();
        }
    }

    pub fn start_env_rename(&mut self) {
        if self.env_popup_selected == 0 {
            return;
        }
        let idx = self.env_popup_selected - 1;
        if let Some(env) = self.environments.get(idx) {
            self.env_name_buffer = env.name.clone();
            self.env_renaming = true;
        }
    }

    pub fn confirm_env_rename(&mut self) {
        if self.env_popup_selected == 0 {
            self.env_renaming = false;
            return;
        }
        let idx = self.env_popup_selected - 1;
        if self.env_name_buffer.trim().is_empty() {
            self.cancel_env_rename();
            return;
        }
        if let Some(env) = self.environments.get_mut(idx) {
            env.name.clone_from(&self.env_name_buffer);
        }
        self.env_renaming = false;
        self.save_environments();
    }

    pub fn cancel_env_rename(&mut self) {
        if self.env_popup_selected > 0 {
            let idx = self.env_popup_selected - 1;
            if idx < self.environments.len() && self.environments[idx].name.is_empty() {
                self.environments.remove(idx);
                let max = self.env_popup_count().saturating_sub(1);
                if self.env_popup_selected > max {
                    self.env_popup_selected = max;
                }
                self.save_environments();
            }
        }
        self.env_renaming = false;
    }

    pub fn open_env_import(&mut self) {
        self.env_import_buffer = String::new();
        self.env_import_error = false;
        self.env_import_open = true;
    }

    pub fn confirm_env_import(&mut self) {
        let path = std::path::Path::new(self.env_import_buffer.trim());
        let Some(vars) = environments::parse_dotenv(path) else {
            self.env_import_error = true;
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
        self.environments.push(env);
        self.env_popup_selected = self.environments.len();
        self.save_environments();
        self.env_import_open = false;
    }

    pub fn open_env_editor(&mut self) {
        if self.env_popup_selected == 0 {
            return;
        }
        let idx = self.env_popup_selected - 1;
        if let Some(env) = self.environments.get(idx) {
            self.env_editor_id = env.id.clone();
            self.env_editor_selected = 0;
            self.env_popup_open = false;
            self.env_editor_open = true;
        }
    }

    fn edited_env(&self) -> Option<&Environment> {
        self.environments
            .iter()
            .find(|e| e.id == self.env_editor_id)
    }

    pub fn env_editor_count(&self) -> usize {
        self.edited_env().map_or(1, |e| e.variables.len() + 1)
    }

    pub fn start_add_var(&mut self) {
        self.env_var_key_buffer.clear();
        self.env_var_value_buffer.clear();
        self.env_var_field = 0;
        self.env_editing_var = true;
    }

    pub fn start_edit_var(&mut self) {
        let id = self.env_editor_id.clone();
        let Some(env) = self.environments.iter().find(|e| e.id == id) else {
            return;
        };
        if self.env_editor_selected >= env.variables.len() {
            self.start_add_var();
            return;
        }
        self.env_var_key_buffer = env.variables[self.env_editor_selected].key.clone();
        self.env_var_value_buffer = env.variables[self.env_editor_selected].value.clone();
        self.env_var_field = 0;
        self.env_editing_var = true;
    }

    pub fn confirm_var_edit(&mut self) {
        if self.env_var_key_buffer.trim().is_empty() {
            self.env_editing_var = false;
            return;
        }
        let id = self.env_editor_id.clone();
        let Some(env) = self.environments.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if self.env_editor_selected < env.variables.len() {
            let var = &mut env.variables[self.env_editor_selected];
            var.key.clone_from(&self.env_var_key_buffer);
            var.value.clone_from(&self.env_var_value_buffer);
        } else {
            env.variables.push(Variable {
                key: self.env_var_key_buffer.clone(),
                value: self.env_var_value_buffer.clone(),
                secret: false,
            });
        }
        self.env_editing_var = false;
        self.save_environments();
    }

    pub fn delete_var(&mut self) {
        let id = self.env_editor_id.clone();
        let Some(env) = self.environments.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if self.env_editor_selected < env.variables.len() {
            env.variables.remove(self.env_editor_selected);
            let max = env.variables.len();
            if self.env_editor_selected > max {
                self.env_editor_selected = max;
            }
            self.save_environments();
        }
    }

    pub fn toggle_var_secret(&mut self) {
        let id = self.env_editor_id.clone();
        let Some(env) = self.environments.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if self.env_editor_selected < env.variables.len() {
            let var = &mut env.variables[self.env_editor_selected];
            var.secret = !var.secret;
            self.save_environments();
        }
    }

    fn resolve_variables(&self, input: &str) -> String {
        let Some(env_id) = &self.active_env_id else {
            return input.to_owned();
        };
        let Some(env) = self.environments.iter().find(|e| e.id == *env_id) else {
            return input.to_owned();
        };
        let mut result = input.to_owned();
        for var in &env.variables {
            let pattern = format!("{{{{{}}}}}", var.key);
            result = result.replace(&pattern, &var.value);
        }
        result
    }

    // -- Request --

    pub fn send_request(&mut self) {
        self.loading = true;
        self.response_scroll = 0;
        self.sync_to_collection();

        let resolved_url = self.resolve_variables(&self.url);
        let resolved_body = self.resolve_variables(&self.body);
        let resolved_headers: HashMap<String, String> = self
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), self.resolve_variables(v)))
            .collect();

        let opts = RequestOptions {
            method: self.method.as_str().to_owned(),
            url: resolved_url,
            headers: resolved_headers,
            body: if resolved_body.is_empty() {
                None
            } else {
                Some(resolved_body)
            },
        };

        let result = frogbite::core::http::send_request(&opts);

        let entry = HistoryEntry {
            method: self.method.as_str().to_owned(),
            url: self.url.clone(),
            body: self.body.clone(),
            status: result.as_ref().ok().map(|r| r.status),
            duration_ms: result.as_ref().ok().map(|r| r.duration_ms),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
        };
        history::append(entry);

        self.response = Some(result);
        self.loading = false;
    }

    pub fn formatted_response_body(&self) -> String {
        match &self.response {
            Some(Ok(resp)) => serde_json::from_str::<serde_json::Value>(&resp.body)
                .ok()
                .and_then(|json| serde_json::to_string_pretty(&json).ok())
                .unwrap_or_else(|| resp.body.clone()),
            Some(Err(e)) => e.clone(),
            None => String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SidebarItem {
    Folder(Folder),
    Request(SavedRequest),
    NewRequest,
}

fn name_from_url(url: &str) -> String {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let path = without_scheme.split('?').next().unwrap_or(without_scheme);
    let last_segment = path.rsplit('/').find(|s| !s.is_empty()).unwrap_or(path);
    if last_segment.is_empty() || last_segment.len() > 20 {
        "Imported Request".to_owned()
    } else {
        last_segment.to_owned()
    }
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
            },
        ],
        active_request_id: None,
    }
}
