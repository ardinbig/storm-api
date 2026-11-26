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

#[tokio::test]
#[serial]
async fn e2e_swagger_documents_live_routes() {
    let app = TestApp::spawn().await;
    let ui = app.get("/api/v1/docs/").await;
    assert_eq!(ui.status(), 200);
    assert!(ui.text().await.unwrap().contains("swagger-ui-bundle.js"));
    let initializer = app.get("/api/v1/docs/swagger-initializer.js").await;
    assert_eq!(initializer.status(), 200);
    assert!(
        initializer
            .text()
            .await
            .unwrap()
            .contains("/api-doc/openapi.json")
    );
    assert_eq!(
        app.get("/api/v1/docs/swagger-ui-bundle.js").await.status(),
        200
    );

    let response = app.get("/api-doc/openapi.json").await;
    assert_eq!(response.status(), 200);
    let doc: serde_json::Value = response.json().await.unwrap();
    assert_eq!(doc["info"]["version"], "0.1.6");
    assert_eq!(
        doc["components"]["securitySchemes"]["bearer"]["scheme"],
        "bearer"
    );

    let mut pending = vec![&doc];
    while let Some(value) = pending.pop() {
        if let Some(reference) = value.get("$ref").and_then(|v| v.as_str()) {
            let pointer = reference.strip_prefix('#').expect("local schema reference");
            assert!(
                doc.pointer(pointer).is_some(),
                "unresolved schema {reference}"
            );
        }
        if let Some(object) = value.as_object() {
            pending.extend(object.values());
        }
        if let Some(array) = value.as_array() {
            pending.extend(array);
        }
    }

    let paths = doc["paths"].as_object().unwrap();
    let page = &paths["/api/v1/customers/paginated"]["get"];
    assert_eq!(page["parameters"][0]["name"], "page");
    assert_eq!(page["parameters"][0]["in"], "query");
    assert_eq!(
        page["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/PaginatedCustomerResponse"
    );
    let mut ops = 0;
    for (path, methods) in paths {
        for (method, spec) in methods.as_object().unwrap() {
            if !matches!(method.as_str(), "get" | "post" | "put" | "delete" | "patch") {
                continue;
            }
            ops += 1;
            if let Some(params) = spec["parameters"].as_array() {
                for param in params.iter().filter(|p| p["in"] == "path") {
                    let name = param["name"].as_str().unwrap();
                    assert!(
                        path.contains(&format!("{{{name}}}")),
                        "{method} {path}: undeclared path parameter {name}"
                    );
                }
            }
            let mut uri = path.clone();
            for segment in path.split('/') {
                if let Some(param) = segment.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
                    let params = spec["parameters"].as_array().expect("path parameters");
                    assert!(
                        params.iter().any(|p| p["name"] == param
                            && p["in"] == "path"
                            && p["required"] == true),
                        "{method} {path} has no required parameter {param}"
                    );
                    uri = uri.replace(segment, "00000000-0000-0000-0000-000000000000");
                }
            }
            let public = matches!(
                path.as_str(),
                "/health"
                    | "/ready"
                    | "/metrics"
                    | "/api/v1/auth/login"
                    | "/api/v1/auth/register"
                    | "/api/v1/agents/login"
            );
            assert_eq!(
                spec["security"][0].get("bearer").is_some(),
                !public,
                "{method} {path}"
            );
            let response = app
                .client
                .request(
                    reqwest::Method::from_bytes(method.to_ascii_uppercase().as_bytes()).unwrap(),
                    format!("{}{uri}", app.addr),
                )
                .header("Content-Type", "application/json")
                .body("{")
                .send()
                .await
                .unwrap();
            let expected = if !public {
                401
            } else if method == "get" {
                200
            } else {
                400
            };
            assert_eq!(response.status().as_u16(), expected, "{method} {path}");
        }
    }
    println!("Verified Swagger dispatch and auth metadata for {ops} operations");
    let admin = storm_api::services::auth_service::create_token(
        &crate::common::test_config(),
        &uuid::Uuid::nil().to_string(),
        "admin",
    )
    .unwrap();
    let response = app
        .get_auth("/api/v1/customers/paginated?page=1", &admin)
        .await;
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["data"], json!([]));
}
