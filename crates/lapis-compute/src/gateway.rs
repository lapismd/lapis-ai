//! Authenticated loopback OpenAI bridge with explicit local/paired routing.
use crate::Shared;
use axum::{
    Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use futures_util::StreamExt;
use serde_json::Value;
use std::sync::Arc;
const MAX_RESPONSE: usize = 2 * 1024 * 1024;
pub fn router(shared: Arc<Shared>) -> Router {
    Router::new()
        .route("/v1/models", get(models))
        .route("/v1/{*path}", post(local))
        .route("/peer/{peer}/v1/models", get(remote_models))
        .route("/peer/{peer}/v1/{*path}", post(remote))
        .layer(DefaultBodyLimit::max(256 * 1024))
        .with_state(shared)
}
async fn models(State(shared): State<Arc<Shared>>, headers: HeaderMap) -> impl IntoResponse {
    if !authorized(&shared, &headers) {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({"error":"compute authentication required"})),
        );
    }
    let config = shared.config.read().await;
    let data = config
        .model
        .as_ref()
        .map(|id| serde_json::json!({"id":id,"object":"model","owned_by":"local"}));
    (
        StatusCode::OK,
        axum::Json(
            serde_json::json!({"object":"list","data":data.into_iter().collect::<Vec<_>>()}),
        ),
    )
}
async fn remote_models(
    State(shared): State<Arc<Shared>>,
    Path(peer): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !authorized(&shared, &headers) {
        return (
            StatusCode::UNAUTHORIZED,
            "compute authentication required".into(),
        );
    }
    match crate::peers::infer(&shared, &peer, "models", b"{}").await {
        Ok((status, body)) => (
            StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY),
            body,
        ),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "paired inference unavailable".into(),
        ),
    }
}
fn authorized(shared: &Shared, headers: &HeaderMap) -> bool {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == format!("Bearer {}", shared.token))
}
fn valid(path: &str, bytes: &[u8]) -> bool {
    matches!(path, "chat/completions" | "responses")
        && serde_json::from_slice::<Value>(bytes).is_ok_and(|v| v["stream"] != true)
}
async fn local(
    State(shared): State<Arc<Shared>>,
    Path(path): Path<String>,
    headers: HeaderMap,
    bytes: Bytes,
) -> impl IntoResponse {
    if !authorized(&shared, &headers) {
        return (
            StatusCode::UNAUTHORIZED,
            "compute authentication required".into(),
        );
    }
    if !valid(&path, &bytes) {
        return (
            StatusCode::BAD_REQUEST,
            "unsupported inference request".into(),
        );
    }
    match infer(&shared, &path, &bytes).await {
        Ok((status, body)) => (
            StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY),
            body,
        ),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "inference unavailable".into(),
        ),
    }
}
async fn remote(
    State(shared): State<Arc<Shared>>,
    Path((peer, path)): Path<(String, String)>,
    headers: HeaderMap,
    bytes: Bytes,
) -> impl IntoResponse {
    if !authorized(&shared, &headers) {
        return (
            StatusCode::UNAUTHORIZED,
            "compute authentication required".into(),
        );
    }
    if !valid(&path, &bytes) {
        return (
            StatusCode::BAD_REQUEST,
            "unsupported inference request".into(),
        );
    }
    match crate::peers::infer(&shared, &peer, &path, &bytes).await {
        Ok((status, body)) => (
            StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY),
            body,
        ),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "paired inference unavailable".into(),
        ),
    }
}
pub async fn infer(shared: &Shared, path: &str, bytes: &[u8]) -> anyhow::Result<(u16, String)> {
    if path == "models" {
        let config = shared.config.read().await;
        let data = config
            .model
            .as_ref()
            .map(|id| serde_json::json!({"id":id,"object":"model","owned_by":"paired"}));
        return Ok((
            200,
            serde_json::json!({"object":"list","data":data.into_iter().collect::<Vec<_>>()} )
                .to_string(),
        ));
    }
    anyhow::ensure!(
        valid(path, bytes) && bytes.len() <= 256 * 1024,
        "invalid inference"
    );
    let _permit = shared
        .slots
        .try_acquire()
        .map_err(|_| anyhow::anyhow!("compute busy"))?;
    let source = shared
        .source
        .read()
        .await
        .clone()
        .ok_or_else(|| anyhow::anyhow!("model not loaded"))?;
    let mut body: Value = serde_json::from_slice(bytes)?;
    let config = shared.config.read().await;
    anyhow::ensure!(
        body["model"] == config.model.as_deref().unwrap_or("") || body["model"] == "auto",
        "model not loaded"
    );
    drop(config);
    body["model"] = serde_json::json!(
        shared
            .source_model
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("model not ready"))?
    );
    let response = shared
        .client
        .post(format!("{source}/{path}"))
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await?;
    let status = response.status().as_u16();
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        anyhow::ensure!(body.len() + chunk.len() <= MAX_RESPONSE, "response limit");
        body.extend_from_slice(&chunk);
    }
    Ok((status, String::from_utf8(body)?))
}
