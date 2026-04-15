//! Customer profile types for the `customers` table.
//!
//! A **customer** represents a cardholder with personal details and an
//! assigned NFC card. This module provides the data transfer objects (DTOs)
//! for interacting with customer records, including registration, update,
//! and lookup operations. All fields and types are kept in sync with the
//! current database schema.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Database row for the `customers` table.
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Customer {
    /// Primary key (`UUID`).
    pub id: Uuid,
    /// Status (0 = inactive, 1 = active, etc.).
    pub status: i32,
    /// Auto-generated client code.
    pub client_code: Option<String>,
    /// Given name
    pub first_name: Option<String>,
    /// Middle name.
    pub middle_name: Option<String>,
    /// Family name.
    pub last_name: Option<String>,
    /// Postal address.
    pub address: Option<String>,
    /// Assigned network code.
    pub networks: Option<String>,
    /// Phone number.
    pub phone: Option<String>,
    /// Foreign key to [`Category`](super::category::Category).
    pub category_ref: Option<Uuid>,
    /// NFC card identifier (links to `cards.card_id` / `card_details.nfc_ref`).
    pub card_id: String,
    /// Gender (e.g. `"M"`, `"F"`).
    pub gender: Option<String>,
    /// Marital status.
    pub marital_status: Option<String>,
    /// Organisational affiliation.
    pub affiliation: Option<String>,
}

/// Request body for `POST /api/v1/customers` (register a customer).
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterCustomerRequest {
    /// Auto-generated client code.
    pub client_code: Option<String>,
    /// NFC card identifier to associate (required).
    pub card_id: String,
    /// Given name (required).
    pub first_name: String,
    /// Middle name (optional).
    pub middle_name: Option<String>,
    /// Family name (required).
    pub last_name: String,
    /// Postal address.
    pub address: Option<String>,
    /// Assigned network code.
    pub networks: Option<String>,
    /// Phone number (required).
    pub phone: String,
    /// Foreign key to [`Category`](super::category::Category).
    pub category_ref: Option<Uuid>,
    /// Gender (e.g. "M", "F").
    pub gender: Option<String>,
    /// Marital status.
    pub marital_status: Option<String>,
    /// Organisational affiliation.
    pub affiliation: Option<String>,
}

/// Minimal response when looking up a customer by card.
#[derive(Debug, Serialize, ToSchema)]
pub struct CustomerByCardResponse {
    /// The customer's generated client code.
    pub client_code: String,
}

/// Request body for `PUT /api/v1/customers/{id}`.
///
/// All fields are optional; only non-`None` values will be applied to the
/// existing record via `COALESCE`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateCustomerRequest {
    /// Given name.
    pub first_name: Option<String>,
    /// Middle name
    pub middle_name: Option<String>,
    /// Family name
    pub last_name: Option<String>,
    /// Postal address.
    pub address: Option<String>,
    /// Phone number.
    pub phone: Option<String>,
    /// Gender.
    pub gender: Option<String>,
    /// Marital status.
    pub marital_status: Option<String>,
    /// Affiliation.
    pub affiliation: Option<String>,
    /// Network code.
    pub networks: Option<String>,
    /// NFC card identifier.
    pub card_id: Option<String>,
    /// Category foreign key.
    pub category_ref: Option<Uuid>,
}
