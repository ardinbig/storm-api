//! Fuel consumption log types for the `consumptions` table.
//!
//! Each consumption record captures a fuel dispensing event: the client,
//! fuel type, quantity, price, and the station operator who performed it.
//! A database trigger (`fn_consumption_bonus_tree`) fires on insert to
//! calculate MLM loyalty bonuses.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

/// Database row for the `consumptions` table.
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Consumption {
    /// Client reference code (links to `customers.client_code`).
    pub client_ref: String,
    /// Fuel type (e.g. `"diesel"`, `"essence"`).
    pub consumption_type: String,
    /// Quantity dispensed (liters).
    pub quantity: f64,
    /// Unit price at the time of dispensing.
    pub price: f64,
    /// Username of the station operator who processed the dispensing.
    pub username: String,
    /// Timestamp of the consumption event.
    pub consumption_date: chrono::DateTime<chrono::Utc>,
    /// Processing status flag (`1` = processed).
    pub status: i32,
}

/// Request body for `POST /api/v1/consumptions` (sync from station terminal).
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateConsumptionRequest {
    /// ISO-8601 date/time string of the consumption event.
    pub date: String,
    /// Client reference code.
    pub client_ref: String,
    /// Fuel type.
    pub consumption_type: String,
    /// Quantity dispensed (liters).
    pub quantity: f64,
    /// Unit price.
    pub price: f64,
    /// Operator username.
    pub username: String,
    /// Whether this record was created while online (informational only).
    #[allow(dead_code)]
    pub is_online: bool,
}
