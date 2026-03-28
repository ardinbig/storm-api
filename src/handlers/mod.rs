//! Thin request/response handler functions.
//!
//! Each submodule corresponds to a domain entity.  Handlers extract state
//! and JSON payloads via Axum extractors, delegate to the appropriate
//! service function, and wrap the result in `Json` or an HTTP status code.

pub mod auth_handler;
pub mod card_handler;
pub mod category_handler;
pub mod commission_handler;
pub mod commission_tier_handler;
pub mod health_handler;
pub mod user_handler;
