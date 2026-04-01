use sqlx::PgPool;
use uuid::Uuid;

/// Insert a card into `cards`.
pub async fn seed_card(pool: &PgPool, nfc: &str) {
    sqlx::query("INSERT INTO cards (id, card_id) VALUES ($1, $2)")
        .bind(Uuid::new_v4())
        .bind(nfc)
        .execute(pool)
        .await
        .unwrap();
}

/// Insert a commission percentage into `commissions`.
async fn seed_commission(pool: &PgPool, percentage: f64) {
    sqlx::query("INSERT INTO commissions (id, percentage) VALUES ($1, $2)")
        .bind(Uuid::new_v4())
        .bind(percentage)
        .execute(pool)
        .await
        .unwrap();
}

/// Insert a customer into `customers`.
pub async fn seed_customer(pool: &PgPool, client_code: &str, first_name: &str, nfc: &str) {
    sqlx::query(
        "INSERT INTO customers (id, client_code, first_name, last_name, card_id)
         VALUES ($1, $2, $3, 'Customer', $4)",
    )
    .bind(Uuid::new_v4())
    .bind(client_code)
    .bind(first_name)
    .bind(nfc)
    .execute(pool)
    .await
    .unwrap();
}

/// Insert an agent account with a hashed password and initial balance.
pub async fn seed_agent(pool: &PgPool, agent_ref: &str, name: &str, password: &str, balance: f64) {
    let hash = storm_api::services::auth_service::hash_password(password).unwrap();
    sqlx::query(
        "INSERT INTO agent_accounts (id, agent_ref, name, password, balance, currency_code)
         VALUES ($1, $2, $3, $4, $5, 'CDF')",
    )
    .bind(Uuid::new_v4())
    .bind(agent_ref)
    .bind(name)
    .bind(&hash)
    .bind(balance)
    .execute(pool)
    .await
    .unwrap();
}

/// Insert the STORM-ACCOUNT-0000 house account.
pub async fn seed_house_account(pool: &PgPool) {
    let hash = storm_api::services::auth_service::hash_password("house123").unwrap();
    sqlx::query(
        "INSERT INTO agent_accounts (id, agent_ref, name, password, balance, currency_code)
         VALUES ($1, $2, 'House Account', $3, 0, 'CDF')
         ON CONFLICT (agent_ref) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(storm_api::models::agent::HOUSE_ACCOUNT_REF)
    .bind(&hash)
    .execute(pool)
    .await
    .unwrap();
}

async fn seed_card_details(
    pool: &PgPool,
    nfc: &str,
    client_code: &str,
    password: &str,
    amount: f64,
) {
    let hash = storm_api::services::auth_service::hash_password(password).unwrap();
    sqlx::query(
        "INSERT INTO card_details (nfc_ref, client_code, password, amount)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (nfc_ref) DO UPDATE
         SET client_code = EXCLUDED.client_code,
             password = EXCLUDED.password,
             amount = EXCLUDED.amount",
    )
    .bind(nfc)
    .bind(client_code)
    .bind(&hash)
    .bind(amount)
    .execute(pool)
    .await
    .unwrap();
}

/// Parameters for [`seed_withdrawal_scenario`].
pub struct WithdrawalScenario<'a> {
    pub nfc: &'a str,
    pub client_code: &'a str,
    pub client_password: &'a str,
    pub client_balance: f64,
    pub agent_ref: &'a str,
    pub agent_password: &'a str,
    pub agent_balance: f64,
    pub commission_pct: f64,
}

/// Seed a unique card + customer and return the generated `client_code`.
///
/// Useful for consumption/transaction tests that need a valid cardholder
/// without caring about the exact identifiers.
pub async fn seed_card_with_customer(pool: &PgPool) -> String {
    use uuid::Uuid;
    let nfc = format!("NFC-{}", &Uuid::new_v4().to_string()[..8]);
    let client_code = format!("CC-{}", &Uuid::new_v4().to_string()[..8]);
    seed_card(pool, &nfc).await;
    sqlx::query(
        "INSERT INTO customers (id, client_code, first_name, last_name, card_id)
         VALUES ($1, $2, 'Test', 'Customer', $3)",
    )
    .bind(Uuid::new_v4())
    .bind(&client_code)
    .bind(&nfc)
    .execute(pool)
    .await
    .unwrap();
    client_code
}

/// Seed everything needed for a withdrawal test:
/// card -> customer -> card_details -> agent -> house account -> commission.
pub async fn seed_withdrawal_scenario(pool: &PgPool, s: &WithdrawalScenario<'_>) {
    seed_card(pool, s.nfc).await;
    seed_customer(pool, s.client_code, "Test Customer", s.nfc).await;
    seed_card_details(
        pool,
        s.nfc,
        s.client_code,
        s.client_password,
        s.client_balance,
    )
    .await;
    seed_agent(
        pool,
        s.agent_ref,
        "Test Agent",
        s.agent_password,
        s.agent_balance,
    )
    .await;
    seed_house_account(pool).await;
    seed_commission(pool, s.commission_pct).await;
}
