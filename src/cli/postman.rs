use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::collections::{self, Folder, SavedRequest};

#[derive(Deserialize)]
struct Collection {
    item: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    name: Option<String>,
    request: Option<Request>,
    #[serde(rename = "item")]
    children: Option<Vec<Self>>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum UrlValue {
    String(String),
    Object { raw: Option<String> },
}

#[derive(Deserialize)]
struct Request {
    method: Option<String>,
    url: Option<UrlValue>,
    header: Option<Vec<Header>>,
    body: Option<Body>,
}

#[derive(Deserialize)]
struct Header {
    key: String,
    value: String,
    disabled: Option<bool>,
}

#[derive(Deserialize)]
struct Body {
    mode: Option<String>,
    raw: Option<String>,
}

pub struct ImportResult {
    pub folders: Vec<Folder>,
    pub requests: Vec<SavedRequest>,
}

pub fn import(path: &Path) -> Option<ImportResult> {
    let content = fs::read_to_string(path).ok()?;
    let collection: Collection = serde_json::from_str(&content).ok()?;

    let mut folders = Vec::new();
    let mut requests = Vec::new();

    for item in &collection.item {
        process_item(item, None, &mut folders, &mut requests);
    }

    Some(ImportResult { folders, requests })
}

fn process_item(
    item: &Item,
    folder_id: Option<String>,
    folders: &mut Vec<Folder>,
    requests: &mut Vec<SavedRequest>,
) {
    let name = item.name.clone().unwrap_or_else(|| "Untitled".into());

    if let Some(children) = &item.children {
        let fid = collections::new_id();
        folders.push(Folder {
            id: fid.clone(),
            name,
            expanded: true,
        });
        for child in children {
            process_item(child, Some(fid.clone()), folders, requests);
        }
        return;
    }

    if let Some(req) = &item.request {
        let method = req
            .method
            .clone()
            .unwrap_or_else(|| "GET".into())
            .to_uppercase();

        let url = match &req.url {
            Some(UrlValue::String(s)) => s.clone(),
            Some(UrlValue::Object { raw }) => raw.clone().unwrap_or_default(),
            None => String::new(),
        };

        let mut headers = HashMap::new();
        if let Some(hs) = &req.header {
            for h in hs {
                if !h.disabled.unwrap_or(false) {
                    headers.insert(h.key.clone(), h.value.clone());
                }
            }
        }

        let body = req
            .body
            .as_ref()
            .filter(|b| b.mode.as_deref() == Some("raw"))
            .and_then(|b| b.raw.clone())
            .unwrap_or_default();

        requests.push(SavedRequest {
            id: collections::new_id(),
            name,
            method,
            url,
            body,
            headers,
            folder_id,
            auth: collections::Auth::None,
            body_type: collections::BodyType::Raw,
            content_type: collections::ContentType::Json,
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
        });
    }
}
