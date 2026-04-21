//! Shared pagination types used across the API.
//!
//! Each paginated endpoint has its own concrete response struct
//! (`Paginated*Response`) so that utoipa can produce named, non-generic
//! OpenAPI schemas without relying on unstable generic-alias features.

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::models::{consumption::Consumption, transaction::Transaction};

/// Number of items returned per page across all paginated endpoints.
pub const PAGE_SIZE: u32 = 10;

// Activity item
// =============

/// A single event in the unified activity feed.
///
/// Each row is either a financial withdrawal (`kind = "WITHDRAWAL"`) or a fuel
/// dispensing event (`kind = "CONSUMPTION"`).
#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct ActivityItem {
    /// Timestamp of the event.
    pub date: Option<chrono::DateTime<chrono::Utc>>,
    /// `"WITHDRAWAL"` or `"CONSUMPTION"`.
    pub kind: String,
    /// Agent reference code that performed the operation.
    pub agent_ref: Option<String>,
    /// Client reference (NFC code for withdrawals; `client_ref` for consumptions).
    pub client_ref: Option<String>,
    /// Monetary amount (withdrawal amount, or `quantity × price` for consumptions).
    pub amount: Option<f64>,
    /// Station (system-user UUID) the agent belongs to, if any.
    pub station_id: Option<Uuid>,
}

// Macro-generated paginated response types
//
// All three concrete response structs (`PaginatedTransactionResponse`,
// `PaginatedConsumptionResponse`, `PaginatedActivityResponse`) share an
// identical field layout and `new()` constructor.  Instead of copy-pasting
// the same 8 fields and the same arithmetic three times, a single
// `paginated_response!` macro invocation generates each distinct named type.
//
// Distinct named types are required because utoipa resolves OpenAPI schemas
// by struct name at compile time; a generic `PaginatedResponse<T>` would
// produce a single ambiguous schema entry rather than three separate,
// self-documenting schema objects in the Swagger UI.
//
// The `new()` constructor:
//   - clamps `page` to a minimum of `1` (guards against `?page=0`).
//   - computes `total_pages` as `ceil(total_items / PAGE_SIZE)`, floored at `1`
//     so an empty result set still reports one page.
//   - derives `has_next_page`, `has_prev_page`, and `remaining_items` from
//     those two values so callers never have to recompute them.
macro_rules! paginated_response {
    ($name:ident, $item:ty, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Serialize, ToSchema)]
        pub struct $name {
            pub data: Vec<$item>,
            /// Current page (1-based).
            pub page: u32,
            /// Items per page (always [`PAGE_SIZE`]).
            pub page_size: u32,
            /// Total matching items across all pages.
            pub total_items: i64,
            /// Total number of pages (`ceil(total / page_size)`, minimum 1).
            pub total_pages: u32,
            /// `true` when a next page exists.
            pub has_next_page: bool,
            /// `true` when a previous page exists.
            pub has_prev_page: bool,
            /// Items remaining after the current page.
            pub remaining_items: i64,
        }

        impl $name {
            /// Build from a page of `data`, a 1-based `page` number, and the
            /// `total_items` count.  `page` is clamped to `1` if `0` is given.
            pub fn new(data: Vec<$item>, page: u32, total_items: i64) -> Self {
                let page = page.max(1);
                let total_pages = ((total_items as f64 / PAGE_SIZE as f64).ceil() as u32).max(1);
                Self {
                    data,
                    page,
                    page_size: PAGE_SIZE,
                    total_items,
                    total_pages,
                    has_next_page: page < total_pages,
                    has_prev_page: page > 1,
                    remaining_items: (total_items - page as i64 * PAGE_SIZE as i64).max(0),
                }
            }
        }
    };
}

paginated_response!(
    PaginatedTransactionResponse,
    Transaction,
    "Paginated response wrapper for financial transactions. Returned by `GET /api/v1/transactions`."
);
paginated_response!(
    PaginatedConsumptionResponse,
    Consumption,
    "Paginated response wrapper for fuel consumption events. Returned by `GET /api/v1/consumptions`."
);
paginated_response!(
    PaginatedActivityResponse,
    ActivityItem,
    "Paginated response wrapper for the unified activity feed. Returned by `GET /api/v1/activity`."
);

//  Query-parameter structs
//
// `TransactionQuery` and `ConsumptionQuery` share the same three fields.
// The macro generates distinct named types (required by utoipa).
// `ActivityQuery` adds a `kind` arm.

macro_rules! query_params {
    // Base variant: page + agent + station
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Default, Deserialize, IntoParams, ToSchema)]
        pub struct $name {
            /// Page number (1-based). Defaults to `1`.
            #[param(minimum = 1, default = 1, example = 1)]
            pub page: Option<u32>,
            /// Filter by agent reference code.
            #[param(example = "STORM-AGENT-0001")]
            pub agent: Option<String>,
            /// Filter by station (system-user UUID).
            pub station: Option<Uuid>,
        }
    };
    // Extended variant: page + kind + agent + station
    ($name:ident, $doc:literal, +kind) => {
        #[doc = $doc]
        #[derive(Debug, Default, Deserialize, IntoParams, ToSchema)]
        pub struct $name {
            /// Page number (1-based). Defaults to `1`.
            #[param(minimum = 1, default = 1, example = 1)]
            pub page: Option<u32>,
            /// Filter by event type: `"WITHDRAWAL"` or `"CONSUMPTION"`.
            #[param(example = "WITHDRAWAL")]
            pub kind: Option<String>,
            /// Filter by agent reference code.
            #[param(example = "STORM-AGENT-0001")]
            pub agent: Option<String>,
            /// Filter by station (system-user UUID).
            pub station: Option<Uuid>,
        }
    };
}

query_params!(
    TransactionQuery,
    "Query parameters for `GET /api/v1/transactions`."
);
query_params!(
    ConsumptionQuery,
    "Query parameters for `GET /api/v1/consumptions`."
);
query_params!(
    ActivityQuery,
    "Query parameters for `GET /api/v1/activity`.",
    +kind
);
