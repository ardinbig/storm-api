//! System user business logic: authentication and registration.

use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::user::{AuthResponse, RegisterRequest, SystemUser, UserInfo},
    services::auth_service,
    state::app_state::AuthConfig,
    utils::password,
};

/// Authenticates a system user by username and password.
///
/// Looks up the user in the `users` table, verifies the password with
/// Argon2, and on success issues a JWT with `role = "user"`.
///
/// # Errors
///
/// - [`AppError::Unauthorized`] — user not found or password mismatch.
/// - [`AppError::Internal`] — JWT signing failure.
/// - [`AppError::Database`] — query failure.
pub async fn authenticate(
    pool: &PgPool,
    config: &AuthConfig,
    username: &str,
    password: &str,
) -> Result<AuthResponse, AppError> {
    let user = sqlx::query_as::<_, SystemUser>(
        "SELECT id, name, email, password, username FROM users WHERE username = $1",
    )
    .bind(username)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    if !auth_service::verify_password(password, &user.password) {
        return Err(AppError::Unauthorized);
    }

    let token = auth_service::create_token(config, &user.id.to_string(), "user")
        .map_err(|_| AppError::Internal)?;

    Ok(AuthResponse {
        token,
        user: UserInfo {
            id: user.id,
            name: user.name,
            email: user.email,
            username: user.username,
        },
    })
}

/// Registers a new system user.
///
/// Hashes the password with Argon2, generates a `UUID` primary key, and
/// inserts a row into the `users` table.
///
/// # Errors
///
/// - [`AppError::Internal`] — password hashing failure.
/// - [`AppError::Database`] — duplicate username or other DB constraint
///   violation.
pub async fn register(pool: &PgPool, input: &RegisterRequest) -> Result<SystemUser, AppError> {
    let hashed = password::hash(&input.password)?;
    let id = Uuid::new_v4();

    let user = sqlx::query_as::<_, SystemUser>(
        "INSERT INTO users (id, name, email, password, username)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, name, email, password, username",
    )
    .bind(id)
    .bind(&input.name)
    .bind(input.email.as_deref())
    .bind(&hashed)
    .bind(&input.username)
    .fetch_one(pool)
    .await?;

    Ok(user)
}
