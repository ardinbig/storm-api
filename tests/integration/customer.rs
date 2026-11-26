use crate::common::{
    body_to_value, create_test_app, create_test_app_with_token, register_and_login, seed_card,
    test_config,
};
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::json;
use sqlx::PgPool;
use tower_service::Service;
use uuid::Uuid;

#[sqlx::test]
async fn register_customer_via_endpoint(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    seed_card(&pool, "NFC-REG-001").await;
    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/customers")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "card_id": "NFC-REG-001",
                        "name": "Jane Doe",
                        "last_name": "Doe",
                        "first_name": "Jane",
                        "phone": "08111111",
                        "password": "pass123",
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[sqlx::test]
async fn list_customers_empty(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/customers")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert!(body.as_array().unwrap().is_empty());
}

#[sqlx::test]
async fn get_customer_not_found(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let id = Uuid::new_v4();
    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/customers/{id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn get_customer_by_card_not_found(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/customers/by-card/NONEXISTENT")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn update_and_delete_customer(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;

    let customer_id = Uuid::new_v4();
    let card_id = "CARD-UPDATE-001";
    seed_card(&pool, card_id).await;
    sqlx::query(
        "INSERT INTO customers (id, client_code, first_name, last_name, phone, card_id)
         VALUES ($1, 'CUST-001', 'Old Firstname', 'Old Lastname', '000', $2)",
    )
    .bind(customer_id)
    .bind(card_id)
    .execute(&pool)
    .await
    .unwrap();

    let mut app = create_test_app(pool);

    // Get
    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/customers/{customer_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Update
    let resp = app
        .call(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/customers/{customer_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "first_name": "New Firstname",
                        "last_name": "New Lastname"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["first_name"], "New Firstname");
    assert_eq!(body["last_name"], "New Lastname");

    // Delete
    let resp = app
        .call(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/customers/{customer_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[sqlx::test]
async fn update_customer_not_found(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let id = Uuid::new_v4();
    let resp = app
        .call(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/customers/{id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "first_name": "X", "last_name": "Y" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn delete_customer_not_found(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let id = Uuid::new_v4();
    let resp = app
        .call(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/customers/{id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn get_customer_by_card_success(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;

    let card_nfc = "NFC-BYCARD-001";
    seed_card(&pool, card_nfc).await;
    sqlx::query(
        "INSERT INTO customers (id, client_code, first_name, last_name, card_id)
         VALUES ($1, 'CC-001', 'ByCard', 'Customer', $2)",
    )
    .bind(Uuid::new_v4())
    .bind(card_nfc)
    .execute(&pool)
    .await
    .unwrap();

    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/customers/by-card/{card_nfc}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["client_code"], "CC-001");
}

#[sqlx::test]
async fn partial_update_preserves_unchanged_fields(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;

    let customer_id = Uuid::new_v4();
    let card_id = "CARD-PARTIAL-001";
    seed_card(&pool, card_id).await;
    sqlx::query(
        "INSERT INTO customers (id, client_code, phone, first_name, last_name, card_id)
         VALUES ($1, 'PARTIAL-001', '0800000', 'OrigFirst', 'OrigLast', $2)",
    )
    .bind(customer_id)
    .bind(card_id)
    .execute(&pool)
    .await
    .unwrap();

    let mut app = create_test_app(pool);

    let resp = app
        .call(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/customers/{customer_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"first_name": "OrigFirst", "last_name": "OrigLast"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["phone"], "0800000");
    assert_eq!(body["first_name"], "OrigFirst");
    assert_eq!(body["last_name"], "OrigLast");
}

async fn get_customer_page(
    app: &mut axum::Router,
    query: &str,
    role: Option<&str>,
) -> (StatusCode, serde_json::Value) {
    let path = format!("/api/v1/customers/paginated{query}");
    let mut request = Request::builder().uri(path);
    if let Some(role) = role {
        let token = storm_api::services::auth_service::create_token(
            &test_config(),
            &Uuid::nil().to_string(),
            role,
        )
        .unwrap();
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = app
        .call(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let value = if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::NOT_FOUND) {
        serde_json::Value::Null
    } else {
        body_to_value(response.into_body()).await
    };
    (status, value)
}

#[sqlx::test]
async fn paginated_customers_keeps_tied_names_ordered_across_pages(pool: PgPool) {
    for number in (1..=12).rev() {
        let nfc = format!("PAGE-{number}");
        seed_card(&pool, &nfc).await;
        sqlx::query(
            "INSERT INTO customers (id, client_code, first_name, last_name, card_id, address)
                     VALUES ($1, $2, 'Alex', 'Same', $2, 'not a list field')",
        )
        .bind(Uuid::from_u128(number))
        .bind(&nfc)
        .execute(&pool)
        .await
        .unwrap();
    }
    let mut app = create_test_app(pool);
    for (query, page, expected) in [("", 1, 1..=10), ("?page=2", 2, 11..=12)] {
        let (status, body) = get_customer_page(&mut app, query, Some("admin")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["page"], page);
        assert_eq!(body["total_pages"], 2);
        assert_eq!(body["page_size"], 10);
        assert_eq!(body["total_items"], 12);
        assert_eq!(body["has_next_page"], page == 1);
        assert_eq!(body["has_prev_page"], page == 2);
        assert_eq!(body["remaining_items"], if page == 1 { 2 } else { 0 });
        let rows = body["data"].as_array().unwrap();
        let ids = rows
            .iter()
            .map(|row| {
                assert!(row.get("address").is_none());
                assert_eq!(row["first_name"], "Alex");
                Uuid::parse_str(row["id"].as_str().unwrap())
                    .unwrap()
                    .as_u128()
            })
            .collect::<Vec<_>>();
        assert_eq!(ids, expected.collect::<Vec<_>>());
    }
    let (status, body) = get_customer_page(&mut app, "?page=3", Some("admin")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"], json!([]));
    assert_eq!(body["page"], 3);
    assert_eq!(body["total_pages"], 2);
    assert_eq!(body["total_items"], 12);
    assert_eq!(body["has_next_page"], false);
    assert_eq!(body["remaining_items"], 0);
}

#[sqlx::test]
async fn paginated_customers_documents_empty_page_and_rejects_invalid_page(pool: PgPool) {
    let mut app = create_test_app(pool);
    let (status, body) = get_customer_page(&mut app, "?page=1", Some("admin")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({"data":[],"page":1,"page_size":10,"total_items":0,"total_pages":1,
        "has_next_page":false,"has_prev_page":false,"remaining_items":0})
    );
    for query in [
        "?page=0",
        "?page=-1",
        "?page=invalid",
        "?page=4294967296",
        "?page=1&page=2",
    ] {
        let (status, body) = get_customer_page(&mut app, query, Some("admin")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}");
        assert_eq!(body["code"], 400);
    }
}

#[sqlx::test]
async fn paginated_customers_requires_admin(pool: PgPool) {
    let mut app = create_test_app(pool);
    assert_eq!(
        get_customer_page(&mut app, "", None).await.0,
        StatusCode::UNAUTHORIZED
    );
    for role in ["agent", "user"] {
        let (status, body) = get_customer_page(&mut app, "", Some(role)).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["code"], 403);
    }
}
