use super::*;

#[allow(clippy::option_if_let_else)]
fn http_to_grpc_scheme(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.starts_with("grpc://") || trimmed.starts_with("grpcs://") {
        trimmed.to_owned()
    } else if let Some(rest) = trimmed.strip_prefix("https://") {
        format!("grpcs://{rest}")
    } else if let Some(rest) = trimmed.strip_prefix("http://") {
        format!("grpc://{rest}")
    } else if trimmed.is_empty() {
        "grpc://".to_owned()
    } else {
        format!("grpc://{trimmed}")
    }
}

#[allow(clippy::option_if_let_else)]
fn grpc_to_http_scheme(url: &str) -> String {
    let trimmed = url.trim();
    if let Some(rest) = trimmed.strip_prefix("grpcs://") {
        format!("https://{rest}")
    } else if let Some(rest) = trimmed.strip_prefix("grpc://") {
        format!("http://{rest}")
    } else {
        trimmed.to_owned()
    }
}

impl App {
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
            (
                "Check for updates on startup",
                self.ui.settings.update_check,
            ),
        ]
    }

    pub fn toggle_setting(&mut self) {
        match self.ui.settings_selected {
            0 => self.ui.settings.splash_animation = !self.ui.settings.splash_animation,
            1 => self.ui.settings.vim_keys = !self.ui.settings.vim_keys,
            2 => self.ui.settings.update_check = !self.ui.settings.update_check,
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
        let new_method = Method::all()[self.method_popup.selected].clone();
        let was_grpc = self.request.method == Method::Grpc;
        let now_grpc = new_method == Method::Grpc;
        self.request.method = new_method;
        if now_grpc && !was_grpc {
            self.request.url = http_to_grpc_scheme(&self.request.url);
            self.request.cursor_pos = self.request.url.len();
        } else if was_grpc && !now_grpc {
            self.request.url = grpc_to_http_scheme(&self.request.url);
            self.request.cursor_pos = self.request.url.len();
        }
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

    pub(super) fn load_request_by_id(&mut self, id: &str) {
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
            self.proxy.url.clone_from(&req.proxy_url);
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

    pub(super) fn load_request_at(&mut self, idx: usize) {
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
            req.proxy_url.clone_from(&self.proxy.url);
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
            proto_path: String::new(),
            grpc_method: String::new(),
            gql_variables: String::new(),
            gql_operation_name: String::new(),
            proxy_url: String::new(),
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
                proto_path: req.proto_path,
                grpc_method: req.grpc_method,
                gql_variables: req.gql_variables,
                gql_operation_name: req.gql_operation_name,
                proxy_url: req.proxy_url,
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
}
