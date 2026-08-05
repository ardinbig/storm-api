use storm_api::middleware::idempotency::{IdempotencyTelemetry, NO_MATCHED_PATH_FALLBACK_MARKER};

/// Asserts the canonical key layout for idempotency telemetry records.
pub fn assert_canonical_idempotency_log_layout(telemetry: &IdempotencyTelemetry<'_>) {
    let actual_keys = telemetry.canonical_field_names();

    if telemetry.user_id.is_some() {
        assert_eq!(
            actual_keys.as_slice(),
            &["event", "reason", "path", "user_id"]
        );
    } else {
        assert_eq!(actual_keys.as_slice(), &["event", "reason", "path"]);
    }
}

/// Returns the fallback marker used by idempotency telemetry tests.
pub fn fallback_marker() -> &'static str {
    NO_MATCHED_PATH_FALLBACK_MARKER
}
