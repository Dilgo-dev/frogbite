use super::*;

impl App {
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
