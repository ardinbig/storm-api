//! Shared utility helpers.
//!
//! - [`password::hash`] — Argon2 password hashing with `AppError` mapping.
//! - [`cache`] — Redis caching helpers and JWT blocklist operations.

pub mod cache;
pub mod client_code;
pub mod password;
