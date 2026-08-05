use crate::common::{assert_canonical_idempotency_log_layout, fallback_marker};
use storm_api::middleware::idempotency::{IdempotencyEvent, IdempotencyTelemetry};

#[test]
fn no_matched_path_fallback_marker_is_stable_and_exact() {
    assert_eq!(fallback_marker(), "{METHOD}::NO_MATCHED_PATH");
}

#[test]
fn idempotency_telemetry_uses_a_canonical_key_layout() {
    let with_user = IdempotencyTelemetry::new(
        IdempotencyEvent::MalformedHeader,
        "duplicate idempotency header",
        "POST::NO_MATCHED_PATH",
        Some("user-1"),
    );
    let without_user = IdempotencyTelemetry::new(
        IdempotencyEvent::CacheMiss,
        "cache miss",
        "POST::NO_MATCHED_PATH",
        None::<&str>,
    );

    assert_canonical_idempotency_log_layout(&with_user);
    assert_canonical_idempotency_log_layout(&without_user);
}
