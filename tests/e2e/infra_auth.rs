use crate::common::{JWT_SECRET, TestApp};
use serde_json::json;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn e2e_infra_endpoints() {
    let app = TestApp::spawn().await;

    let resp = app.get("/health").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await.unwrap(), "OK");

    let resp = app.get("/ready").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await.unwrap(), "ready");

    let resp = app.get("/metrics").await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["requests"].as_u64().unwrap() >= 1);

    let resp = app.get("/nonexistent/route").await;
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
#[serial]
async fn e2e_register_and_login() {
    let app = TestApp::spawn().await;

    let resp = app
        .post_json(
            "/api/v1/auth/register",
            &json!({
                "name": "E2E User",
                "email": "e2e@example.com",
                "username": "e2euser",
                "password": "strong.pass"
            }),
        )
        .await;
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["username"], "e2euser");

    let resp = app
        .post_json(
            "/api/v1/auth/login",
            &json!({"username": "e2euser", "password": "strong.pass"}),
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["token"].is_string());
}

#[tokio::test]
#[serial]
async fn e2e_auth_middleware() {
    let app = TestApp::spawn().await;

    let resp = app.get("/api/v1/users/me").await;
    assert_eq!(resp.status(), 401);

    let token = app.token().await;
    let resp = app.get_auth("/api/v1/users/me", &token).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["role"], "user");
}

#[tokio::test]
#[serial]
async fn e2e_malformed_requests_rejected() {
    let app = TestApp::spawn().await;

    let resp = app
        .client
        .post(format!("{}/api/v1/auth/register", app.addr))
        .header("Content-Type", "application/json")
        .body("{invalid json")
        .send()
        .await
        .unwrap();
    assert!(resp.status() == 400 || resp.status() == 422);

    let resp = app
        .client
        .post(format!("{}/api/v1/auth/login", app.addr))
        .header("Content-Type", "application/json")
        .body("")
        .send()
        .await
        .unwrap();
    assert!(resp.status() == 400 || resp.status() == 422);

    let resp = app
        .post_json("/api/v1/auth/register", &json!({"name": "Only Name"}))
        .await;
    assert!(resp.status() == 400 || resp.status() == 422);
}

#[tokio::test]
#[serial]
async fn e2e_expired_jwt_rejected() {
    use chrono::Utc;
    use jsonwebtoken::{EncodingKey, Header, encode};
    use storm_api::models::user::Claims;

    let app = TestApp::spawn().await;

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
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .unwrap();

    let resp = app.get_auth("/api/v1/users/me", &token).await;
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
#[serial]
async fn e2e_logout_blocklists_token() {
    let app = TestApp::spawn().await;
    let token = app.token().await;

    let resp = app.get_auth("/api/v1/users/me", &token).await;
    assert_eq!(resp.status(), 200);

    let resp = app.post_auth("/api/v1/auth/logout", &token).await;
    assert_eq!(resp.status(), 200);

    let resp = app.get_auth("/api/v1/users/me", &token).await;
    assert_eq!(resp.status(), 401);
}
