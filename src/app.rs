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
//!    header and injects [`CurrentUser`] into request extensions.

use axum::{
    Router,
    extract::{Request, State},
    http::{Method, StatusCode, header},
    middleware::{self, Next},
    response::Response,
    routing::post,
};
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer, cors::CorsLayer, timeout::TimeoutLayer, trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    errors::ErrorResponse,
    handlers::{
        agent_handler, auth_handler, card_handler, category_handler, commission_handler,
        commission_tier_handler, consumption_handler, customer_handler, health_handler,
        price_handler, transaction_handler, user_handler,
    },
    models::{
        agent::{
            AgentAuthResponse, AgentHistoryRow, AgentInfo, AgentLoginRequest,
            AgentRegisterCustomerRequest, CreateAgentRequest, UpdateAgentPasswordRequest,
            UpdateAgentRequest,
        },
        card::{BalanceCheckRequest, BalanceResponse, Card, CardDetail, CreateCardRequest},
        category::{Category, CreateCategoryRequest},
        commission::{Commission, CreateCommissionRequest},
        commission_tier::{CommissionTier, CreateCommissionTierRequest},
        consumption::{Consumption, CreateConsumptionRequest},
        customer::{
            Customer, CustomerByCardResponse, RegisterCustomerRequest, UpdateCustomerRequest,
        },
        price::{CreatePriceRequest, FuelPrice},
        transaction::{Transaction, WithdrawalRequest, WithdrawalResponse},
        user::{AuthResponse, CurrentUser, LoginRequest, MeResponse, RegisterRequest, UserInfo},
    },
    routes,
    services::auth_service,
    state::app_state::{AppState, AuthConfig, RedisPool},
    utils::cache,
};

/// Maximum duration for a single request before the server responds with
/// `408 Request Timeout`.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// OpenAPI documentation for the Storm API.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Storm API",
        version = "0.1.1",
        description = "Fuel station management REST API — NFC card balances, agent withdrawals with commission, fuel consumption logging, and MLM loyalty bonuses.",
    ),
    paths(
        // Health
        health_handler::health,
        health_handler::ready,
        health_handler::metrics,
        // Auth
        auth_handler::login,
        auth_handler::register,
        auth_handler::logout,
        // Users
        user_handler::me,
        // Agents
        agent_handler::list_agents,
        agent_handler::get_agent,
        agent_handler::create_agent,
        agent_handler::update_agent,
        agent_handler::delete_agent,
        agent_handler::login,
        agent_handler::check_balance,
        agent_handler::history,
        agent_handler::register_customer,
        agent_handler::update_password,
        // Cards
        card_handler::list_cards,
        card_handler::get_card,
        card_handler::create_card,
        card_handler::check_balance,
        // Categories
        category_handler::list_categories,
        category_handler::get_category,
        category_handler::create_category,
        // Customers
        customer_handler::list_customers,
        customer_handler::get_customer,
        customer_handler::get_by_card,
        customer_handler::register,
        customer_handler::update_customer,
        customer_handler::delete_customer,
        // Consumptions
        consumption_handler::list_consumptions,
        consumption_handler::list_by_client,
        consumption_handler::create,
        // Transactions
        transaction_handler::list_transactions,
        transaction_handler::list_by_agent,
        transaction_handler::withdrawal,
        // Commissions
        commission_handler::list_commissions,
        commission_handler::get_current,
        commission_handler::create_commission,
        // Commission Tiers
        commission_tier_handler::list_tiers,
        commission_tier_handler::get_by_category,
        commission_tier_handler::create_tier,
        // Prices
        price_handler::list_prices,
        price_handler::get_by_type,
        price_handler::create_price,
    ),
    components(
        schemas(
            // Error
            ErrorResponse,
            // Auth / User
            LoginRequest, RegisterRequest, AuthResponse, UserInfo, MeResponse,
            // Agent
            AgentLoginRequest, CreateAgentRequest, UpdateAgentPasswordRequest,
            UpdateAgentRequest,
            AgentRegisterCustomerRequest, AgentAuthResponse, AgentInfo, AgentHistoryRow,
            // Card
            Card, CardDetail, CreateCardRequest, BalanceCheckRequest, BalanceResponse,
            // Category
            Category, CreateCategoryRequest,
            // Customer
            Customer, RegisterCustomerRequest, UpdateCustomerRequest,
            CustomerByCardResponse,
            // Consumption
            Consumption, CreateConsumptionRequest,
            // Transaction
            Transaction, WithdrawalRequest, WithdrawalResponse,
            // Commission
            Commission, CreateCommissionRequest,
            // Commission Tier
            CommissionTier, CreateCommissionTierRequest,
            // Price
            FuelPrice, CreatePriceRequest,
            // Health
            health_handler::MetricsResponse,
        ),
    ),
    tags(
        (name = "Health", description = "Liveness, readiness, and metrics"),
        (name = "Auth", description = "System user authentication"),
        (name = "Users", description = "Current user identity"),
        (name = "Agents", description = "Agent accounts, login, history, and customer registration"),
        (name = "Cards", description = "NFC card management and balance checks"),
        (name = "Categories", description = "Vehicle/customer categories"),
        (name = "Customers", description = "Customer profiles and enrollment"),
        (name = "Consumptions", description = "Fuel consumption logging"),
        (name = "Transactions", description = "Financial transactions and withdrawals"),
        (name = "Commissions", description = "Withdrawal commission rates"),
        (name = "Commission Tiers", description = "MLM loyalty bonus tiers"),
        (name = "Prices", description = "Fuel pricing"),
    ),
    modifiers(&SecurityAddon),
)]
struct ApiDoc;

/// Adds the `bearer` HTTP security scheme (JWT) to the OpenAPI spec.
struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::HttpBuilder::new()
                        .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some(
                            "Enter the JWT token obtained from /api/v1/auth/login or /api/v1/agents/login",
                        ))
                        .build(),
                ),
            );
        }
    }
}

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
/// | `/api/v1/commissions` | **Yes** | [`routes::commissions`] |
/// | `/api/v1/commission-tiers` | **Yes** | [`routes::commission_tiers`] |
/// | `/api/v1/prices` | **Yes** | [`routes::prices`] |
/// | `/api/v1/docs` | No | Swagger UI (OpenAPI docs) |
/// | `/api-doc/openapi.json` | No | OpenAPI JSON spec |
///
/// Any unmatched path returns **404**.
pub fn create_app(state: AppState) -> Router {
    // Public routes (no auth required)
    let public = Router::new()
        .nest("/api/v1/auth", routes::auth::routes())
        .nest(
            "/api/v1/agents/login",
            Router::new().route("/", post(agent_handler::login)),
        );

    // Protected routes (JWT required)
    let protected = Router::new()
        .route("/api/v1/auth/logout", post(auth_handler::logout))
        .nest("/api/v1/users", routes::users::routes())
        .nest("/api/v1/categories", routes::categories::routes())
        .nest("/api/v1/cards", routes::cards::routes())
        .nest("/api/v1/prices", routes::prices::routes())
        .nest("/api/v1/agents", routes::agents::routes())
        .nest("/api/v1/customers", routes::customers::routes())
        .nest("/api/v1/consumptions", routes::consumptions::routes())
        .nest("/api/v1/transactions", routes::transactions::routes())
        .nest("/api/v1/commissions", routes::commissions::routes())
        .nest(
            "/api/v1/commission-tiers",
            routes::commission_tiers::routes(),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    Router::new()
        .merge(routes::health::routes())
        .merge(public)
        .merge(protected)
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
            request_counter,
        ))
        .fallback(not_found)
        .with_state(state)
}

/// Builds a permissive CORS layer that allows any origin, common HTTP methods,
/// and the `Content-Type` / `Authorization` headers.
fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
}

/// JWT authentication middleware.
///
/// Extracts the `Bearer <token>` from the `Authorization` header, checks the
/// Redis blocklist (for logged-out tokens), verifies the token via
/// [`auth_service::verify_token`], and on success inserts a [`CurrentUser`]
/// into request extensions for downstream handlers.
///
/// Returns `401 Unauthorized` if the header is missing, malformed, the token
/// is blocklisted, or the token is invalid/expired.
async fn auth_middleware(
    State(config): State<Arc<AuthConfig>>,
    State(redis): State<RedisPool>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Reject blocklisted (logged-out) tokens
    if cache::is_blocklisted(&redis, token).await {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let claims =
        auth_service::verify_token(&config, token).map_err(|_| StatusCode::UNAUTHORIZED)?;

    request.extensions_mut().insert(CurrentUser {
        id: claims.sub,
        role: claims.role,
    });

    Ok(next.run(request).await)
}

/// Fallback handler for unmatched routes.
///
/// Returns `(404, "404 - Route not found")`.
async fn not_found() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "404 - Route not found")
}

/// Middleware that atomically increments the global request counter on every
/// inbound request. The current count is exposed via the `/metrics` endpoint.
async fn request_counter(State(state): State<AppState>, request: Request, next: Next) -> Response {
    state.request_count.fetch_add(1, Ordering::Relaxed);
    next.run(request).await
}
