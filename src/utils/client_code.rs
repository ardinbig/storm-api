//! Client code generation helpers.

/// Generates client codes in the canonical format: `STORM-YYYYMMDD-HHMMSS`.
pub fn generate_client_code() -> String {
    let now = chrono::Utc::now();
    format!("STORM-{}", now.format("%Y%m%d-%H%M%S"))
}
