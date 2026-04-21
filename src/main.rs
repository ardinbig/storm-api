//! Server entry-point for Storm API.

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;
use storm_api::state::app_state::{AppState, AuthConfig};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Time to wait after receiving a shutdown signal before the process exits.
const SHUTDOWN_GRACE_PERIOD: Duration = Duration::from_secs(5);

/// Returns the value of the environment variable `key`, falling back to
/// `default` when the variable is unset.
fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.into())
}

/// Application entry-point.
///
/// 1. Loads `.env` via [`dotenvy`].
/// 2. Initializes structured JSON logging with an env-filter.
/// 3. Creates the PostgreSQL connection pool.
/// 4. Assembles [`AppState`] and builds the Axum router.
/// 5. Binds a TCP listener and serves requests with graceful shutdown.
#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    // Structured JSON logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "storm_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let pool = storm_api::db::connection::create_pool(&env_or(
        "DATABASE_URL",
        "postgres://postgres:postgres@localhost/stormdb",
    ))
    .await;

    // Ensure the super-admin account exists on every cold start.
    if let Err(e) = storm_api::services::user_service::seed_super_admin(&pool).await {
        tracing::error!("Failed to seed super-admin account: {e}");
    }

    // Redis — optional; the application degrades gracefully when unavailable.
    let redis = match redis::Client::open(env_or("REDIS_URL", "redis://127.0.0.1:6379")) {
        Ok(client) => match redis::aio::ConnectionManager::new(client).await {
            Ok(mgr) => {
                tracing::info!("Redis connection established");
                Some(mgr)
            }
            Err(e) => {
                tracing::warn!("Redis unavailable (connection failed: {e}), caching disabled");
                None
            }
        },
        Err(e) => {
            tracing::warn!("Redis unavailable (invalid URL: {e}), caching disabled");
            None
        }
    };

    let ready = Arc::new(AtomicBool::new(true));

    let state = AppState {
        pool,
        redis,
        auth_config: Arc::new(AuthConfig {
            jwt_secret: env_or("JWT_SECRET", "dev-secret-change-in-production"),
            jwt_expiry_hours: 24,
        }),
        ready: ready.clone(),
        request_count: Arc::new(AtomicU64::new(0)),
    };

    let app = storm_api::app::create_app(state);
    let addr = env_or("APP_ADDR", "127.0.0.1:3000");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("Failed to bind to {addr}: {e}"));

    tracing::info!("Server listening on {addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(ready))
        .await
        .expect("Server failed to start");
}

/// Waits for either `Ctrl+C` or `SIGTERM`, then marks the application as
/// *not ready* and sleeps for the grace period to let in-flight requests
/// drain before the runtime shuts down.
async fn shutdown_signal(ready: Arc<AtomicBool>) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    tracing::info!("Shutdown signal received, starting graceful shutdown");
    ready.store(false, Ordering::SeqCst);
    tokio::time::sleep(SHUTDOWN_GRACE_PERIOD).await;
}
