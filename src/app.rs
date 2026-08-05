//! Application router construction.
//!
//! [`create_app`] assembles the full Axum [`Router`] by merging public routes
//! (authentication, agent login, health checks) with JWT-protected routes
//! (users, cards, customers, agents, transactions, etc.).
//!
//! Middleware layers applied (outer → inner):
//!
//! 1. **Request counter** — atomically increments [`AppState::request_count`]
//!    for every inbound request, surfaced at `/metrics`.
//! 2. **Tracing** — `tower_http` request/response logging.
//! 3. **Compression** — gzip response bodies.
//! 4. **Timeout** — returns `408 Request Timeout` after `REQUEST_TIMEOUT`.
//! 5. **CORS** — permissive cross-origin policy.
//! 6. **Auth** (protected routes only) — validates the `Authorization: Bearer`
//!    header and injects the authenticated user into request extensions.
//! 7. **Idempotency** (protected mutating routes only) — coordinates
//!    idempotency keys via Redis and replays successful cached responses.

mod open_api;

use axum::{
    Router,
    http::{HeaderName, Method, StatusCode, header},
    middleware,
    routing::{get, post},
};
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer, cors::CorsLayer, timeout::TimeoutLayer, trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    handlers::{agent_handler, auth_handler, transaction_handler},
    middleware::{auth, idempotency, request_counter},
    routes,
    state::app_state::AppState,
};
use open_api::ApiDoc;

/// Maximum duration for a single request before the server responds with
/// `408 Request Timeout`.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Constructs the complete Axum [`Router`] with all routes, middleware layers,
/// and shared application state.
///
/// # Route Layout
///
/// | Prefix | Auth | Module |
/// |--------|------|--------|
/// | `/health`, `/ready`, `/metrics` | No | [`routes::health`] |
/// | `/api/v1/auth` | No | [`routes::auth`] |
/// | `/api/v1/agents/login` | No | [`agent_handler::login`] |
/// | `/api/v1/users` | **Yes** | [`routes::users`] |
/// | `/api/v1/cards` | **Yes** | [`routes::cards`] |
/// | `/api/v1/categories` | **Yes** | [`routes::categories`] |
/// | `/api/v1/customers` | **Yes** | [`routes::customers`] |
/// | `/api/v1/consumptions` | **Yes** | [`routes::consumptions`] |
/// | `/api/v1/agents` | **Yes** | [`routes::agents`] |
/// | `/api/v1/transactions` | **Yes** | [`routes::transactions`] |
/// | `/api/v1/activity` | **Yes** | [`transaction_handler::list_activity`] |
/// | `/api/v1/commissions` | **Yes** | [`routes::commissions`] |
/// | `/api/v1/commission-tiers` | **Yes** | [`routes::commission_tiers`] |
/// | `/api/v1/prices` | **Yes** | [`routes::prices`] |
/// | `/api/v1/docs` | No | Swagger UI (OpenAPI docs) |
/// | `/api-doc/openapi.json` | No | OpenAPI JSON spec |
///
/// Protected routes are wrapped with JWT authentication and idempotency
/// middleware before handler execution.
///
/// Any unmatched path returns **404**.
pub fn create_app(state: AppState) -> Router {
    Router::new()
        .merge(routes::health::routes())
        .merge(public_routes())
        .merge(protected_routes(&state))
        .merge(SwaggerUi::new("/api/v1/docs").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    REQUEST_TIMEOUT,
                ))
                .layer(cors_layer()),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            request_counter::request_counter,
        ))
        .fallback(not_found)
        .with_state(state)
}

/// Returns the public, unauthenticated routes.
fn public_routes() -> Router<AppState> {
    Router::new()
        .nest("/api/v1/auth", routes::auth::routes())
        .nest(
            "/api/v1/agents/login",
            Router::new().route("/", post(agent_handler::login)),
        )
}

/// Returns all JWT-protected routes with auth and idempotency middleware.
fn protected_routes(state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/logout", post(auth_handler::logout))
        .nest("/api/v1/users", routes::users::routes())
        .nest("/api/v1/categories", routes::categories::routes())
        .nest("/api/v1/cards", routes::cards::routes())
        .nest("/api/v1/prices", routes::prices::routes())
        .nest("/api/v1/agents", routes::agents::routes())
        // TODO(ardinbig): Implement pagination for customers list (add metadata)
        .nest("/api/v1/customers", routes::customers::routes())
        .nest("/api/v1/consumptions", routes::consumptions::routes())
        .nest("/api/v1/transactions", routes::transactions::routes())
        .route("/api/v1/activity", get(transaction_handler::list_activity))
        .nest("/api/v1/commissions", routes::commissions::routes())
        .nest(
            "/api/v1/commission-tiers",
            routes::commission_tiers::routes(),
        )
        .route_layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    auth::auth_middleware,
                ))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    idempotency::idempotency_middleware,
                )),
        )
}

/// Builds a permissive CORS layer that allows any origin, common HTTP methods,
/// and the `Content-Type`, `Authorization`, and `X-Idempotency-Key` headers.
fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            HeaderName::from_static(idempotency::IDEMPOTENCY_HEADER_NAME),
        ])
}

/// Fallback handler for unmatched routes.
///
/// Returns `(404, "404 - Route not found")`.
async fn not_found() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "404 - Route not found")
}
