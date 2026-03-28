//! Convenience wrapper around Argon2 password hashing.
//!
//! Delegates to [`auth_service::hash_password`](auth_service::hash_password)
//! and maps any hashing failure to [`AppError::Internal`].

use crate::{errors::AppError, services::auth_service};

/// Hashes a plaintext password using Argon2, returning the PHC-format string.
///
/// This is the single entry-point that all service modules should call when
/// they need to store a hashed password.
///
/// # Errors
///
/// Returns [`AppError::Internal`] if the underlying Argon2 operation fails.
pub fn hash(password: &str) -> Result<String, AppError> {
    auth_service::hash_password(password).map_err(|_| AppError::Internal)
}
