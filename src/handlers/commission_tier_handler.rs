//! Commission tier handlers: list, get-by-category, and create.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use sqlx::PgPool;

use crate::{
    errors::AppError,
    models::commission_tier::{CommissionTier, CreateCommissionTierRequest},
    services::commission_tier_service,
};

/// `GET /api/v1/commission-tiers`
///
/// Lists all MLM commission tiers.
pub async fn list_tiers(State(pool): State<PgPool>) -> Result<Json<Vec<CommissionTier>>, AppError> {
    let tiers = commission_tier_service::list(&pool).await?;
    Ok(Json(tiers))
}

/// `GET /api/v1/commission-tiers/by-category/{category}`
///
/// Retrieves the current commission tier for a vehicle category.
pub async fn get_by_category(
    State(pool): State<PgPool>,
    Path(category): Path<String>,
) -> Result<Json<CommissionTier>, AppError> {
    let tier = commission_tier_service::get_by_category(&pool, &category).await?;
    Ok(Json(tier))
}

/// `POST /api/v1/commission-tiers`
///
/// Creates a new commission tier. Returns `201 Created`.
pub async fn create_tier(
    State(pool): State<PgPool>,
    Json(input): Json<CreateCommissionTierRequest>,
) -> Result<(StatusCode, Json<CommissionTier>), AppError> {
    let tier = commission_tier_service::create(&pool, &input).await?;
    Ok((StatusCode::CREATED, Json(tier)))
}
