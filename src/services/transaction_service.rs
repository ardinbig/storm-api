//! Financial transaction business logic: listing and the withdrawal flow.
//!
//! The withdrawal flow is the most complex operation in the API.
//! 8 step atomic transaction that verifies the card password, checks the
//! agent, computes commission, validates sufficient balance, records the
//! transaction, credits the agent and house account, and debits the
//! customer's card.

use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{
        agent::HOUSE_ACCOUNT_REF,
        transaction::{Transaction, WithdrawalRequest, WithdrawalResponse},
    },
    services::auth_service,
    state::app_state::RedisPool,
    utils::cache,
};

/// SQL column list (with Postgres casts) reused across transaction queries.
const TX_COLUMNS: &str = "id, date, transaction_type, client_account, agent_account, \
     amount::FLOAT8 AS amount, currency_code, commission::FLOAT8 AS commission";

/// Lists all transactions, most recent first.
///
/// # Errors
///
/// Returns [`AppError::Database`] on query failure.
pub async fn list(pool: &PgPool) -> Result<Vec<Transaction>, AppError> {
    Ok(sqlx::query_as::<_, Transaction>(&format!(
        "SELECT {TX_COLUMNS} FROM transactions ORDER BY date DESC"
    ))
    .fetch_all(pool)
    .await?)
}

/// Lists transactions for a specific agent, most recent first.
///
/// # Errors
///
/// Returns [`AppError::Database`] on query failure.
pub async fn list_by_agent(pool: &PgPool, agent_ref: &str) -> Result<Vec<Transaction>, AppError> {
    Ok(sqlx::query_as::<_, Transaction>(&format!(
        "SELECT {TX_COLUMNS} FROM transactions WHERE agent_account = $1 ORDER BY date DESC"
    ))
    .bind(agent_ref)
    .fetch_all(pool)
    .await?)
}

// Pure helpers — extracted for unit-testing without a database
// ============================================================

/// Computes the commission amount for a withdrawal.
///
/// # Formula
///
/// `amount × percentage / 100`
pub fn calculate_commission(amount: f64, percentage: f64) -> f64 {
    amount * percentage / 100.0
}

/// Validates that the client balance can cover the withdrawal plus
/// commission.
///
/// # Errors
///
/// Returns [`AppError::BadRequest`] with message `"Insufficient balance"`
/// when `balance < withdrawal + commission`.
pub fn validate_sufficient_balance(
    balance: f64,
    withdrawal: f64,
    commission: f64,
) -> Result<(), AppError> {
    if balance < withdrawal + commission {
        return Err(AppError::BadRequest("Insufficient balance".into()));
    }
    Ok(())
}

// Withdrawal flow
// ===============

/// Performs an atomic withdrawal from a customer's NFC card to an agent
/// account.
///
/// ## Steps (inside a single SQL transaction)
///
/// 1. **Verify customer card** — look up `card_details` by NFC ref, verify
///    the supplied password.
/// 2. **Verify agent** — ensure the agent exists with the matching currency.
/// 3. **Fetch commission rate** — get the latest `commissions.percentage`.
/// 4. **Calculate commission** — `amount × percentage / 100`.
/// 5. **Check balance** — card must cover `withdrawal + commission`.
/// 6. **Record transaction** — insert into `transactions`.
/// 7. **Credit agent** — increase agent balance by the withdrawal amount.
/// 8. **Credit house account** — increase [`HOUSE_ACCOUNT_REF`] balance by
///    the commission.
/// 9. **Debit card** — decrease `card_details.amount` by `withdrawal +
///    commission`.
///
/// # Errors
///
/// - [`AppError::BadRequest`] — invalid card, password mismatch, agent not
///   found, or insufficient balance.
/// - [`AppError::Unauthorized`] — card has no password set.
/// - [`AppError::Internal`] — no commission rate configured.
/// - [`AppError::Database`] — any SQL error.
pub async fn withdrawal(
    pool: &PgPool,
    input: &WithdrawalRequest,
    redis: &RedisPool,
) -> Result<WithdrawalResponse, AppError> {
    let mut tx = pool.begin().await?;

    // 1. Verify customer card
    let (card_id, client_balance, stored_hash) = sqlx::query_as::<_, (Uuid, f64, Option<String>)>(
        "SELECT dc.id, dc.amount::FLOAT8, dc.password
         FROM card_details dc
         JOIN customers c ON c.card_id = dc.nfc_ref
         WHERE dc.nfc_ref = $1",
    )
    .bind(&input.client_code)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::BadRequest("Invalid card".into()))?;

    let hash = stored_hash.as_deref().ok_or(AppError::Unauthorized)?;
    if !auth_service::verify_password(&input.client_password, hash) {
        return Err(AppError::BadRequest("Invalid password".into()));
    }

    // 2. Verify agent exists with matching currency
    let (agent_id, agent_balance) = sqlx::query_as::<_, (Uuid, f64)>(
        "SELECT id, COALESCE(balance, 0)::FLOAT8
         FROM agent_accounts
         WHERE agent_ref = $1 AND currency_code = $2",
    )
    .bind(&input.agent_code)
    .bind(&input.currency_type)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::BadRequest("Agent not found for this currency".into()))?;

    // 3. Get latest commission percentage
    let (percentage,) = sqlx::query_as::<_, (f64,)>(
        "SELECT percentage::FLOAT8 FROM commissions ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::Internal)?;

    let commission = calculate_commission(input.withdrawal_amount, percentage);

    // 4. Check sufficient balance
    validate_sufficient_balance(client_balance, input.withdrawal_amount, commission)?;

    let total_deduction = input.withdrawal_amount + commission;

    // 5. Insert transaction record
    sqlx::query(
        "INSERT INTO transactions (id, transaction_type, client_account, agent_account, amount, currency_code, commission)
         VALUES ($1, 'WITHDRAWAL', $2, $3, $4, $5, $6)",
    )
    .bind(Uuid::new_v4())
    .bind(&input.client_code)
    .bind(&input.agent_code)
    .bind(input.withdrawal_amount)
    .bind(&input.currency_type)
    .bind(commission)
    .execute(&mut *tx)
    .await?;

    // 6. Credit agent account
    sqlx::query("UPDATE agent_accounts SET balance = balance + $1 WHERE id = $2")
        .bind(input.withdrawal_amount)
        .bind(agent_id)
        .execute(&mut *tx)
        .await?;

    // 7. Credit commission to house account
    sqlx::query("UPDATE agent_accounts SET balance = balance + $1 WHERE agent_ref = $2")
        .bind(commission)
        .bind(HOUSE_ACCOUNT_REF)
        .execute(&mut *tx)
        .await?;

    // 8. Deduct from customer card
    sqlx::query("UPDATE card_details SET amount = amount - $1 WHERE id = $2")
        .bind(total_deduction)
        .bind(card_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    // Invalidate cached card detail — balance has changed
    cache::del(redis, &cache::card_detail_key(&input.client_code)).await;

    Ok(WithdrawalResponse {
        message: "Withdrawal successful".into(),
        client_balance: client_balance - total_deduction,
        agent_balance: agent_balance + input.withdrawal_amount,
    })
}

// Unit tests (pure-function tests, no database or mocks required)
// ================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // calculate_commission

    #[test]
    fn commission_on_zero_amount() {
        assert_eq!(calculate_commission(0.0, 5.0), 0.0);
    }

    #[test]
    fn commission_on_normal_amount() {
        let c = calculate_commission(100.0, 5.0);
        assert!((c - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn commission_on_zero_percentage() {
        assert_eq!(calculate_commission(500.0, 0.0), 0.0);
    }

    #[test]
    fn commission_on_fractional_values() {
        let c = calculate_commission(333.33, 7.5);
        assert!((c - (333.33 * 7.5) / 100.0).abs() < 1e-10);
    }

    #[test]
    fn commission_on_large_amount() {
        let c = calculate_commission(1_000_000.0, 2.5);
        assert!((c - 25_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn commission_on_hundred_percent() {
        let c = calculate_commission(500.0, 100.0);
        assert!((c - 500.0).abs() < f64::EPSILON);
    }

    // validate_sufficient_balance

    #[test]
    fn sufficient_balance_ok() {
        assert!(validate_sufficient_balance(1000.0, 500.0, 25.0).is_ok());
    }

    #[test]
    fn sufficient_balance_exact() {
        assert!(validate_sufficient_balance(1000.0, 950.0, 50.0).is_ok());
    }

    #[test]
    fn sufficient_balance_with_zero_withdrawal() {
        assert!(validate_sufficient_balance(100.0, 0.0, 0.0).is_ok());
    }

    #[test]
    fn sufficient_balance_with_zero_balance_and_zero_amount() {
        assert!(validate_sufficient_balance(0.0, 0.0, 0.0).is_ok());
    }

    #[test]
    fn insufficient_balance() {
        assert!(matches!(
            validate_sufficient_balance(100.0, 90.0, 20.0),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn insufficient_balance_slightly_over() {
        assert!(matches!(
            validate_sufficient_balance(100.0, 100.0, 0.01),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn insufficient_balance_with_zero_balance() {
        assert!(matches!(
            validate_sufficient_balance(0.0, 1.0, 0.0),
            Err(AppError::BadRequest(_))
        ));
    }

    // withdrawal orchestration (pure computation, no mocks)

    #[test]
    fn withdrawal_happy_path_computation() {
        let client_balance = 10_000.0;
        let agent_balance = 500.0;
        let withdrawal_amount = 100.0;
        let percentage = 5.0;

        let commission = calculate_commission(withdrawal_amount, percentage);
        assert!((commission - 5.0).abs() < f64::EPSILON);

        validate_sufficient_balance(client_balance, withdrawal_amount, commission).unwrap();

        let new_client = client_balance - withdrawal_amount - commission;
        let new_agent = agent_balance + withdrawal_amount;

        assert!((new_client - 9895.0).abs() < f64::EPSILON);
        assert!((new_agent - 600.0).abs() < f64::EPSILON);
    }

    #[test]
    fn withdrawal_insufficient_funds() {
        let client_balance = 50.0;
        let withdrawal_amount = 100.0;
        let commission = calculate_commission(withdrawal_amount, 10.0);

        assert!(matches!(
            validate_sufficient_balance(client_balance, withdrawal_amount, commission),
            Err(AppError::BadRequest(_))
        ));
    }
}
