//! NFC card business logic: CRUD, balance lookups, and password-protected
//! balance checks.
//!
//! [`get_detail_by_nfc`] is a shared helper used by both this module and
//! [`super::agent_service`] to avoid duplicating the
//! `card_details` lookup query. When a [`RedisPool`] is provided, results
//! are cached for `CARD_DETAIL_TTL` seconds.

use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::card::{BalanceResponse, Card, CardDetail, CreateCardRequest},
    services::auth_service,
    state::app_state::RedisPool,
    utils::cache,
};

/// Cache TTL for card detail lookups (5 minutes).
const CARD_DETAIL_TTL: u64 = 300;

/// Fetches a [`CardDetail`] row by NFC reference string.
///
/// When `redis` is `Some`, the result is served from cache on hit and
/// populated on miss.  Pass `None` to bypass the cache (e.g. when the
/// password hash is needed, since it is excluded from cached values).
///
/// # Errors
///
/// Returns [`AppError::Database`] on query failure.
pub async fn get_detail_by_nfc(
    pool: &PgPool,
    nfc_ref: &str,
    redis: &RedisPool,
) -> Result<Option<CardDetail>, AppError> {
    let key = cache::card_detail_key(nfc_ref);

    // Try cache first
    if let Some(cached) = cache::get::<CardDetail>(redis, &key).await {
        return Ok(Some(cached));
    }

    let card = sqlx::query_as::<_, CardDetail>(
        "SELECT id, amount::FLOAT8 AS amount, nfc_ref, client_code, password, network
         FROM card_details
         WHERE nfc_ref = $1",
    )
    .bind(nfc_ref)
    .fetch_optional(pool)
    .await?;

    // Populate cache on miss
    if let Some(ref card) = card {
        cache::set(redis, &key, card, CARD_DETAIL_TTL).await;
    }

    Ok(card)
}

/// Lists all cards ordered by `card_id`.
///
/// # Errors
///
/// Returns [`AppError::Database`] on query failure.
pub async fn list(pool: &PgPool) -> Result<Vec<Card>, AppError> {
    let cards = sqlx::query_as::<_, Card>("SELECT id, card_id, status FROM cards ORDER BY card_id")
        .fetch_all(pool)
        .await?;

    Ok(cards)
}

/// Retrieves a single card by primary key.
///
/// # Errors
///
/// - [`AppError::NotFound`] — no card with this `id`.
/// - [`AppError::Database`] — query failure.
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Card, AppError> {
    let card = sqlx::query_as::<_, Card>("SELECT id, card_id, status FROM cards WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Card not found".to_string()))?;

    Ok(card)
}

/// Creates a new NFC card master record.
///
/// A database trigger automatically creates the associated `card_details`
/// row on insert.
///
/// # Errors
///
/// Returns [`AppError::Database`] on duplicate `card_id` or other
/// constraint violation.
pub async fn create(pool: &PgPool, input: &CreateCardRequest) -> Result<Card, AppError> {
    let id = Uuid::new_v4();
    let card = sqlx::query_as::<_, Card>(
        "INSERT INTO cards (id, card_id) VALUES ($1, $2)
         RETURNING id, card_id, status",
    )
    .bind(id)
    .bind(&input.card_id)
    .fetch_one(pool)
    .await?;

    Ok(card)
}

/// Password-protected balance inquiry.
///
/// Verifies the caller-supplied plaintext password against the stored
/// Argon2 hash before returning the card balance details.
///
/// # Errors
///
/// - [`AppError::NotFound`] — no card found for the NFC reference.
/// - [`AppError::Unauthorized`] — no password set or mismatch.
/// - [`AppError::Database`] — query failure.
pub async fn check_balance(
    pool: &PgPool,
    nfc_ref: &str,
    password: &str,
) -> Result<BalanceResponse, AppError> {
    // Bypass cache — password hash is needed for verification and is
    // excluded from cached values.
    let card = get_detail_by_nfc(pool, nfc_ref, &None)
        .await?
        .ok_or_else(|| AppError::NotFound("Card not found".to_string()))?;

    let stored_hash = card.password.as_deref().ok_or(AppError::Unauthorized)?;
    if !auth_service::verify_password(password, stored_hash) {
        return Err(AppError::Unauthorized);
    }

    Ok(BalanceResponse {
        nfc_ref: card.nfc_ref,
        client_code: card.client_code,
        amount: card.amount,
        network: card.network,
    })
}
