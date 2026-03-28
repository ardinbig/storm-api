//! Authentication handlers for system user login, registration, and logout.

use axum::{Json, extract::Request, extract::State, http::StatusCode, http::header};
use sqlx::PgPool;
use std::sync::Arc;

use crate::{
    errors::AppError,
    models::user::{AuthResponse, LoginRequest, RegisterRequest, UserInfo},
    services::user_service,
    state::app_state::{AuthConfig, RedisPool},
    utils::cache,
};

/// `POST /api/v1/auth/login`
///
/// Authenticates a system user and returns a JWT.
pub async fn login(
    State(pool): State<PgPool>,
    State(config): State<Arc<AuthConfig>>,
    Json(input): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let response =
        user_service::authenticate(&pool, &config, &input.username, &input.password).await?;
    Ok(Json(response))
}

/// `POST /api/v1/auth/register`
///
/// Creates a new system user account. Returns `201 Created` with the
/// user profile (no password).
pub async fn register(
    State(pool): State<PgPool>,
    Json(input): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<UserInfo>), AppError> {
    let user = user_service::register(&pool, &input).await?;
    Ok((
        StatusCode::CREATED,
        Json(UserInfo {
            id: user.id,
            name: user.name,
            email: user.email,
            username: user.username,
        }),
    ))
}

/// `POST /api/v1/auth/logout`  (protected route)
///
/// Adds the caller's JWT to the Redis blocklist for its remaining lifetime.
/// Subsequent requests with this token will be rejected by the
/// auth middleware. When Redis is unavailable the endpoint still returns
/// `200` — the token will simply expire naturally.
pub async fn logout(
    State(config): State<Arc<AuthConfig>>,
    State(redis): State<RedisPool>,
    request: Request,
) -> Result<StatusCode, AppError> {
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized)?;

    // Compute remaining TTL so the blocklist entry auto-expires
    let claims = crate::services::auth_service::verify_token(&config, token)?;
    let now = chrono::Utc::now().timestamp();
    let ttl = (claims.exp - now).max(0) as u64;

    cache::blocklist_token(&redis, token, ttl).await;

    Ok(StatusCode::OK)
}
