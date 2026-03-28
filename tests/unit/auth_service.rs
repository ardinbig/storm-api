use crate::common::test_config;
use storm_api::{services::auth_service, state::app_state::AuthConfig};

#[test]
fn hash_and_verify_password() {
    let hash = auth_service::hash_password("secret").unwrap();
    assert!(auth_service::verify_password("secret", &hash));
    assert!(!auth_service::verify_password("wrong", &hash));
}

#[test]
fn verify_password_with_invalid_hash() {
    assert!(!auth_service::verify_password("secret", "not-a-valid-hash"));
}

#[test]
fn verify_password_with_empty_hash() {
    assert!(!auth_service::verify_password("secret", ""));
}

#[test]
fn hash_empty_password_succeeds() {
    let hash = auth_service::hash_password("").unwrap();
    assert!(auth_service::verify_password("", &hash));
    assert!(!auth_service::verify_password("not.empty", &hash));
}

#[test]
fn create_and_verify_token() {
    let config = test_config();
    let token = auth_service::create_token(&config, "user-123", "user").unwrap();
    let claims = auth_service::verify_token(&config, &token).unwrap();
    assert_eq!(claims.sub, "user-123");
    assert_eq!(claims.role, "user");
}

#[test]
fn verify_token_fails_with_wrong_secret() {
    let config = test_config();
    let token = auth_service::create_token(&config, "user-1", "user").unwrap();

    let bad_config = AuthConfig {
        jwt_secret: "wrong-secret".to_string(),
        jwt_expiry_hours: 24,
    };
    assert!(auth_service::verify_token(&bad_config, &token).is_err());
}

#[test]
fn verify_token_fails_with_garbage() {
    let config = test_config();
    assert!(auth_service::verify_token(&config, "not.a.token").is_err());
}

#[test]
fn token_with_past_expiry_is_rejected() {
    use chrono::Utc;
    use jsonwebtoken::{EncodingKey, Header, encode};
    use storm_api::models::user::Claims;

    let config = test_config();
    let now = Utc::now();
    let claims = Claims {
        sub: "user-exp".to_string(),
        iat: (now - chrono::Duration::hours(2)).timestamp(),
        exp: (now - chrono::Duration::hours(1)).timestamp(),
        role: "user".to_string(),
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
    .unwrap();
    let result = auth_service::verify_token(&config, &token);
    assert!(result.is_err());
}

#[test]
fn token_preserves_role_agent() {
    let config = test_config();
    let token = auth_service::create_token(&config, "agent-42", "agent").unwrap();
    let claims = auth_service::verify_token(&config, &token).unwrap();
    assert_eq!(claims.sub, "agent-42");
    assert_eq!(claims.role, "agent");
}

#[test]
fn verify_token_with_empty_string() {
    let config = test_config();
    assert!(auth_service::verify_token(&config, "").is_err());
}
