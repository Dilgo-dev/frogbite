use std::collections::HashMap;

use frogbite::core::http::{self, RequestBody, RequestOptions};
use serde_json::{Value, json};

use super::App;

#[derive(Debug, Clone)]
pub struct GqlOperation {
    pub kind: String,
    pub name: String,
    pub args: Vec<GqlArg>,
    pub return_fields: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GqlArg {
    pub name: String,
    pub type_name: String,
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
      fields {
        name
        args {
          name
          type { kind name ofType { kind name ofType { kind name ofType { kind name } } } }
        }
        type {
          kind
          name
          ofType { kind name ofType { kind name ofType { kind name } } }
        }
      }
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
        let (template, vars_skeleton) = build_template(&op);
        self.request.body = template;
        self.request.body_row = 0;
        self.request.body_col = 0;
        if let Some(id) = self.sidebar.active_request_id.clone() {
            if let Some(req) = self.sidebar.requests.iter_mut().find(|r| r.id == id) {
                req.body.clone_from(&self.request.body);
                op.name.clone_into(&mut req.gql_operation_name);
                if !vars_skeleton.is_empty() {
                    req.gql_variables = vars_skeleton;
                }
            }
            self.save_collections();
        }
        self.graphql.schema_popup_open = false;
    }
}

/// Builds a query template + variables JSON skeleton for an operation.
fn build_template(op: &GqlOperation) -> (String, String) {
    let kind_kw = op.kind.to_lowercase();
    let op_title = capitalize(&op.name);

    let (sig, call_args, vars_json) = if op.args.is_empty() {
        (String::new(), String::new(), String::new())
    } else {
        let sig = op
            .args
            .iter()
            .map(|a| format!("${}: {}", a.name, a.type_name))
            .collect::<Vec<_>>()
            .join(", ");
        let call = op
            .args
            .iter()
            .map(|a| format!("{}: ${}", a.name, a.name))
            .collect::<Vec<_>>()
            .join(", ");
        let mut map = serde_json::Map::new();
        for a in &op.args {
            map.insert(a.name.clone(), placeholder_for(&a.type_name));
        }
        let json = serde_json::to_string_pretty(&Value::Object(map)).unwrap_or_default();
        (format!("({sig})"), format!("({call})"), json)
    };

    let selection = if op.return_fields.is_empty() {
        String::new()
    } else {
        let inner = op
            .return_fields
            .iter()
            .map(|f| format!("    {f}"))
            .collect::<Vec<_>>()
            .join("\n");
        format!(" {{\n{inner}\n  }}")
    };

    let template = format!(
        "{kind_kw} {op_title}{sig} {{\n  {name}{call_args}{selection}\n}}\n",
        name = op.name,
    );
    (template, vars_json)
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect::<String>())
        .unwrap_or_default()
}

fn placeholder_for(type_name: &str) -> Value {
    let base = type_name.trim_end_matches('!').trim_end_matches('!');
    let base = base.trim_start_matches('[').trim_end_matches(']');
    match base {
        "Int" | "Float" => Value::from(0),
        "Boolean" => Value::from(false),
        _ => Value::from(""),
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
                        let Some(name) = f.get("name").and_then(Value::as_str) else {
                            continue;
                        };
                        let args = parse_args(f.get("args"));
                        let ret_type = unwrap_named_type(f.get("type"));
                        let return_fields = ret_type
                            .as_deref()
                            .map(|n| fields_of_type(types, n))
                            .unwrap_or_default();
                        ops.push(GqlOperation {
                            kind: kind.clone(),
                            name: name.to_owned(),
                            args,
                            return_fields,
                        });
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

fn parse_args(value: Option<&Value>) -> Vec<GqlArg> {
    let Some(arr) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|a| {
            let name = a.get("name").and_then(Value::as_str)?.to_owned();
            let type_name = render_type(a.get("type"));
            Some(GqlArg { name, type_name })
        })
        .collect()
}

/// Renders a type ref into a GraphQL type literal like `String!` or `[Int!]!`.
fn render_type(value: Option<&Value>) -> String {
    let Some(v) = value else {
        return "String".to_owned();
    };
    let kind = v.get("kind").and_then(Value::as_str).unwrap_or("");
    let name = v.get("name").and_then(Value::as_str);
    match kind {
        "NON_NULL" => format!("{}!", render_type(v.get("ofType"))),
        "LIST" => format!("[{}]", render_type(v.get("ofType"))),
        _ => name.unwrap_or("String").to_owned(),
    }
}

/// Returns the named base type (peels `NON_NULL` and `LIST` wrappers).
fn unwrap_named_type(value: Option<&Value>) -> Option<String> {
    let mut cur = value?;
    loop {
        let kind = cur.get("kind").and_then(Value::as_str).unwrap_or("");
        if kind == "NON_NULL" || kind == "LIST" {
            cur = cur.get("ofType")?;
        } else {
            return cur.get("name").and_then(Value::as_str).map(str::to_owned);
        }
    }
}

/// Returns the list of scalar/leaf field names of a named OBJECT type, capped
/// to keep the template readable. Skips fields that themselves take args.
fn fields_of_type(types: &[Value], type_name: &str) -> Vec<String> {
    for t in types {
        if t.get("name").and_then(Value::as_str) != Some(type_name) {
            continue;
        }
        let Some(fields) = t.get("fields").and_then(Value::as_array) else {
            return Vec::new();
        };
        return fields
            .iter()
            .filter_map(|f| {
                if f.get("args")
                    .and_then(Value::as_array)
                    .is_some_and(|a| !a.is_empty())
                {
                    return None;
                }
                f.get("name").and_then(Value::as_str).map(str::to_owned)
            })
            .take(8)
            .collect();
    }
    Vec::new()
}
