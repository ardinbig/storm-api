//! Authentication primitives: password hashing/verification and JWT
//! creation/verification.
//!
//! Uses [Argon2](https://docs.rs/argon2) for password hashing and
//! [jsonwebtoken](https://docs.rs/jsonwebtoken) for JWT operations.

use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};

use crate::{models::user::Claims, state::app_state::AuthConfig};

/// Hashes a plaintext password with Argon2id using a random salt.
///
/// Returns the PHC-format hash string on success.
///
/// # Errors
///
/// Returns the underlying [`argon2::password_hash::Error`] if hashing fails.
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

/// Verifies a plaintext password against an Argon2 PHC-format hash.
///
/// Returns `true` if the password matches, `false` otherwise (including
/// when the stored hash is malformed).
pub fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

/// Creates a signed JWT for the given `user_id` and `role`.
///
/// The token is valid for [`AuthConfig::jwt_expiry_hours`] hours from the
/// current UTC time.
///
/// # Errors
///
/// Returns a [`jsonwebtoken::errors::Error`] if signing fails.
pub fn create_token(
    config: &AuthConfig,
    user_id: &str,
    role: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let expiry = now + chrono::Duration::hours(config.jwt_expiry_hours);
    let claims = Claims {
        sub: user_id.to_string(),
        iat: now.timestamp(),
        exp: expiry.timestamp(),
        role: role.to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
}

/// Decodes and validates a JWT, returning the embedded [`Claims`].
///
/// # Errors
///
/// Returns a [`jsonwebtoken::errors::Error`] if the token is invalid,
/// expired, or the signature does not match.
pub fn verify_token(
    config: &AuthConfig,
    token: &str,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}
