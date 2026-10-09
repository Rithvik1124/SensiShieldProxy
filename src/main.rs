mod scanner;
mod document;
mod cache;

use axum::{
    extract::{DefaultBodyLimit, Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use sha2::Digest;
use tracing::{error, info};

#[derive(Clone)]
struct AppState {
    cache: cache::ScanCache,
    max_upload_bytes: usize,
}

#[derive(Debug, Deserialize)]
struct ScanRequest {
    #[serde(default)]
    text: String,
}

pub use scanner::ScanResult;

#[derive(Debug, Serialize)]
struct ErrorBody {
    decision: String,
    error: String,
}

async fn health() -> &'static str { "ok" }

async fn scan_text(
    State(_state): State<Arc<AppState>>,
    Json(body): Json<ScanRequest>,
) -> Json<ScanResult> {
    Json(scanner::scan_text(&body.text))
}

async fn scan_file(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Response {
    let mut filename = String::from("upload.pdf");
    let mut bytes: Option<Vec<u8>> = None;

    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                if let Some(name) = field.file_name() {
                    filename = name.rsplit(|c| c == '/' || c == '\\').next().unwrap_or("upload.pdf").to_string();
                }
                match field.bytes().await {
                    Ok(b) => {
                        if b.len() > state.max_upload_bytes {
                            return (
                                StatusCode::PAYLOAD_TOO_LARGE,
                                Json(ErrorBody {
                                    decision: "error".into(),
                                    error: format!("Upload exceeds {} byte limit", state.max_upload_bytes),
                                }),
                            ).into_response();
                        }
                        // This service accepts one file field; ignore empty form fields.
                        if !b.is_empty() {
                            bytes = Some(b.to_vec());
                            break;
                        }
                    }
                    Err(e) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(ErrorBody { decision: "error".into(), error: e.to_string() }),
                        ).into_response();
                    }
                }
            }
            Ok(None) => break,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorBody { decision: "error".into(), error: e.to_string() }),
                ).into_response();
            }
        }
    }

    let Some(bytes) = bytes else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody { decision: "error".into(), error: "Multipart request did not contain a file".into() }),
        ).into_response();
    };

    let hash = hex::encode(sha2::Sha256::digest(&bytes));

    if let Some(cached) = state.cache.get(&hash).await {
        info!("CACHE HIT hash={}", hash);
        return Json(cached).into_response();
    }
    info!("CACHE MISS hash={} filename={} bytes={}", hash, filename, bytes.len());

    let is_large = bytes.len() > 100 * 1024 * 1024;
    let outcome = tokio::task::spawn_blocking(move || {
        document::extract_and_scan(&bytes, &filename, if is_large { 100 } else { 120 }, if is_large { 20 } else { 12 })
    }).await;

    let result = match outcome {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => {
            error!("document processing failed: {e:#}");
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ErrorBody { decision: "error".into(), error: e.to_string() }),
            ).into_response();
        }
        Err(e) => {
            error!("document worker failed: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody { decision: "error".into(), error: e.to_string() }),
            ).into_response();
        }
    };

    if let Err(e) = state.cache.put(&hash, &result).await {
        // Cache is an optimization: do not fail a scan solely because Redis is down.
        error!("cache write failed: {e}");
    }
    Json(result).into_response()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();

    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".into());
    let cache = cache::ScanCache::new(&redis_url).await;
    let max_upload_bytes = std::env::var("MAX_UPLOAD_BYTES")
        .ok().and_then(|v| v.parse().ok()).unwrap_or(150 * 1024 * 1024);

    let state = Arc::new(AppState { cache, max_upload_bytes });
    let app = Router::new()
        .route("/health", get(health))
        .route("/scan", post(scan_text))
        .route("/scan-file/openai", post(scan_file))
        .route("/scan-file/claude", post(scan_file))
        .layer(DefaultBodyLimit::max(max_upload_bytes))
        .with_state(state);

    let addr: SocketAddr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8000".into()).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Rust DLP backend listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
