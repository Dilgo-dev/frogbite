use std::collections::HashMap;
use std::path::PathBuf;

use frogbite::core::grpc;
use frogbite::core::http::HttpResponse;
use prost_reflect::DescriptorPool;

use super::App;

#[derive(Default)]
pub struct GrpcState {
    pub pool: Option<DescriptorPool>,
    pub methods: Vec<String>,
    pub proto_error: Option<String>,
    pub proto_popup_open: bool,
    pub proto_buffer: String,
    pub method_popup_open: bool,
    pub method_popup_selected: usize,
}

impl App {
    pub fn open_proto_popup(&mut self) {
        self.grpc.proto_buffer = self
            .sidebar
            .requests
            .iter()
            .find(|r| Some(&r.id) == self.sidebar.active_request_id.as_ref())
            .map(|r| r.proto_path.clone())
            .unwrap_or_default();
        self.grpc.proto_error = None;
        self.grpc.proto_popup_open = true;
    }

    pub fn confirm_proto_popup(&mut self) {
        let path = PathBuf::from(self.grpc.proto_buffer.trim());
        if path.as_os_str().is_empty() {
            self.grpc.proto_error = Some("path is empty".to_owned());
            return;
        }
        match grpc::load_proto(&path) {
            Ok(pool) => {
                self.grpc.methods = grpc::list_methods(&pool);
                self.grpc.pool = Some(pool);
                self.grpc.proto_error = None;
                self.grpc.proto_popup_open = false;
                if let Some(id) = self.sidebar.active_request_id.clone() {
                    if let Some(req) = self.sidebar.requests.iter_mut().find(|r| r.id == id) {
                        self.grpc
                            .proto_buffer
                            .trim()
                            .clone_into(&mut req.proto_path);
                    }
                    self.save_collections();
                }
            }
            Err(e) => self.grpc.proto_error = Some(e),
        }
    }

    pub fn open_grpc_method_popup(&mut self) {
        if self.grpc.pool.is_none() {
            self.try_load_active_proto();
        }
        if self.grpc.pool.is_none() {
            self.open_proto_popup();
            return;
        }
        let active = self
            .sidebar
            .requests
            .iter()
            .find(|r| Some(&r.id) == self.sidebar.active_request_id.as_ref())
            .map(|r| r.grpc_method.clone())
            .unwrap_or_default();
        self.grpc.method_popup_selected = self
            .grpc
            .methods
            .iter()
            .position(|m| m == &active)
            .unwrap_or(0);
        self.grpc.method_popup_open = true;
    }

    pub fn confirm_grpc_method_popup(&mut self) {
        let Some(name) = self
            .grpc
            .methods
            .get(self.grpc.method_popup_selected)
            .cloned()
        else {
            self.grpc.method_popup_open = false;
            return;
        };
        if let Some(id) = self.sidebar.active_request_id.clone() {
            if let Some(req) = self.sidebar.requests.iter_mut().find(|r| r.id == id) {
                req.grpc_method = name;
            }
            self.save_collections();
        }
        self.grpc.method_popup_open = false;
    }

    pub fn try_load_active_proto(&mut self) {
        let Some(id) = self.sidebar.active_request_id.clone() else {
            return;
        };
        let path = self
            .sidebar
            .requests
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.proto_path.clone())
            .unwrap_or_default();
        if path.trim().is_empty() {
            return;
        }
        match grpc::load_proto(&PathBuf::from(path.trim())) {
            Ok(pool) => {
                self.grpc.methods = grpc::list_methods(&pool);
                self.grpc.pool = Some(pool);
                self.grpc.proto_error = None;
            }
            Err(e) => {
                self.grpc.proto_error = Some(e);
            }
        }
    }

    pub fn active_grpc_method_label(&self) -> String {
        self.sidebar
            .requests
            .iter()
            .find(|r| Some(&r.id) == self.sidebar.active_request_id.as_ref())
            .map(|r| r.grpc_method.clone())
            .unwrap_or_default()
    }

    /// Sends a unary gRPC call synchronously and stores the result in
    /// `response.last` as a synthetic `HttpResponse`.
    pub fn send_grpc_request(&mut self) {
        self.response.loading = true;
        self.response.last = None;
        self.response.scroll = 0;
        self.sync_to_collection();

        if self.grpc.pool.is_none() {
            self.try_load_active_proto();
        }
        let Some(pool) = self.grpc.pool.clone() else {
            self.response.last = Some(Err("no .proto loaded - press P to load one".to_owned()));
            self.response.loading = false;
            return;
        };

        let active = self
            .sidebar
            .requests
            .iter()
            .find(|r| Some(&r.id) == self.sidebar.active_request_id.as_ref())
            .cloned();
        let Some(active) = active else {
            self.response.last = Some(Err("no active request".to_owned()));
            self.response.loading = false;
            return;
        };

        if active.grpc_method.is_empty() {
            self.response.last = Some(Err(
                "no gRPC method selected - press G to pick one".to_owned()
            ));
            self.response.loading = false;
            return;
        }

        let Some(method) = grpc::find_method(&pool, &active.grpc_method) else {
            self.response.last = Some(Err(format!(
                "method {} not found in proto",
                active.grpc_method
            )));
            self.response.loading = false;
            return;
        };

        let url = self.resolve_variables(&active.url);
        let body = self.resolve_variables(&active.body);
        let metadata: HashMap<String, String> = active
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), self.resolve_variables(v)))
            .collect();

        let result = grpc::unary_call_blocking(url, method, &body, &metadata, active.timeout_secs);
        self.response.loading = false;
        match result {
            Ok(call) => {
                let synth = HttpResponse {
                    status: 200,
                    status_text: format!("gRPC {}", call.status_code),
                    headers: call.metadata.into_iter().collect(),
                    body: call.body_json,
                    duration_ms: call.duration_ms,
                    set_cookies: Vec::new(),
                    redirect_chain: Vec::new(),
                };
                self.response.last = Some(Ok(synth));
            }
            Err(e) => {
                self.response.last = Some(Err(e));
            }
        }
    }
}
