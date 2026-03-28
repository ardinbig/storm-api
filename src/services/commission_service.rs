//! Withdrawal commission rate business logic.

use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::commission::{Commission, CreateCommissionRequest},
};

/// Lists all commission rate records, most recent first.
///
/// # Errors
///
/// Returns [`AppError::Database`] on query failure.
pub async fn list(pool: &PgPool) -> Result<Vec<Commission>, AppError> {
    let commissions = sqlx::query_as::<_, Commission>(
        "SELECT id, percentage::FLOAT8 AS percentage, created_at
         FROM commissions ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(commissions)
}

/// Retrieves the currently active (most recent) commission rate.
///
/// # Errors
///
/// - [`AppError::NotFound`] — no commission rate has been configured yet.
/// - [`AppError::Database`] — query failure.
pub async fn get_current(pool: &PgPool) -> Result<Commission, AppError> {
    let commission = sqlx::query_as::<_, Commission>(
        "SELECT id, percentage::FLOAT8 AS percentage, created_at
         FROM commissions ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("No commission rate configured".to_string()))?;

    Ok(commission)
}

/// Creates a new commission rate record.
///
/// The most recently created record is always used as the active rate.
///
/// # Errors
///
/// Returns [`AppError::Database`] on constraint violation or query failure.
pub async fn create(
    pool: &PgPool,
    input: &CreateCommissionRequest,
) -> Result<Commission, AppError> {
    let id = Uuid::new_v4();
    let commission = sqlx::query_as::<_, Commission>(
        "INSERT INTO commissions (id, percentage) VALUES ($1, $2)
         RETURNING id, percentage::FLOAT8 AS percentage, created_at",
    )
    .bind(id)
    .bind(input.percentage)
    .fetch_one(pool)
    .await?;

    Ok(commission)
}
