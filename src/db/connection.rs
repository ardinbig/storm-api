//! PostgreSQL connection pool construction.

use sqlx::{PgPool, postgres::PgPoolOptions};

/// Creates a PostgreSQL connection pool from the given `database_url`.
///
/// The maximum number of connections is read from the `MAX_DB_CONNECTIONS`
/// environment variable, falling back to **10** when unset or unparseable.
/// A minimum of **2** idle connections is always maintained.
///
/// # Panics
///
/// Panics if the pool cannot be established.
pub async fn create_pool(database_url: &str) -> PgPool {
    let max_connections: u32 = std::env::var("MAX_DB_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);

    PgPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(2)
        .connect(database_url)
        .await
        .unwrap_or_else(|err| panic!("Failed to connect to database: {err}"))
}
