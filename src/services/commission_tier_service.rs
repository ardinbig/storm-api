//! MLM loyalty bonus tier business logic.
//!
//! Tiers define the Level-1 and Level-2 bonus percentages used by the
//! consumption trigger, optionally scoped to a vehicle category.

use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::commission_tier::{CommissionTier, CreateCommissionTierRequest},
};

/// Lists all commission tiers, most recent first.
///
/// # Errors
///
/// Returns [`AppError::Database`] on query failure.
pub async fn list(pool: &PgPool) -> Result<Vec<CommissionTier>, AppError> {
    let tiers = sqlx::query_as::<_, CommissionTier>(
        "SELECT id, level1::FLOAT8 AS level1, level2::FLOAT8 AS level2, category, created_at
         FROM commission_tiers ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(tiers)
}

/// Retrieves the most recent commission tier for a given vehicle category.
///
/// # Errors
///
/// - [`AppError::NotFound`] — no tier defined for this category.
/// - [`AppError::Database`] — query failure.
pub async fn get_by_category(pool: &PgPool, category: &str) -> Result<CommissionTier, AppError> {
    let tier = sqlx::query_as::<_, CommissionTier>(
        "SELECT id, level1::FLOAT8 AS level1, level2::FLOAT8 AS level2, category, created_at
         FROM commission_tiers WHERE category = $1
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(category)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        AppError::NotFound(format!("No commission tier found for category: {category}"))
    })?;

    Ok(tier)
}

/// Creates a new commission tier record.
///
/// # Errors
///
/// Returns [`AppError::Database`] on constraint violation or query failure.
pub async fn create(
    pool: &PgPool,
    input: &CreateCommissionTierRequest,
) -> Result<CommissionTier, AppError> {
    let id = Uuid::new_v4();
    let tier = sqlx::query_as::<_, CommissionTier>(
        "INSERT INTO commission_tiers (id, level1, level2, category) VALUES ($1, $2, $3, $4)
         RETURNING id, level1::FLOAT8 AS level1, level2::FLOAT8 AS level2, category, created_at",
    )
    .bind(id)
    .bind(input.level1)
    .bind(input.level2)
    .bind(&input.category)
    .fetch_one(pool)
    .await?;

    Ok(tier)
}
