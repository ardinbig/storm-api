//! Fuel pricing types for the `prices` table.
//!
//! Each record captures the price of a specific fuel type at a point in
//! time.  The most recent row per `consumption_type` is considered the
//! current price.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Database row for the `prices` table.
#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct FuelPrice {
    /// Primary key (`UUID`).
    pub id: Uuid,
    /// Fuel type (e.g. `"diesel"`, `"essence"`).
    pub consumption_type: String,
    /// Unit price per liter.
    pub price: f64,
    /// Timestamp of when this price took effect.
    pub price_date: chrono::DateTime<chrono::Utc>,
}

/// Request body for `POST /api/v1/prices`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePriceRequest {
    /// Fuel type to set the price for.
    pub consumption_type: String,
    /// New unit price per liter.
    pub price: f64,
}
