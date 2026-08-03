//! Request counting middleware.

use std::sync::atomic::Ordering;

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::state::app_state::AppState;

/// Atomically increments the global request counter on every inbound request.
///
/// The current count is exposed via the `/metrics` endpoint.
pub async fn request_counter(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    state.request_count.fetch_add(1, Ordering::Relaxed);
    next.run(request).await
}
