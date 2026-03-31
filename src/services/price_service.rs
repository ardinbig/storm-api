//! Fuel pricing business logic.
//!
//! The most recent price per `consumption_type` is considered the current
//! price. When a [`RedisPool`] is provided, `get_by_type` results are
//! cached for [`PRICE_TTL`] seconds and invalidated on `create`.

use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::price::{CreatePriceRequest, FuelPrice},
    state::app_state::RedisPool,
    utils::cache,
};

/// Cache TTL for fuel price lookups (30 minutes).
const PRICE_TTL: u64 = 1800;

/// Lists all fuel prices, most recent first.
///
/// # Errors
///
/// Returns [`AppError::Database`] on query failure.
pub async fn list(pool: &PgPool) -> Result<Vec<FuelPrice>, AppError> {
    let prices = sqlx::query_as::<_, FuelPrice>(
        "SELECT id, consumption_type, price::FLOAT8 AS price, price_date
         FROM prices ORDER BY price_date DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(prices)
}

/// Retrieves the current (most recent) price for a fuel type.
///
/// Results are cached when `redis` is `Some`.
///
/// # Errors
///
/// - [`AppError::NotFound`] — no price defined for this consumption type.
/// - [`AppError::Database`] — query failure.
pub async fn get_by_type(
    pool: &PgPool,
    consumption_type: &str,
    redis: &RedisPool,
) -> Result<FuelPrice, AppError> {
    let key = cache::price_key(consumption_type);

    if let Some(cached) = cache::get::<FuelPrice>(redis, &key).await {
        return Ok(cached);
    }

    let price = sqlx::query_as::<_, FuelPrice>(
        "SELECT id, consumption_type, price::FLOAT8 AS price, price_date
         FROM prices WHERE consumption_type = $1
         ORDER BY price_date DESC LIMIT 1",
    )
    .bind(consumption_type)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        AppError::NotFound(format!(
            "No price found for consumption type: {consumption_type}"
        ))
    })?;

    cache::set(redis, &key, &price, PRICE_TTL).await;

    Ok(price)
}

/// Creates a new fuel price record and invalidates the cached price for
/// the corresponding consumption type.
///
/// # Errors
///
/// Returns [`AppError::Database`] on constraint violation or query failure.
pub async fn create(
    pool: &PgPool,
    input: &CreatePriceRequest,
    redis: &RedisPool,
) -> Result<FuelPrice, AppError> {
    let id = Uuid::new_v4();
    let price = sqlx::query_as::<_, FuelPrice>(
        "INSERT INTO prices (id, consumption_type, price) VALUES ($1, $2, $3)
         RETURNING id, consumption_type, price::FLOAT8 AS price, price_date",
    )
    .bind(id)
    .bind(&input.consumption_type)
    .bind(input.price)
    .fetch_one(pool)
    .await?;

    // Invalidate cached price for this consumption type
    cache::del(redis, &cache::price_key(&input.consumption_type)).await;

    Ok(price)
}
