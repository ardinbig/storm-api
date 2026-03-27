//! System user types for authentication and authorization.
//!
//! The `users` table holds station/operator accounts that log in via
//! `/api/v1/auth/login` and receive a JWT with `role = "user"`.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Database row for the `users` table — system/station login accounts.
///
/// The `password` field is excluded from JSON serialization via
/// `#[serde(skip_serializing)]`.
#[derive(Debug, Serialize, FromRow)]
pub struct SystemUser {
    /// Primary key (`UUID`).
    pub id: Uuid,
    /// Display name.
    pub name: String,
    /// Optional e-mail address.
    pub email: Option<String>,
    /// Argon2-hashed password (never serialized).
    #[serde(skip_serializing)]
    pub password: String,
    /// Unique login identifier.
    pub username: String,
}

/// Request body for `POST /api/v1/auth/login`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    /// The user's login identifier.
    pub username: String,
    /// Plaintext password to verify.
    pub password: String,
}

/// Request body for `POST /api/v1/auth/register`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    /// Display name.
    pub name: String,
    /// Optional e-mail.
    pub email: Option<String>,
    /// Desired login identifier (must be unique).
    pub username: String,
    /// Plaintext password (will be Argon2-hashed before storage).
    pub password: String,
}

/// JWT claims embedded in every token issued by the API.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — the user or agent UUID as a string.
    pub sub: String,
    /// Expiration timestamp (Unix epoch seconds).
    pub exp: i64,
    /// Issued-at timestamp (Unix epoch seconds).
    pub iat: i64,
    /// Role indicator: `"user"` for system users, `"agent"` for agents.
    pub role: String,
}

/// Identity extracted from a validated JWT and injected into Axum request
/// extensions by the auth middleware.
#[derive(Debug, Clone)]
pub struct CurrentUser {
    /// The authenticated user's or agent's UUID (as a string).
    pub id: String,
    /// `"user"` or `"agent"`.
    pub role: String,
}

/// Successful authentication response returned by login endpoints.
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    /// Signed JWT to include in subsequent `Authorization: Bearer` headers.
    pub token: String,
    /// Summary of the authenticated user (no password).
    pub user: UserInfo,
}

/// Public-facing user information (password omitted).
#[derive(Debug, Serialize, ToSchema)]
pub struct UserInfo {
    /// Primary key.
    pub id: Uuid,
    /// Display name.
    pub name: String,
    /// Optional e-mail.
    pub email: Option<String>,
    /// Login identifier.
    pub username: String,
}

/// Response for `GET /api/v1/users/me` — the authenticated identity.
#[derive(Debug, Serialize, ToSchema)]
pub struct MeResponse {
    /// The authenticated user's or agent's UUID (as a string).
    pub id: String,
    /// `"user"` or `"agent"`.
    pub role: String,
}
