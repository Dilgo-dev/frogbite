use super::*;

impl App {
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
            proto_path: String::new(),
            grpc_method: String::new(),
            gql_variables: String::new(),
            gql_operation_name: String::new(),
            proxy_url: String::new(),
            plugins: Vec::new(),
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
}
