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

/// Seeds the super-admin account on first boot.
///
/// Checks whether a user with `username = "suadmin"` already exists in the
/// `users` table. If it does, the function is a no-op. Otherwise, it inserts
/// the super-admin row using the plaintext password supplied via the
/// `SUPER_ADMIN_PASSWORD` environment variable.
///
/// Call this once during application startup, **after** the database pool is
/// ready, so that a fresh deployment always has an administrative account
/// available.
///
/// # Errors
///
/// - [`AppError::Internal`] — password hashing failure.
/// - [`AppError::Database`] — any unexpected database error.
pub async fn seed_super_admin(pool: &PgPool) -> Result<(), AppError> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)")
        .bind("suadmin")
        .fetch_one(pool)
        .await?;

    if exists {
        tracing::info!("Super-admin account already exists — skipping seed");
        return Ok(());
    }

    let raw_password =
        std::env::var("SUPER_ADMIN_PASSWORD").unwrap_or_else(|_| "superadminpassword".into());

    let hashed = password::hash(&raw_password)?;
    let id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO users (id, name, email, password, username)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind("Super Administrator")
    .bind("info@ardinbig.com")
    .bind(&hashed)
    .bind("suadmin")
    .execute(pool)
    .await?;

    tracing::info!("Super-admin account created (username: suadmin)");
    Ok(())
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
