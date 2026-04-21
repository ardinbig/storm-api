use crate::common::{
    body_to_value, create_test_app, create_test_app_with_token, register_and_login, seed_agent,
    seed_agent_with_station, seed_card_with_customer, test_config,
};
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::json;
use sqlx::PgPool;
use storm_api::{
    models::{consumption::CreateConsumptionRequest, pagination::ConsumptionQuery},
    services::consumption_service,
};
use tower_service::Service;
use uuid::Uuid;

#[sqlx::test]
async fn create_and_list_consumptions(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let client_code = seed_card_with_customer(&pool).await;
    let mut app = create_test_app(pool);

    // Create
    let resp = app
        .call(
            Request::builder()
                .method("POST")
                .uri("/api/v1/consumptions")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "date": "2025-01-15T10:00:00Z",
                        "client_ref": client_code,
                        "consumption_type": "Diesel",
                        "quantity": 50.0,
                        "price": 2500.0,
                        "username": "test.user",
                        "is_online": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Paginated list — body is now an object with `data` array
    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/consumptions")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert!(!body["data"].as_array().unwrap().is_empty());
    assert_eq!(body["total_items"], 1);

    // Deprecated by-client list (still returns flat array)
    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/consumptions/by-client/{client_code}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[sqlx::test]
async fn list_by_client_empty(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/consumptions/by-client/NOBODY")
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

// Paginated list_consumptions
// ===========================

/// Empty DB → consumptions returns paginated shape.
#[sqlx::test]
async fn list_consumptions_empty_returns_paginated_shape(pool: PgPool) {
    let (mut app, token) = create_test_app_with_token(pool).await;

    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/consumptions")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert!(body["data"].as_array().unwrap().is_empty());
    assert_eq!(body["page"], 1);
    assert_eq!(body["page_size"], 10);
    assert_eq!(body["total_items"], 0);
    assert_eq!(body["total_pages"], 1);
    assert_eq!(body["has_next_page"], false);
    assert_eq!(body["has_prev_page"], false);
    assert_eq!(body["remaining_items"], 0);
}

/// `?agent=` filters consumptions by operator username.
#[sqlx::test]
async fn list_consumptions_filter_by_agent(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;
    let client_code = seed_card_with_customer(&pool).await;
    let agent_ref = "FILT-AGENT-001";
    seed_agent(&pool, agent_ref, "Filt Agent", "pw", 0.0).await;

    let mut app = create_test_app(pool);

    // Create one consumption by the agent
    app.call(
        Request::builder()
            .method("POST")
            .uri("/api/v1/consumptions")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "date": "2025-01-15T10:00:00Z",
                    "client_ref": client_code,
                    "consumption_type": "Diesel",
                    "quantity": 10.0,
                    "price": 500.0,
                    "username": agent_ref,
                    "is_online": true
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();

    // Filter by that agent
    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/consumptions?agent={agent_ref}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["total_items"], 1);
    assert_eq!(body["data"][0]["username"], agent_ref);

    // Unknown agent → 0 results
    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/consumptions?agent=NOBODY")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["total_items"], 0);
}

/// `?station=` filters consumptions by the station the operator belongs to.
#[sqlx::test]
async fn list_consumptions_filter_by_station(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;

    let station_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, name, username, password) VALUES ($1, 'Sta C', 'sta_c', 'x')",
    )
    .bind(station_id)
    .execute(&pool)
    .await
    .unwrap();

    let agent_ref = "STA-CONS-001";
    seed_agent_with_station(&pool, agent_ref, "Sta Cons Agent", "pw", 0.0, station_id).await;
    let client_code = seed_card_with_customer(&pool).await;

    let mut app = create_test_app(pool);

    app.call(
        Request::builder()
            .method("POST")
            .uri("/api/v1/consumptions")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "date": "2025-01-15T10:00:00Z",
                    "client_ref": client_code,
                    "consumption_type": "Diesel",
                    "quantity": 5.0,
                    "price": 200.0,
                    "username": agent_ref,
                    "is_online": true
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();

    // Correct station
    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/consumptions?station={station_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["total_items"], 1);

    // Wrong station → 0 results
    let resp = app
        .call(
            Request::builder()
                .uri(format!("/api/v1/consumptions?station={}", Uuid::new_v4()))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["total_items"], 0);
}

/// Pagination metadata is correct when there are more than PAGE_SIZE items.
#[sqlx::test]
async fn list_consumptions_pagination(pool: PgPool) {
    let config = test_config();
    let token = register_and_login(&pool, &config).await;

    // Insert 11 records
    for _ in 0..11u32 {
        let client_code = seed_card_with_customer(&pool).await;
        sqlx::query(
            "INSERT INTO consumptions (id, client_ref, consumption_type,
             quantity, price, username, consumption_date, status)
             VALUES ($1, $2, 'Diesel', 1.0, 100.0, 'pg.agent', NOW(), 1)",
        )
        .bind(Uuid::new_v4())
        .bind(&client_code)
        .execute(&pool)
        .await
        .unwrap();
    }

    let mut app = create_test_app(pool);

    // Page 1
    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/consumptions?page=1")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["total_items"], 11);
    assert_eq!(body["total_pages"], 2);
    assert_eq!(body["data"].as_array().unwrap().len(), 10);
    assert!(body["has_next_page"].as_bool().unwrap());
    assert_eq!(body["remaining_items"], 1);

    // Page 2
    let resp = app
        .call(
            Request::builder()
                .uri("/api/v1/consumptions?page=2")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_to_value(resp.into_body()).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert!(!body["has_next_page"].as_bool().unwrap());
    assert!(body["has_prev_page"].as_bool().unwrap());
}

#[sqlx::test]
#[allow(deprecated)]
async fn service_list_returns_records(pool: PgPool) {
    let client_code = seed_card_with_customer(&pool).await;

    sqlx::query(
        "INSERT INTO consumptions (id, client_ref, consumption_type,
         quantity, price, username, consumption_date, status)
         VALUES ($1, $2, 'Diesel', 3.0, 700.0, 'svc.agent', NOW(), 1)",
    )
    .bind(Uuid::new_v4())
    .bind(&client_code)
    .execute(&pool)
    .await
    .unwrap();

    let items = consumption_service::list(&pool).await.unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].client_ref, client_code);
    assert_eq!(items[0].consumption_type, "Diesel");
}

#[sqlx::test]
#[allow(deprecated)]
async fn service_list_by_client_filters_records(pool: PgPool) {
    let target_client = seed_card_with_customer(&pool).await;
    let other_client = seed_card_with_customer(&pool).await;

    sqlx::query(
        "INSERT INTO consumptions (id, client_ref, consumption_type,
         quantity, price, username, consumption_date, status)
         VALUES ($1, $2, 'Diesel', 2.0, 500.0, 'svc.agent', NOW(), 1)",
    )
    .bind(Uuid::new_v4())
    .bind(&target_client)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO consumptions (id, client_ref, consumption_type,
         quantity, price, username, consumption_date, status)
         VALUES ($1, $2, 'Essence', 4.0, 900.0, 'svc.agent', NOW(), 1)",
    )
    .bind(Uuid::new_v4())
    .bind(&other_client)
    .execute(&pool)
    .await
    .unwrap();

    let items = consumption_service::list_by_client(&pool, &target_client)
        .await
        .unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].client_ref, target_client);
}

#[sqlx::test]
async fn service_create_and_paginated_list_clamps_zero_page(pool: PgPool) {
    let client_code = seed_card_with_customer(&pool).await;

    let input = CreateConsumptionRequest {
        date: "2025-01-15T10:00:00Z".into(),
        client_ref: client_code.clone(),
        consumption_type: "Diesel".into(),
        quantity: 6.0,
        price: 250.0,
        username: "svc.operator".into(),
        is_online: true,
    };

    consumption_service::create(&pool, &input).await.unwrap();

    let page0 = ConsumptionQuery {
        page: Some(0),
        ..Default::default()
    };
    let result = consumption_service::list_paginated(&pool, &page0)
        .await
        .unwrap();

    assert_eq!(result.page, 1);
    assert_eq!(result.total_items, 1);
    assert_eq!(result.data.len(), 1);
    assert_eq!(result.data[0].client_ref, client_code);
}
