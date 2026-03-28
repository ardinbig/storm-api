//! Application router construction.

use axum::{
    Router,
    extract::{Request, State},
    http::{Method, StatusCode, header},
    middleware::{self, Next},
    response::Response,
    routing::post,
};
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer, cors::CorsLayer, timeout::TimeoutLayer, trace::TraceLayer,
};

use crate::{
    handlers::auth_handler,
    models::user::CurrentUser,
    routes,
    services::auth_service,
    state::app_state::{AppState, AuthConfig, RedisPool},
    utils::cache,
};

/// Maximum duration for a single request before the server responds with
/// `408 Request Timeout`.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub fn create_app(state: AppState) -> Router {
    // Public routes (no auth required)
    let public = Router::new().nest("/api/v1/auth", routes::auth::routes());

    // Protected routes (JWT required)
    let protected = Router::new()
        .route("/api/v1/auth/logout", post(auth_handler::logout))
        .nest("/api/v1/users", routes::users::routes())
        .nest("/api/v1/categories", routes::categories::routes())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    Router::new()
        .merge(routes::health::routes())
        .merge(public)
        .merge(protected)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    REQUEST_TIMEOUT,
                ))
                .layer(cors_layer()),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            request_counter,
        ))
        .fallback(not_found)
        .with_state(state)
}

/// Builds a permissive CORS layer that allows any origin, common HTTP methods,
/// and the `Content-Type` / `Authorization` headers.
fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
}

/// JWT authentication middleware.
///
/// Extracts the `Bearer <token>` from the `Authorization` header, checks the
/// Redis blocklist (for logged-out tokens), verifies the token via
/// [`auth_service::verify_token`], and on success inserts a [`CurrentUser`]
/// into request extensions for downstream handlers.
///
/// Returns `401 Unauthorized` if the header is missing, malformed, the token
/// is blocklisted, or the token is invalid/expired.
async fn auth_middleware(
    State(config): State<Arc<AuthConfig>>,
    State(redis): State<RedisPool>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Reject blocklisted (logged-out) tokens
    if cache::is_blocklisted(&redis, token).await {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let claims =
        auth_service::verify_token(&config, token).map_err(|_| StatusCode::UNAUTHORIZED)?;

    request.extensions_mut().insert(CurrentUser {
        id: claims.sub,
        role: claims.role,
    });

    Ok(next.run(request).await)
}

/// Fallback handler for unmatched routes.
///
/// Returns `(404, "404 - Route not found")`.
async fn not_found() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "404 - Route not found")
}

/// Middleware that atomically increments the global request counter on every
/// inbound request. The current count is exposed via the `/metrics` endpoint.
async fn request_counter(State(state): State<AppState>, request: Request, next: Next) -> Response {
    state.request_count.fetch_add(1, Ordering::Relaxed);
    next.run(request).await
}
