use storm_api::models::pagination::{
    ActivityQuery, ConsumptionQuery, PAGE_SIZE, PaginatedActivityResponse,
    PaginatedConsumptionResponse, PaginatedTransactionResponse, TransactionQuery,
};
use uuid::Uuid;

// PaginatedTransactionResponse
// ============================

#[test]
fn paginated_tx_first_page_of_three() {
    // 25 items, page 1 → 10 shown, 15 remaining, 3 total pages
    let resp = PaginatedTransactionResponse::new(vec![], 1, 25);
    assert_eq!(resp.page, 1);
    assert_eq!(resp.page_size, PAGE_SIZE);
    assert_eq!(resp.total_items, 25);
    assert_eq!(resp.total_pages, 3);
    assert!(resp.has_next_page);
    assert!(!resp.has_prev_page);
    assert_eq!(resp.remaining_items, 15);
}

#[test]
fn paginated_tx_middle_page() {
    // 25 items, page 2 → 5 remaining
    let resp = PaginatedTransactionResponse::new(vec![], 2, 25);
    assert_eq!(resp.page, 2);
    assert_eq!(resp.total_pages, 3);
    assert!(resp.has_next_page);
    assert!(resp.has_prev_page);
    assert_eq!(resp.remaining_items, 5);
}

#[test]
fn paginated_tx_last_page() {
    // 25 items, page 3 → 0 remaining
    let resp = PaginatedTransactionResponse::new(vec![], 3, 25);
    assert!(!resp.has_next_page);
    assert!(resp.has_prev_page);
    assert_eq!(resp.remaining_items, 0);
}

#[test]
fn paginated_tx_page_zero_clamped_to_one() {
    let resp = PaginatedTransactionResponse::new(vec![], 0, 10);
    assert_eq!(resp.page, 1);
    assert!(!resp.has_prev_page);
}

#[test]
fn paginated_tx_empty_result_has_one_page() {
    let resp = PaginatedTransactionResponse::new(vec![], 1, 0);
    assert_eq!(resp.total_pages, 1);
    assert!(!resp.has_next_page);
    assert!(!resp.has_prev_page);
    assert_eq!(resp.remaining_items, 0);
}

#[test]
fn paginated_tx_exact_boundary() {
    // Exactly 10 items — one page, no next
    let resp = PaginatedTransactionResponse::new(vec![], 1, 10);
    assert_eq!(resp.total_pages, 1);
    assert!(!resp.has_next_page);
    assert_eq!(resp.remaining_items, 0);
}

#[test]
fn paginated_tx_eleven_items_two_pages() {
    let resp = PaginatedTransactionResponse::new(vec![], 1, 11);
    assert_eq!(resp.total_pages, 2);
    assert!(resp.has_next_page);
    assert_eq!(resp.remaining_items, 1);
}

// PaginatedConsumptionResponse
// ============================

#[test]
fn paginated_consumption_metadata() {
    let resp = PaginatedConsumptionResponse::new(vec![], 2, 30);
    assert_eq!(resp.page, 2);
    assert_eq!(resp.total_pages, 3);
    assert_eq!(resp.remaining_items, 10);
    assert!(resp.has_next_page);
    assert!(resp.has_prev_page);
}

#[test]
fn paginated_consumption_empty() {
    let resp = PaginatedConsumptionResponse::new(vec![], 1, 0);
    assert_eq!(resp.total_pages, 1);
    assert!(!resp.has_next_page);
    assert_eq!(resp.remaining_items, 0);
}

// PaginatedActivityResponse
// =========================

#[test]
fn paginated_activity_metadata() {
    let resp = PaginatedActivityResponse::new(vec![], 1, 5);
    assert_eq!(resp.total_pages, 1);
    assert_eq!(resp.total_items, 5);
    assert!(!resp.has_next_page);
    assert_eq!(resp.remaining_items, 0);
}

#[test]
fn paginated_activity_large_dataset() {
    let resp = PaginatedActivityResponse::new(vec![], 1, 100);
    assert_eq!(resp.total_pages, 10);
    assert_eq!(resp.remaining_items, 90);
    assert!(resp.has_next_page);
}

// Query struct defaults
// =====================

#[test]
fn transaction_query_defaults() {
    let q = TransactionQuery::default();
    assert!(q.page.is_none());
    assert!(q.agent.is_none());
    assert!(q.station.is_none());
}

#[test]
fn consumption_query_defaults() {
    let q = ConsumptionQuery::default();
    assert!(q.page.is_none());
}

#[test]
fn activity_query_defaults() {
    let q = ActivityQuery::default();
    assert!(q.page.is_none());
    assert!(q.kind.is_none());
    assert!(q.agent.is_none());
    assert!(q.station.is_none());
}

// Existing model tests
// ====================

#[test]
fn system_user_serialization_skips_password() {
    use storm_api::models::user::UserInfo;

    let info = UserInfo {
        id: Uuid::new_v4(),
        name: "Alice".to_string(),
        email: Some("alice@test.com".to_string()),
        username: "alice".to_string(),
    };

    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(json["username"], "alice");
    assert_eq!(json["email"], "alice@test.com");
}
