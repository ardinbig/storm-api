//! OpenAPI document assembly for the application router.

use utoipa::OpenApi;

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
        pagination::{
            ActivityItem, ActivityQuery, ConsumptionQuery, PaginatedActivityResponse,
            PaginatedConsumptionResponse, PaginatedTransactionResponse, TransactionQuery,
        },
        price::{CreatePriceRequest, FuelPrice},
        transaction::{Transaction, WithdrawalRequest, WithdrawalResponse},
        user::{AuthResponse, LoginRequest, MeResponse, RegisterRequest, UserInfo},
    },
};

/// OpenAPI documentation for the Storm API.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Storm API",
        version = "0.1.4",
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
        // Activity
        transaction_handler::list_activity,
        // Commissions
        commission_handler::list_commissions,
        commission_handler::get_current,
        commission_handler::create_commission,
        commission_handler::delete_commission,
        // Commission tiers
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
            ErrorResponse,
            LoginRequest, RegisterRequest, AuthResponse, UserInfo, MeResponse,
            AgentLoginRequest, CreateAgentRequest, UpdateAgentPasswordRequest,
            UpdateAgentRequest, AgentRegisterCustomerRequest, AgentAuthResponse,
            AgentInfo, AgentHistoryRow,
            Card, CardDetail, CreateCardRequest, BalanceCheckRequest, BalanceResponse,
            Category, CreateCategoryRequest,
            Customer, RegisterCustomerRequest, UpdateCustomerRequest, CustomerByCardResponse,
            Consumption, CreateConsumptionRequest,
            Transaction, WithdrawalRequest, WithdrawalResponse,
            Commission, CreateCommissionRequest,
            CommissionTier, CreateCommissionTierRequest,
            FuelPrice, CreatePriceRequest,
            health_handler::MetricsResponse,
            ActivityItem, ActivityQuery, TransactionQuery, ConsumptionQuery,
            PaginatedTransactionResponse, PaginatedConsumptionResponse, PaginatedActivityResponse,
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
        (name = "Activity", description = "Unified paginated feed of withdrawals and consumptions"),
        (name = "Commissions", description = "Withdrawal commission rates"),
        (name = "Commission Tiers", description = "MLM loyalty bonus tiers"),
        (name = "Prices", description = "Fuel pricing"),
    ),
    modifiers(&SecurityAddon),
)]
pub(super) struct ApiDoc;

/// Adds the `bearer` HTTP security scheme (JWT) to the generated OpenAPI spec.
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
