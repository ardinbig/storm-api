use sqlx::PgPool;
use storm_api::{models, services::user_service, state::app_state::AuthConfig};
use uuid::Uuid;

/// Register a user with a unique username and return the JWT token.
pub async fn register_and_login(pool: &PgPool, config: &AuthConfig) -> String {
    let unique = &Uuid::new_v4().to_string()[..8];
    let username = format!("test.user-{unique}");

    user_service::register(
        pool,
        &models::user::RegisterRequest {
            name: "Test User".into(),
            email: Some(format!("{username}@example.com")),
            username: username.clone(),
            password: "password123".into(),
        },
    )
    .await
    .unwrap();

    user_service::authenticate(pool, config, &username, "password123")
        .await
        .unwrap()
        .token
}
