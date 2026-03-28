//! Withdrawal commission rate types for the `commissions` table.
//!
//! A single active commission percentage is used by the withdrawal flow
//! to compute the fee deducted from the customer's balance and credited
//! to the house account.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Database row for the `commissions` table.
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Commission {
    /// Primary key (`UUID`).
    pub id: Uuid,
    /// Commission percentage (e.g. `5.0` means 5%).
    pub percentage: f64,
    /// Timestamp of creation — the most recent row is the active rate.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Request body for `POST /api/v1/commissions`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateCommissionRequest {
    /// The new commission percentage to apply.
    pub percentage: f64,
}
