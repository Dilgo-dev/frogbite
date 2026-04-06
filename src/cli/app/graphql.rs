use std::collections::HashMap;

use frogbite::core::http::{self, RequestBody, RequestOptions};
use serde_json::{Value, json};

use super::App;

#[derive(Debug, Clone)]
pub struct GqlOperation {
    pub kind: String,
    pub name: String,
}

#[derive(Default)]
pub struct GraphqlState {
    pub vars_popup_open: bool,
    pub vars_buffer: String,
    pub vars_error: Option<String>,
    pub schema_popup_open: bool,
    pub schema_popup_selected: usize,
    pub operations: Vec<GqlOperation>,
    pub schema_error: Option<String>,
}

const INTROSPECTION_QUERY: &str = r"
query IntrospectionQuery {
  __schema {
    queryType { name }
    mutationType { name }
    subscriptionType { name }
    types {
      kind
      name
      fields { name }
    }
  }
}
";

impl App {
    /// Builds the JSON envelope sent for a GraphQL request:
    /// `{"query": ..., "variables": ..., "operationName": ...}`.
    pub fn build_graphql_envelope(&self) -> String {
        let query = self.resolve_variables(&self.request.body);
        let active = self
            .sidebar
            .requests
            .iter()
            .find(|r| Some(&r.id) == self.sidebar.active_request_id.as_ref());
        let vars_raw = active.map(|r| r.gql_variables.clone()).unwrap_or_default();
        let op_name = active
            .map(|r| r.gql_operation_name.clone())
            .unwrap_or_default();

        let variables: Value = if vars_raw.trim().is_empty() {
            Value::Object(serde_json::Map::new())
        } else {
            let resolved = self.resolve_variables(&vars_raw);
            serde_json::from_str(&resolved)
                .unwrap_or_else(|_| Value::Object(serde_json::Map::new()))
        };

        let mut envelope = serde_json::Map::new();
        envelope.insert("query".to_owned(), Value::String(query));
        envelope.insert("variables".to_owned(), variables);
        if !op_name.trim().is_empty() {
            envelope.insert("operationName".to_owned(), Value::String(op_name));
        }
        serde_json::to_string(&Value::Object(envelope)).unwrap_or_default()
    }

    pub fn active_gql_variables(&self) -> String {
        self.sidebar
            .requests
            .iter()
            .find(|r| Some(&r.id) == self.sidebar.active_request_id.as_ref())
            .map(|r| r.gql_variables.clone())
            .unwrap_or_default()
    }

    pub fn set_active_gql_variables(&mut self, value: String) {
        if let Some(id) = self.sidebar.active_request_id.clone() {
            if let Some(req) = self.sidebar.requests.iter_mut().find(|r| r.id == id) {
                req.gql_variables = value;
            }
            self.save_collections();
        }
    }

    /// Sends the standard introspection query synchronously and stores the
    /// list of operations in `graphql.operations`.
    pub fn open_gql_vars_popup(&mut self) {
        self.graphql.vars_buffer = self.active_gql_variables();
        self.graphql.vars_error = None;
        self.graphql.vars_popup_open = true;
    }

    pub fn confirm_gql_vars_popup(&mut self) {
        let buf = self.graphql.vars_buffer.clone();
        if !buf.trim().is_empty() {
            if let Err(e) = serde_json::from_str::<Value>(&buf) {
                self.graphql.vars_error = Some(format!("invalid json: {e}"));
                return;
            }
        }
        self.set_active_gql_variables(buf);
        self.graphql.vars_popup_open = false;
        self.graphql.vars_error = None;
    }

    pub fn run_introspection(&mut self) {
        self.graphql.schema_error = None;
        self.graphql.operations.clear();

        let url = self.resolve_variables(&self.request.url);
        if url.trim().is_empty() {
            self.graphql.schema_error = Some("URL is empty".to_owned());
            return;
        }

        let mut headers: HashMap<String, String> = self
            .request
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), self.resolve_variables(v)))
            .collect();
        self.apply_auth_headers(&mut headers);
        headers
            .entry("Content-Type".to_owned())
            .or_insert_with(|| "application/json".to_owned());

        let body = json!({ "query": INTROSPECTION_QUERY }).to_string();
        let opts = RequestOptions {
            method: "POST".to_owned(),
            url,
            headers,
            body: Some(RequestBody::Raw(body)),
            follow_redirects: self.follow_redirects,
            timeout_secs: self.timeout.secs,
            verify_tls: self.tls.verify,
            ca_cert_path: self.tls.ca_cert.clone(),
            client_cert_path: self.tls.client_cert.clone(),
            client_key_path: self.tls.client_key.clone(),
            tls_min_version: self.tls.min_version.clone(),
        };

        match http::send_request(&opts) {
            Ok(resp) => match parse_introspection(&resp.body) {
                Ok(operations) => {
                    self.graphql.operations = operations;
                    self.graphql.schema_popup_selected = 0;
                    self.graphql.schema_popup_open = true;
                }
                Err(e) => self.graphql.schema_error = Some(e),
            },
            Err(e) => self.graphql.schema_error = Some(e),
        }
    }

    pub fn confirm_gql_schema_popup(&mut self) {
        let Some(op) = self
            .graphql
            .operations
            .get(self.graphql.schema_popup_selected)
            .cloned()
        else {
            self.graphql.schema_popup_open = false;
            return;
        };
        let template = format!("{} {{\n  {}\n}}\n", op.kind.to_lowercase(), op.name);
        self.request.body = template;
        self.request.body_row = 0;
        self.request.body_col = 0;
        if let Some(id) = self.sidebar.active_request_id.clone() {
            if let Some(req) = self.sidebar.requests.iter_mut().find(|r| r.id == id) {
                req.body.clone_from(&self.request.body);
                req.gql_operation_name.clone_from(&op.name);
            }
            self.save_collections();
        }
        self.graphql.schema_popup_open = false;
    }
}

fn parse_introspection(body: &str) -> Result<Vec<GqlOperation>, String> {
    let json: Value = serde_json::from_str(body).map_err(|e| format!("invalid json: {e}"))?;
    let schema = json
        .get("data")
        .and_then(|d| d.get("__schema"))
        .ok_or_else(|| "no __schema in response".to_owned())?;

    let mut roots: Vec<(String, String)> = Vec::new();
    for (kind, key) in [
        ("Query", "queryType"),
        ("Mutation", "mutationType"),
        ("Subscription", "subscriptionType"),
    ] {
        if let Some(name) = schema
            .get(key)
            .and_then(|t| t.get("name"))
            .and_then(Value::as_str)
        {
            roots.push((kind.to_owned(), name.to_owned()));
        }
    }

    let types = schema
        .get("types")
        .and_then(Value::as_array)
        .ok_or_else(|| "no types in schema".to_owned())?;

    let mut ops = Vec::new();
    for (kind, type_name) in &roots {
        for t in types {
            if t.get("name").and_then(Value::as_str) == Some(type_name.as_str()) {
                if let Some(fields) = t.get("fields").and_then(Value::as_array) {
                    for f in fields {
                        if let Some(name) = f.get("name").and_then(Value::as_str) {
                            ops.push(GqlOperation {
                                kind: kind.clone(),
                                name: name.to_owned(),
                            });
                        }
                    }
                }
                break;
            }
        }
    }

    if ops.is_empty() {
        return Err("schema has no operations".to_owned());
    }
    Ok(ops)
}
