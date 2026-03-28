//! MLM loyalty bonus tier types for the `commission_tiers` table.
//!
//! Each tier defines the Level-1 and Level-2 bonus percentages applied by
//! the consumption trigger (`fn_consumption_bonus_tree`), optionally scoped
//! to a vehicle category.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Database row for the `commission_tiers` table.
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct CommissionTier {
    /// Primary key (`UUID`).
    pub id: Uuid,
    /// Level-1 (direct referrer) bonus percentage.
    pub level1: f64,
    /// Level-2 (referrer's referrer) bonus percentage.
    pub level2: f64,
    /// Optional vehicle category this tier applies to.
    pub category: Option<String>,
    /// Timestamp of creation — the most recent row per category is active.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Request body for `POST /api/v1/commission-tiers`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateCommissionTierRequest {
    /// Level-1 bonus percentage.
    pub level1: f64,
    /// Level-2 bonus percentage.
    pub level2: f64,
    /// Optional category scope.
    pub category: Option<String>,
}
