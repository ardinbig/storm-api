//! JWT authentication middleware for protected routes.

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};

use crate::{
    models::user::CurrentUser,
    services::auth_service,
    state::app_state::{AuthConfig, RedisPool},
    utils::cache,
};

/// Extracts the `Bearer <token>` from the `Authorization` header, checks the
/// Redis blocklist (for logged-out tokens), verifies the token, and injects
/// [`CurrentUser`] into request extensions.
///
/// Returns `401 Unauthorized` if the header is missing, malformed, the token
/// is blocklisted, or the token is invalid/expired.
pub async fn auth_middleware(
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

    // Reject blocklisted (logged-out) tokens.
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
