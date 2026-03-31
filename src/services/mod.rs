//! Business logic and database access layer.
//!
//! Each submodule exposes free functions (not methods on structs) that
//! accept `&PgPool` as their first argument. All SQL queries are written
//! inline.

pub mod agent_service;
pub mod auth_service;
pub mod card_service;
pub mod category_service;
pub mod commission_service;
pub mod commission_tier_service;
pub mod consumption_service;
pub mod customer_service;
pub mod price_service;
pub mod transaction_service;
pub mod user_service;
