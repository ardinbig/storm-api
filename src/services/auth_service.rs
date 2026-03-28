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

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use mockall::*;

    // A mockable trait for password operations — allows testing code that
    // depends on hashing/verification without the real argon2 cost.
    #[automock]
    trait PasswordOps {
        fn hash(&self, password: &str) -> Result<String, String>;
        fn verify(&self, password: &str, hash: &str) -> bool;
    }

    #[test]
    fn mock_password_hash_and_verify() {
        let mut mock = MockPasswordOps::new();

        mock.expect_hash()
            .with(eq("secret"))
            .times(1)
            .returning(|_| Ok("mocked-hash".to_string()));

        mock.expect_verify()
            .with(eq("secret"), eq("mocked-hash"))
            .times(1)
            .returning(|_, _| true);

        mock.expect_verify()
            .with(eq("wrong"), eq("mocked-hash"))
            .times(1)
            .returning(|_, _| false);

        let hash = mock.hash("secret").unwrap();
        assert_eq!(hash, "mocked-hash");
        assert!(mock.verify("secret", &hash));
        assert!(!mock.verify("wrong", &hash));
    }

    // A mockable trait for token operations.
    #[automock]
    trait TokenOps {
        fn create(&self, user_id: &str, role: &str) -> Result<String, String>;
        fn verify(&self, token: &str) -> Result<(String, String), String>;
    }

    #[test]
    fn mock_token_create_and_verify() {
        let mut mock = MockTokenOps::new();

        mock.expect_create()
            .with(eq("user-42"), eq("admin"))
            .times(1)
            .returning(|id, role| Ok(format!("token-{id}-{role}")));

        mock.expect_verify()
            .with(eq("token-user-42-admin"))
            .times(1)
            .returning(|_| Ok(("user-42".to_string(), "admin".to_string())));

        mock.expect_verify()
            .with(eq("bad-token"))
            .times(1)
            .returning(|_| Err("invalid".to_string()));

        let token = mock.create("user-42", "admin").unwrap();
        assert_eq!(token, "token-user-42-admin");

        let (sub, role) = mock.verify(&token).unwrap();
        assert_eq!(sub, "user-42");
        assert_eq!(role, "admin");

        assert!(mock.verify("bad-token").is_err());
    }

    // Real function tests (no mocks needed for these pure functions)
    #[test]
    fn real_hash_and_verify() {
        let hash = hash_password("test123").unwrap();
        assert!(verify_password("test123", &hash));
        assert!(!verify_password("wrong", &hash));
    }

    #[test]
    fn real_token_round_trip() {
        let config = AuthConfig {
            jwt_secret: "test-secret".to_string(),
            jwt_expiry_hours: 1,
        };
        let token = create_token(&config, "uid-1", "user").unwrap();
        let claims = verify_token(&config, &token).unwrap();
        assert_eq!(claims.sub, "uid-1");
        assert_eq!(claims.role, "user");
    }

    #[test]
    fn mock_password_hash_returns_error() {
        let mut mock = MockPasswordOps::new();

        mock.expect_hash()
            .with(eq("any"))
            .times(1)
            .returning(|_| Err("hashing failed".to_string()));

        let result = mock.hash("any");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "hashing failed");
    }

    #[test]
    fn mock_token_verify_expired() {
        let mut mock = MockTokenOps::new();

        mock.expect_verify()
            .with(eq("expired-token"))
            .times(1)
            .returning(|_| Err("token expired".to_string()));

        let result = mock.verify("expired-token");
        assert!(result.is_err());
    }

    #[test]
    fn mock_password_ops_multiple_verifications() {
        let mut mock = MockPasswordOps::new();

        mock.expect_hash()
            .with(eq("pw1"))
            .times(1)
            .returning(|_| Ok("hash-pw1".to_string()));

        mock.expect_verify()
            .with(eq("pw1"), eq("hash-pw1"))
            .times(1)
            .returning(|_, _| true);

        mock.expect_verify()
            .with(eq("pw2"), eq("hash-pw1"))
            .times(1)
            .returning(|_, _| false);

        mock.expect_verify()
            .with(eq(""), eq("hash-pw1"))
            .times(1)
            .returning(|_, _| false);

        let hash = mock.hash("pw1").unwrap();
        assert!(mock.verify("pw1", &hash));
        assert!(!mock.verify("pw2", &hash));
        assert!(!mock.verify("", &hash));
    }

    #[test]
    fn mock_token_create_multiple_roles() {
        let mut mock = MockTokenOps::new();

        mock.expect_create()
            .with(eq("user-1"), eq("user"))
            .times(1)
            .returning(|id, role| Ok(format!("token-{id}-{role}")));

        mock.expect_create()
            .with(eq("agent-1"), eq("agent"))
            .times(1)
            .returning(|id, role| Ok(format!("token-{id}-{role}")));

        let user_token = mock.create("user-1", "user").unwrap();
        let agent_token = mock.create("agent-1", "agent").unwrap();

        assert_eq!(user_token, "token-user-1-user");
        assert_eq!(agent_token, "token-agent-1-agent");
        assert_ne!(user_token, agent_token);
    }

    #[test]
    fn real_different_passwords_produce_different_hashes() {
        let h1 = hash_password("password1").unwrap();
        let h2 = hash_password("password2").unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn real_same_password_different_salts() {
        let h1 = hash_password("same").unwrap();
        let h2 = hash_password("same").unwrap();
        assert_ne!(h1, h2, "Different salts should produce different hashes");
        assert!(verify_password("same", &h1));
        assert!(verify_password("same", &h2));
    }
}
