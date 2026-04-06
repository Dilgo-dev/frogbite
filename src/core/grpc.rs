//! Minimal dynamic gRPC client used by the TUI.
//!
//! Loads a `.proto` file with `protox`, builds `prost-reflect` descriptors,
//! and performs unary gRPC calls over tonic with a custom codec that carries
//! `DynamicMessage`s end-to-end.

use std::collections::HashMap;
use std::hash::BuildHasher;
use std::path::Path;

use prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage, MessageDescriptor, MethodDescriptor};
use serde::Serialize;
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};
use tonic::metadata::MetadataKey;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint};
use tonic::{Request, Status};

/// Loads a `.proto` file and returns the parsed descriptor pool.
pub fn load_proto(path: &Path) -> Result<DescriptorPool, String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_set =
        protox::compile([path], [parent]).map_err(|e| format!("proto compile error: {e}"))?;
    DescriptorPool::from_file_descriptor_set(file_set).map_err(|e| format!("descriptor: {e}"))
}

/// Lists fully qualified `service.Method` names exposed by the pool.
pub fn list_methods(pool: &DescriptorPool) -> Vec<String> {
    let mut out = Vec::new();
    for service in pool.services() {
        for method in service.methods() {
            out.push(format!("{}/{}", service.full_name(), method.name()));
        }
    }
    out.sort();
    out
}

/// Finds a method descriptor by `service.full_name/method_name` path.
pub fn find_method(pool: &DescriptorPool, path: &str) -> Option<MethodDescriptor> {
    let (svc, method) = path.split_once('/')?;
    let service = pool.services().find(|s| s.full_name() == svc)?;
    service.methods().find(|m| m.name() == method)
}

/// Result of a unary gRPC call.
pub struct GrpcCallResult {
    pub body_json: String,
    pub status_code: String,
    pub metadata: Vec<(String, String)>,
    pub duration_ms: u128,
}

/// Performs a synchronous unary gRPC call by driving a temporary tokio
/// runtime on the calling thread.
pub fn unary_call_blocking<S: BuildHasher>(
    url: String,
    method: MethodDescriptor,
    request_json: &str,
    metadata: &HashMap<String, String, S>,
    timeout_secs: u64,
) -> Result<GrpcCallResult, String> {
    let metadata: Vec<(String, String)> = metadata
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("runtime: {e}"))?;
    runtime.block_on(unary_call(
        url,
        method,
        request_json,
        &metadata,
        timeout_secs,
    ))
}

#[allow(clippy::significant_drop_tightening)]
async fn unary_call(
    url: String,
    method: MethodDescriptor,
    request_json: &str,
    metadata: &[(String, String)],
    timeout_secs: u64,
) -> Result<GrpcCallResult, String> {
    let http_url = rewrite_scheme(&url);
    let endpoint = Endpoint::from_shared(http_url.clone())
        .map_err(|e| format!("invalid endpoint: {e}"))?
        .timeout(std::time::Duration::from_secs(timeout_secs.max(1)));
    let endpoint = if http_url.starts_with("https://") {
        endpoint
            .tls_config(ClientTlsConfig::new().with_webpki_roots())
            .map_err(|e| format!("tls: {e}"))?
    } else {
        endpoint
    };

    let channel: Channel = endpoint
        .connect()
        .await
        .map_err(|e| format!("connect: {e}"))?;

    #[allow(clippy::significant_drop_tightening)]
    let mut grpc = tonic::client::Grpc::new(channel);
    grpc.ready().await.map_err(|e| format!("not ready: {e}"))?;

    let req_desc = method.input();
    let resp_desc = method.output();

    let req_msg = if request_json.trim().is_empty() {
        DynamicMessage::new(req_desc.clone())
    } else {
        let mut deserializer = serde_json::Deserializer::from_str(request_json);
        DynamicMessage::deserialize(req_desc.clone(), &mut deserializer)
            .map_err(|e| format!("request json: {e}"))?
    };

    let mut request = Request::new(req_msg);
    for (k, v) in metadata {
        let key =
            MetadataKey::from_bytes(k.as_bytes()).map_err(|e| format!("metadata key {k}: {e}"))?;
        let value = v
            .parse()
            .map_err(|e| format!("metadata value for {k}: {e}"))?;
        request.metadata_mut().insert(key, value);
    }

    let service = method.parent_service().full_name().to_owned();
    let path_str = format!("/{}/{}", service, method.name());
    let path = http::uri::PathAndQuery::from_maybe_shared(path_str)
        .map_err(|e| format!("bad path: {e}"))?;

    let codec = DynCodec {
        resp_desc: resp_desc.clone(),
    };

    let start = std::time::Instant::now();
    let response = grpc
        .unary(request, path, codec)
        .await
        .map_err(|e| format!("grpc: {} {}", e.code(), e.message()))?;
    let duration_ms = start.elapsed().as_millis();

    let mut metadata_out: Vec<(String, String)> = response
        .metadata()
        .clone()
        .into_headers()
        .iter()
        .map(|(k, v)| (k.as_str().to_owned(), v.to_str().unwrap_or("").to_owned()))
        .collect();
    metadata_out.sort_by(|a, b| a.0.cmp(&b.0));

    let message = response.into_inner();
    let mut serializer = serde_json::Serializer::pretty(Vec::new());
    message
        .serialize(&mut serializer)
        .map_err(|e| format!("response json: {e}"))?;
    let body_json =
        String::from_utf8(serializer.into_inner()).map_err(|e| format!("response utf8: {e}"))?;

    Ok(GrpcCallResult {
        body_json,
        status_code: "OK".to_owned(),
        metadata: metadata_out,
        duration_ms,
    })
}

fn rewrite_scheme(url: &str) -> String {
    let trimmed = url.trim();
    trimmed.strip_prefix("grpc://").map_or_else(
        || {
            trimmed
                .strip_prefix("grpcs://")
                .map_or_else(|| trimmed.to_owned(), |rest| format!("https://{rest}"))
        },
        |rest| format!("http://{rest}"),
    )
}

struct DynCodec {
    resp_desc: MessageDescriptor,
}

struct DynEncoder;
struct DynDecoder {
    desc: MessageDescriptor,
}

impl Encoder for DynEncoder {
    type Item = DynamicMessage;
    type Error = Status;

    fn encode(&mut self, item: Self::Item, buf: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        item.encode(buf)
            .map_err(|e| Status::internal(format!("encode: {e}")))
    }
}

impl Decoder for DynDecoder {
    type Item = DynamicMessage;
    type Error = Status;

    fn decode(&mut self, buf: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        let mut msg = DynamicMessage::new(self.desc.clone());
        msg.merge(buf)
            .map_err(|e| Status::internal(format!("decode: {e}")))?;
        Ok(Some(msg))
    }
}

impl Codec for DynCodec {
    type Encode = DynamicMessage;
    type Decode = DynamicMessage;
    type Encoder = DynEncoder;
    type Decoder = DynDecoder;

    fn encoder(&mut self) -> Self::Encoder {
        DynEncoder
    }

    fn decoder(&mut self) -> Self::Decoder {
        DynDecoder {
            desc: self.resp_desc.clone(),
        }
    }
}
