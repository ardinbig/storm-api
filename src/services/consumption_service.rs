//! Fuel consumption business logic: listing and recording dispensing events.
//!
//! A database trigger (`fn_consumption_bonus_tree`) fires after each insert
//! to calculate MLM loyalty bonuses based on the consumption amount and the
//! applicable commission tier.

use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::consumption::{Consumption, CreateConsumptionRequest},
};

/// Lists all consumption records, most recent first.
///
/// # Errors
///
/// Returns [`AppError::Database`] on query failure.
pub async fn list(pool: &PgPool) -> Result<Vec<Consumption>, AppError> {
    let consumptions = sqlx::query_as::<_, Consumption>(
        "SELECT client_ref, consumption_type, quantity::FLOAT8 AS quantity,
                price::FLOAT8 AS price, username, consumption_date, status
         FROM consumptions ORDER BY consumption_date DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(consumptions)
}

/// Lists consumption records for a specific client, most recent first.
///
/// # Errors
///
/// Returns [`AppError::Database`] on query failure.
pub async fn list_by_client(pool: &PgPool, client_ref: &str) -> Result<Vec<Consumption>, AppError> {
    let consumptions = sqlx::query_as::<_, Consumption>(
        "SELECT client_ref, consumption_type, quantity::FLOAT8 AS quantity,
                price::FLOAT8 AS price, username, consumption_date, status
         FROM consumptions WHERE client_ref = $1 ORDER BY consumption_date DESC",
    )
    .bind(client_ref)
    .fetch_all(pool)
    .await?;

    Ok(consumptions)
}

/// Records a new fuel consumption event.
///
/// The `date` field from the request is cast to `TIMESTAMPTZ` at the SQL
/// level.  After insertion, a database trigger calculates any applicable
/// MLM loyalty bonuses.
///
/// # Errors
///
/// Returns [`AppError::Database`] on constraint violation or query failure.
pub async fn create(pool: &PgPool, input: &CreateConsumptionRequest) -> Result<(), AppError> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO consumptions (id, client_ref, consumption_type, quantity, price, username, consumption_date, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7::TIMESTAMPTZ, 1)",
    )
    .bind(id)
    .bind(&input.client_ref)
    .bind(&input.consumption_type)
    .bind(input.quantity)
    .bind(input.price)
    .bind(&input.username)
    .bind(&input.date)
    .execute(pool)
    .await?;

    Ok(())
}
