//! Health check and observability handlers.

use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;
use std::sync::atomic::Ordering;
use utoipa::ToSchema;

use crate::state::app_state::AppState;

/// Response body for the `/metrics` endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct MetricsResponse {
    /// Total number of HTTP requests handled since the server started.
    pub requests: u64,
}

/// `GET /health`
///
/// Simple liveness probe — always returns `"OK"`.
#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses(
        (status = 200, description = "Service is alive", body = String, example = json!("OK")),
    ),
)]
pub async fn health() -> &'static str {
    "OK"
}

/// `GET /ready`
///
/// Readiness probe.  Returns `"ready"` (`200`) while the application is
/// accepting traffic, or `"not ready"` (`503`) after a shutdown signal has
/// been received.
#[utoipa::path(
    get,
    path = "/ready",
    tag = "Health",
    responses(
        (status = 200, description = "Service is ready", body = String, example = json!("ready")),
        (status = 503, description = "Service is shutting down", body = String, example = json!("not ready")),
    ),
)]
pub async fn ready(
    State(state): State<AppState>,
) -> Result<&'static str, (StatusCode, &'static str)> {
    if state.ready.load(Ordering::SeqCst) {
        Ok("ready")
    } else {
        Err((StatusCode::SERVICE_UNAVAILABLE, "not ready"))
    }
}

/// `GET /metrics`
///
/// Returns a JSON object with the total number of requests handled since
/// the server started.
#[utoipa::path(
    get,
    path = "/metrics",
    tag = "Health",
    responses(
        (status = 200, description = "Request counter", body = MetricsResponse),
    ),
)]
pub async fn metrics(State(state): State<AppState>) -> Json<MetricsResponse> {
    Json(MetricsResponse {
        requests: state.request_count.load(Ordering::Relaxed),
    })
}
