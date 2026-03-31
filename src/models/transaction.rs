//! Financial transaction types for the `transactions` table.
//!
//! Transactions record monetary movements — primarily withdrawals — between
//! customer cards and agent accounts.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Database row for the `transactions` table.
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Transaction {
    /// Primary key (`UUID`).
    pub id: Uuid,
    /// Timestamp of the transaction.
    pub date: Option<chrono::DateTime<chrono::Utc>>,
    /// E.g. `"WITHDRAWAL"`.
    pub transaction_type: Option<String>,
    /// NFC reference / card ID of the customer involved.
    pub client_account: Option<String>,
    /// Agent reference code of the agent involved.
    pub agent_account: Option<String>,
    /// Monetary amount (exclusive of commission).
    pub amount: Option<f64>,
    /// ISO currency code.
    pub currency_code: Option<String>,
    /// Commission amount deducted from the customer.
    pub commission: Option<f64>,
}

/// Request body for `POST /api/v1/transactions/withdrawal`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct WithdrawalRequest {
    /// NFC reference / card identifier of the customer.
    pub client_code: String,
    /// Amount to withdraw from the customer's card.
    pub withdrawal_amount: f64,
    /// Plaintext card password for verification.
    pub client_password: String,
    /// Agent reference code performing the withdrawal.
    pub agent_code: String,
    /// ISO currency code (must match the agent's currency).
    pub currency_type: String,
}

/// Successful withdrawal response returned to the caller.
#[derive(Debug, Serialize, ToSchema)]
pub struct WithdrawalResponse {
    /// Human-readable confirmation message.
    pub message: String,
    /// Updated customer card balance after deduction.
    pub client_balance: f64,
    /// Updated agent account balance after credit.
    pub agent_balance: f64,
}
