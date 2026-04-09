//! Agent account business logic: CRUD, authentication, customer registration,
//! balance checks, and transaction history.
//!
//! Agents are field operators who perform customer withdrawals.  This module
//! handles the full agent lifecycle including login, password changes, and
//! the agent-initiated customer onboarding flow.

use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::agent::{
        Agent, AgentAuthResponse, AgentHistoryRow, AgentInfo, AgentLoginRequest,
        AgentRegisterCustomerRequest, CreateAgentRequest, DEFAULT_NETWORK, HOUSE_ACCOUNT_REF,
        UpdateAgentPasswordRequest, UpdateAgentRequest,
    },
    models::card::CardDetail,
    services::{auth_service, card_service},
    state::app_state::{AuthConfig, RedisPool},
    utils::{cache, password},
};

/// SQL column list reused across agent queries.
const AGENT_COLUMNS: &str = "id, agent_ref, name, password, balance, currency_code";

/// Precomputed queries
const SELECT_ALL: &str = "SELECT id, agent_ref, name, password, balance, currency_code FROM agent_accounts ORDER BY agent_ref";
const SELECT_BY_ID: &str = "SELECT id, agent_ref, name, password, balance, currency_code FROM agent_accounts WHERE id = $1";
const SELECT_BY_REF: &str = "SELECT id, agent_ref, name, password, balance, currency_code FROM agent_accounts WHERE agent_ref = $1";

// Private helpers
// ===============

/// Looks up an agent by `agent_ref`, verifies the supplied password, and
/// returns the full [`Agent`] row.
///
/// # Errors
///
/// Returns [`AppError::Unauthorized`] if the account does not exist, has no
/// password set, or the password does not match.
async fn fetch_and_verify(
    pool: &PgPool,
    agent_ref: &str,
    plain_password: &str,
) -> Result<Agent, AppError> {
    let agent = sqlx::query_as::<_, Agent>(SELECT_BY_REF)
        .bind(agent_ref)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let stored = agent.password.as_deref().ok_or(AppError::Unauthorized)?;
    if !auth_service::verify_password(plain_password, stored) {
        return Err(AppError::Unauthorized);
    }

    Ok(agent)
}

/// Returns all agent accounts ordered by `agent_ref`.
///
/// # Errors
///
/// Returns [`AppError::Database`] on query failure.
pub async fn list(pool: &PgPool) -> Result<Vec<AgentInfo>, AppError> {
    let agents = sqlx::query_as::<_, Agent>(SELECT_ALL)
        .fetch_all(pool)
        .await?;

    Ok(agents.into_iter().map(AgentInfo::from).collect())
}

/// Retrieves a single agent by primary key.
///
/// # Errors
///
/// - [`AppError::NotFound`] — no agent with this `id`.
/// - [`AppError::Database`] — query failure.
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<AgentInfo, AppError> {
    let agent = sqlx::query_as::<_, Agent>(SELECT_BY_ID)
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Agent not found".into()))?;

    Ok(AgentInfo::from(agent))
}

/// Creates a new agent account.
///
/// The password is Argon2-hashed before storage.  The initial balance is
/// set to `0`.  If `currency_code` is omitted it defaults to `"CDF"`.
///
/// # Errors
///
/// - [`AppError::Internal`] — password hashing failure.
/// - [`AppError::Database`] — duplicate `agent_ref` or other constraint
///   violation.
pub async fn create(pool: &PgPool, input: &CreateAgentRequest) -> Result<AgentInfo, AppError> {
    let hashed = password::hash(&input.password)?;
    let id = Uuid::new_v4();
    let currency = input.currency_code.as_deref().unwrap_or("CDF");

    let agent = sqlx::query_as::<_, Agent>(&format!(
        "INSERT INTO agent_accounts ({AGENT_COLUMNS})
         VALUES ($1, $2, $3, $4, 0, $5)
         RETURNING {AGENT_COLUMNS}"
    ))
    .bind(id)
    .bind(&input.agent_ref)
    .bind(&input.name)
    .bind(&hashed)
    .bind(currency)
    .fetch_one(pool)
    .await?;

    Ok(AgentInfo::from(agent))
}

/// Partially updates an agent account.
///
/// Only non-`None` fields in `input` are applied; existing values are
/// preserved via `COALESCE`.
///
/// # Errors
///
/// - [`AppError::NotFound`] — no agent with this `id`.
/// - [`AppError::Database`] — query failure.
pub async fn update(
    pool: &PgPool,
    id: Uuid,
    input: &UpdateAgentRequest,
) -> Result<AgentInfo, AppError> {
    let agent = sqlx::query_as::<_, Agent>(&format!(
        "UPDATE agent_accounts SET
            name = COALESCE($2, name),
            currency_code = COALESCE($3, currency_code)
         WHERE id = $1
         RETURNING {AGENT_COLUMNS}"
    ))
    .bind(id)
    .bind(input.name.as_ref())
    .bind(input.currency_code.as_ref())
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Agent not found".into()))?;

    Ok(AgentInfo::from(agent))
}

/// Deletes an agent account by primary key.
///
/// The house account ([`HOUSE_ACCOUNT_REF`]) cannot be deleted.
///
/// # Errors
///
/// - [`AppError::NotFound`] — no agent with this `id`.
/// - [`AppError::BadRequest`] — attempted deletion of the house account.
/// - [`AppError::Database`] — query failure.
pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    let agent_ref: Option<String> =
        sqlx::query_scalar("SELECT agent_ref FROM agent_accounts WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;

    match agent_ref.as_deref() {
        None => return Err(AppError::NotFound("Agent not found".into())),
        Some(HOUSE_ACCOUNT_REF) => {
            return Err(AppError::BadRequest(
                "Cannot delete the house commission account".into(),
            ));
        }
        _ => {}
    }

    sqlx::query("DELETE FROM agent_accounts WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Authenticates an agent and returns a JWT with `role = "agent"`.
///
/// # Errors
///
/// - [`AppError::Unauthorized`] — bad credentials.
/// - [`AppError::Internal`] — JWT signing failure.
pub async fn authenticate(
    pool: &PgPool,
    config: &AuthConfig,
    input: &AgentLoginRequest,
) -> Result<AgentAuthResponse, AppError> {
    let agent = fetch_and_verify(pool, &input.username, &input.password).await?;
    let token = auth_service::create_token(config, &agent.id.to_string(), "agent")
        .map_err(|_| AppError::Internal)?;

    Ok(AgentAuthResponse {
        token,
        agent: AgentInfo::from(agent),
    })
}

/// Looks up a card's balance details by NFC reference.
///
/// # Errors
///
/// - [`AppError::NotFound`] — no `card_details` row for this NFC ref.
/// - [`AppError::Database`] — query failure.
pub async fn check_balance(
    pool: &PgPool,
    card_id: &str,
    redis: &RedisPool,
) -> Result<CardDetail, AppError> {
    card_service::get_detail_by_nfc(pool, card_id, redis)
        .await?
        .ok_or_else(|| AppError::NotFound("Card not found".into()))
}

/// Returns the transaction history for a given agent, with client names
/// resolved from the `customers` table.
///
/// # Errors
///
/// Returns [`AppError::Database`] on query failure.
pub async fn history(pool: &PgPool, agent_id: &str) -> Result<Vec<AgentHistoryRow>, AppError> {
    Ok(sqlx::query_as::<_, AgentHistoryRow>(
        "SELECT t.id, t.date, t.transaction_type, t.currency_code, t.amount,
                CONCAT(c.first_name, ' ', c.last_name) AS client
         FROM transactions t
         JOIN customers c ON c.card_id = t.client_account
         WHERE t.agent_account = $1
         ORDER BY t.date DESC",
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await?)
}

/// Agent-initiated customer registration (transactional).
///
/// 1. Checks that the NFC card is not already assigned.
/// 2. Generates a unique `client_code`.
/// 3. Inserts a `customers` row (category defaults to `"Motorbike"`).
/// 4. Upserts a `card_details` row with a default password of `"1234"`.
///
/// The entire operation runs in a single database transaction.
///
/// # Errors
///
/// - [`AppError::Conflict`] — card is already assigned to another customer.
/// - [`AppError::Internal`] — password hashing failure.
/// - [`AppError::Database`] — constraint violation or query failure.
pub async fn register_customer(
    pool: &PgPool,
    input: &AgentRegisterCustomerRequest,
    redis: &RedisPool,
) -> Result<(), AppError> {
    if card_service::get_detail_by_nfc(pool, &input.card_ref, redis)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict(
            "Card is already assigned to a customer".into(),
        ));
    }

    let mut tx = pool.begin().await?;

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM customers")
        .fetch_one(&mut *tx)
        .await?;
    let client_code = format!(
        "Storm-{}{}-{}",
        chrono::Utc::now().format("%m"),
        count.0 + 1,
        rand::random::<u16>() % 5000
    );

    let default_password_hash = password::hash("1234")?;
    let customer_id = Uuid::new_v4();

    sqlx::query(&format!(
        "INSERT INTO customers \
             (id, client_code, first_name, middle_name, last_name, address, \
              networks, phone, category_ref, card_id, gender, marital_status, affiliation) \
         VALUES ($1, $2, $3, $4, $5, $6, '{DEFAULT_NETWORK}', $7, \
                 (SELECT id FROM categories WHERE name = 'Motorbike' LIMIT 1), \
                 $8, $9, $10, $11)"
    ))
    .bind(customer_id)
    .bind(&client_code)
    .bind(&input.first_name)
    .bind(&input.middle_name)
    .bind(&input.last_name)
    .bind(&input.address)
    .bind(&input.phone)
    .bind(&input.card_ref)
    .bind(&input.gender)
    .bind(&input.marital_status)
    .bind(&input.affiliation)
    .execute(&mut *tx)
    .await?;

    sqlx::query(&format!(
        "INSERT INTO card_details (nfc_ref, client_code, password, network) \
         VALUES ($1, $2, $3, '{DEFAULT_NETWORK}') \
         ON CONFLICT (nfc_ref) DO UPDATE SET \
             client_code = EXCLUDED.client_code, \
             password = EXCLUDED.password, \
             network = EXCLUDED.network"
    ))
    .bind(&input.card_ref)
    .bind(&client_code)
    .bind(&default_password_hash)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    // Invalidate cached card detail for this NFC ref
    cache::del(redis, &cache::card_detail_key(&input.card_ref)).await;

    Ok(())
}

/// Changes an agent's password after verifying the current one.
///
/// # Errors
///
/// - [`AppError::Unauthorized`] — current password mismatch.
/// - [`AppError::Internal`] — hashing failure.
/// - [`AppError::Database`] — query failure.
pub async fn update_password(
    pool: &PgPool,
    input: &UpdateAgentPasswordRequest,
) -> Result<(), AppError> {
    fetch_and_verify(pool, &input.agent_ref, &input.last_password).await?;

    let hashed = password::hash(&input.new_password)?;
    sqlx::query("UPDATE agent_accounts SET password = $1 WHERE agent_ref = $2")
        .bind(&hashed)
        .bind(&input.agent_ref)
        .execute(pool)
        .await?;

    Ok(())
}
