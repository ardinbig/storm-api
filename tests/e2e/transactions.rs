use crate::common::{self, TestApp};
use serde_json::json;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn e2e_withdrawal_flow() {
    let app = TestApp::spawn().await;
    let token = app.token().await;

    common::seed_withdrawal_scenario(
        &app.pool,
        &common::WithdrawalScenario {
            nfc: "NFC-E2E-WD",
            client_code: "CC-E2E-WD",
            client_password: "wd.pass",
            client_balance: 10_000.0,
            agent_ref: "AGENT-E2E-WD",
            agent_password: "agent.pw",
            agent_balance: 500.0,
            commission_pct: 5.0,
        },
    )
    .await;

    let resp = app
        .post_json_auth(
            "/api/v1/transactions/withdrawal",
            &json!({
                "client_code": "NFC-E2E-WD",
                "withdrawal_amount": 200.0,
                "client_password": "wd.pass",
                "agent_code": "AGENT-E2E-WD",
                "currency_type": "CDF"
            }),
            &token,
        )
        .await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["message"], "Withdrawal successful");
    assert_eq!(body["client_balance"], 10_000.0 - 210.0);
    assert_eq!(body["agent_balance"], 500.0 + 200.0);

    let resp = app.get_auth("/api/v1/transactions", &token).await;
    assert_eq!(resp.status(), 200);
    let transactions: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(transactions.len(), 1);
}

#[tokio::test]
#[serial]
async fn e2e_consumption_crud_flow() {
    let app = TestApp::spawn().await;
    let token = app.token().await;

    common::seed_card(&app.pool, "NFC-CONS-E2E").await;
    common::seed_customer(&app.pool, "CC-CONS-E2E", "Cons Customer", "NFC-CONS-E2E").await;

    let resp = app
        .post_json_auth(
            "/api/v1/consumptions",
            &json!({
                "date": "2025-06-15T10:00:00Z",
                "client_ref": "CC-CONS-E2E",
                "consumption_type": "Diesel",
                "quantity": 25.0,
                "price": 1850.0,
                "username": "station-op",
                "is_online": true
            }),
            &token,
        )
        .await;
    assert_eq!(resp.status(), 201);

    let resp = app.get_auth("/api/v1/consumptions", &token).await;
    assert_eq!(resp.status(), 200);
    let list: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(!list.is_empty());

    let resp = app
        .get_auth("/api/v1/consumptions/by-client/CC-CONS-E2E", &token)
        .await;
    assert_eq!(resp.status(), 200);
    let list: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["consumption_type"], "Diesel");
    assert_eq!(list[0]["quantity"], 25.0);
}

#[tokio::test]
#[serial]
async fn e2e_concurrent_withdrawals_consistent_balance() {
    let app = TestApp::spawn().await;
    let token = app.token().await;

    common::seed_withdrawal_scenario(
        &app.pool,
        &common::WithdrawalScenario {
            nfc: "NFC-CONC-E2E",
            client_code: "CC-CONC",
            client_password: "pass",
            client_balance: 10_000.0,
            agent_ref: "AGENT-CONC",
            agent_password: "agent.pw",
            agent_balance: 0.0,
            commission_pct: 5.0,
        },
    )
    .await;

    let mut handles = Vec::new();
    for _ in 0..5 {
        let client = app.client.clone();
        let addr = app.addr.clone();
        let tkn = token.clone();
        let handle = tokio::spawn(async move {
            let resp = client
                .post(format!("{addr}/api/v1/transactions/withdrawal"))
                .bearer_auth(&tkn)
                .json(&json!({
                    "client_code": "NFC-CONC-E2E",
                    "withdrawal_amount": 100.0,
                    "client_password": "pass",
                    "agent_code": "AGENT-CONC",
                    "currency_type": "CDF"
                }))
                .send()
                .await
                .unwrap();
            resp.status().as_u16()
        });
        handles.push(handle);
    }

    let mut success_count = 0u32;
    for h in handles {
        let status = h.await.unwrap();
        if status == 200 {
            success_count += 1;
        }
    }

    let card: (f64,) = sqlx::query_as("SELECT amount::FLOAT8 FROM card_details WHERE nfc_ref = $1")
        .bind("NFC-CONC-E2E")
        .fetch_one(&app.pool)
        .await
        .unwrap();

    let deduction_per_wd = 100.0 + (100.0 * 5.0 / 100.0);
    let expected_balance = 10_000.0 - (success_count as f64 * deduction_per_wd);
    assert!(
        (card.0 - expected_balance).abs() < 1e-6,
        "Expected balance {expected_balance}, got {}",
        card.0
    );
    assert!(success_count > 0, "At least one withdrawal should succeed");
}
