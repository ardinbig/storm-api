//! Agent account types for the `agent_accounts` table.
//!
//! Agents are field operators who perform customer withdrawals and earn
//! commissions. They authenticate separately via `/api/v1/agents/login`
//! and receive a JWT with `role = "agent"`.
//!
//! The special house account ([`HOUSE_ACCOUNT_REF`]) collects withdrawal
//! commissions and must never be deleted.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// The `agent_ref` of the special house/commission account.
///
/// Withdrawal commissions are always credited to this account. Attempting
/// to delete this account returns [`AppError::BadRequest`](crate::errors::AppError::BadRequest).
pub const HOUSE_ACCOUNT_REF: &str = "STORM-ACCOUNT-0000";

/// Default network code assigned to every new customer/card created through
/// agent registration.
pub const DEFAULT_NETWORK: &str = "STORM-NETWORK-0000";

/// Database row for the `agent_accounts` table.
///
/// The `password` field is excluded from JSON serialisation.
#[derive(Debug, Serialize, FromRow)]
pub struct Agent {
    /// Primary key (`UUID`).
    pub id: Uuid,
    /// Unique agent reference code (e.g. `"STORM-ACCOUNT-0001"`).
    pub agent_ref: String,
    /// Agent's display name.
    pub name: Option<String>,
    /// Argon2-hashed password (excluded from JSON output).
    #[serde(skip_serializing)]
    pub password: Option<String>,
    /// Current account balance (in `currency_code` units).
    pub balance: Option<f64>,
    /// ISO currency code (typically `"CDF"`).
    pub currency_code: String,
}

/// Request body for `POST /api/v1/agents/login`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AgentLoginRequest {
    /// The agent's `agent_ref` used as login identifier.
    pub username: String,
    /// Plaintext password.
    pub password: String,
}

/// Request body for `PUT /api/v1/agents/password`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateAgentPasswordRequest {
    /// The agent's reference code.
    pub agent_ref: String,
    /// Current password (must match stored hash for verification).
    pub last_password: String,
    /// Desired new password (will be Argon2-hashed before storage).
    pub new_password: String,
}

/// Request body when an agent registers a new customer through the mobile
/// app (`POST /api/v1/agents/customers`).
#[derive(Debug, Deserialize, ToSchema)]
pub struct AgentRegisterCustomerRequest {
    /// Given name (required).
    pub first_name: String,
    /// Middle name (optional).
    pub middle_name: Option<String>,
    /// Family name (required).
    pub last_name: String,
    /// Postal address.
    pub address: Option<String>,
    /// Phone number (required).
    pub phone: String,
    /// NFC card reference to assign to the new customer.
    pub card_ref: String,
    /// Gender (e.g. `"M"`, `"F"`).
    pub gender: Option<String>,
    /// Marital status.
    pub marital_status: Option<String>,
    /// Organisational affiliation.
    pub affiliation: Option<String>,
}

/// A single row from the agent's transaction history view.
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct AgentHistoryRow {
    /// Transaction primary key.
    pub id: Uuid,
    /// Timestamp of the transaction.
    pub date: Option<chrono::DateTime<chrono::Utc>>,
    /// E.g. `"WITHDRAWAL"`.
    pub transaction_type: Option<String>,
    /// ISO currency code.
    pub currency_code: Option<String>,
    /// Monetary amount involved.
    pub amount: Option<f64>,
    /// Formatted client name (`name`).
    pub client: Option<String>,
}

/// Successful agent authentication response.
#[derive(Debug, Serialize, ToSchema)]
pub struct AgentAuthResponse {
    /// Signed JWT.
    pub token: String,
    /// Agent profile (no password).
    pub agent: AgentInfo,
}

/// Public-facing agent information (password omitted).
#[derive(Debug, Serialize, ToSchema)]
pub struct AgentInfo {
    /// Primary key.
    pub id: Uuid,
    /// Unique agent reference code.
    pub agent_ref: String,
    /// Display name.
    pub name: Option<String>,
    /// Current balance.
    pub balance: Option<f64>,
    /// ISO currency code.
    pub currency_code: String,
}

/// Converts a full [`Agent`] database row into the password-free [`AgentInfo`]
/// DTO for API responses.
impl From<Agent> for AgentInfo {
    fn from(a: Agent) -> Self {
        Self {
            id: a.id,
            agent_ref: a.agent_ref,
            name: a.name,
            balance: a.balance,
            currency_code: a.currency_code,
        }
    }
}

/// Request body for `POST /api/v1/agents` (admin-only agent creation).
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAgentRequest {
    /// Unique agent reference code.
    pub agent_ref: String,
    /// Display name.
    pub name: Option<String>,
    /// Initial plaintext password (will be Argon2-hashed).
    pub password: String,
    /// ISO currency code; defaults to `"CDF"` when omitted.
    pub currency_code: Option<String>,
}

/// Request body for `PATCH /api/v1/agents/{id}`.
///
/// All fields are optional; only non-`None` values will be applied to the
/// existing record via `COALESCE`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateAgentRequest {
    /// Display name.
    pub name: Option<String>,
    /// ISO currency code.
    pub currency_code: Option<String>,
}
