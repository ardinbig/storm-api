//! Vehicle/customer category types for the `categories` table.
//!
//! Categories classify customers (e.g. `"Motorbike"`, `"Bus"`) and are
//! referenced by the consumption bonus trigger to look up the appropriate
//! commission tier.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Database row for the `categories` table.
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Category {
    /// Primary key (`UUID`).
    pub id: Uuid,
    /// Category name (e.g. `"Motorbike"`).
    pub name: String,
    /// Timestamp of creation.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Request body for `POST /api/v1/categories`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateCategoryRequest {
    /// The name of the new category.
    pub name: String,
}
