use crate::common::{body_to_value, register_and_login, test_config, test_state};
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::json;
use sqlx::PgPool;
use storm_api::app::create_app;
use tower_service::Service;

#[sqlx::test]
async fn register_user(pool: PgPool) {
    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "name": "Alice",
                        "email": "alice@example.com",
                        "username": "alice",
                        "password": "pass123"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["username"], "alice");
    assert_eq!(body["name"], "Alice");
}

#[sqlx::test]
async fn login_user(pool: PgPool) {
    storm_api::services::user_service::register(
        &pool,
        &storm_api::models::user::RegisterRequest {
            name: "Bob".into(),
            email: None,
            username: "bob".into(),
            password: "bob.pass".into(),
        },
    )
    .await
    .unwrap();

    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"username": "bob", "password": "bob.pass"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert!(body["token"].is_string());
    assert_eq!(body["user"]["username"], "bob");
}

#[sqlx::test]
async fn login_with_wrong_password(pool: PgPool) {
    storm_api::services::user_service::register(
        &pool,
        &storm_api::models::user::RegisterRequest {
            name: "Eve".into(),
            email: None,
            username: "eve".into(),
            password: "correct".into(),
        },
    )
    .await
    .unwrap();

    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"username": "eve", "password": "wrong"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn login_nonexistent_user(pool: PgPool) {
    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"username": "ghost", "password": "x"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn register_duplicate_username_returns_error(pool: PgPool) {
    let state = test_state(pool.clone());
    let mut app = create_app(state);

    let body = json!({
        "name": "First",
        "email": "first@example.com",
        "username": "duplicate",
        "password": "pass123"
    })
    .to_string();

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        resp.status() == StatusCode::CONFLICT || resp.status() == StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[sqlx::test]
async fn logout_returns_ok_with_valid_token_without_redis(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[sqlx::test]
async fn logout_rejects_missing_authorization_header(pool: PgPool) {
    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn logout_rejects_malformed_authorization_header(pool: PgPool) {
    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .header(header::AUTHORIZATION, "Token abc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn logout_rejects_invalid_bearer_token(pool: PgPool) {
    let state = test_state(pool);
    let mut app = create_app(state);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .header(header::AUTHORIZATION, "Bearer invalid.token.here")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn logout_does_not_revoke_token_when_redis_disabled(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let state = test_state(pool);
    let mut app = create_app(state);

    let logout = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(logout.status(), StatusCode::OK);

    let me = app
        .call(
            Request::builder()
                .method("GET")
                .uri("/api/v1/users/me")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(me.status(), StatusCode::OK);
}
