//! Unit tests for P9.5 encrypted notification privacy policy.

use super::*;

#[test]
fn marker_stable() {
    assert_eq!(
        matrix_notification_privacy_markers(),
        MATRIX_NOTIFICATION_PRIVACY_MARKER
    );
}

#[test]
fn encrypted_unlocked_shows_sender_only() {
    let decision = apply_policy(CandidateMeta::new(false), true, false);

    assert_eq!(
        decision,
        PrivacyDecision {
            allowed: true,
            mode: EncryptionPrivacyMode::ShowSenderOnly,
            reason_diagnostic_id: REASON_ENCRYPTED_SENDER_ONLY,
        }
    );
}

#[test]
fn encrypted_unlocked_does_not_require_lock_screen_preview() {
    let without_safe_preview = apply_policy(CandidateMeta::new(false), true, false);
    let with_safe_preview = apply_policy(CandidateMeta::new(true), true, false);

    assert_eq!(without_safe_preview, with_safe_preview);
}

#[test]
fn locked_unsafe_preview_is_suppressed_for_every_room_kind() {
    for room_is_encrypted in [false, true] {
        let decision = apply_policy(CandidateMeta::new(false), room_is_encrypted, true);

        assert!(!decision.allowed);
        assert_eq!(decision.mode, EncryptionPrivacyMode::Suppress);
        assert_eq!(
            decision.reason_diagnostic_id,
            REASON_LOCKED_PREVIEW_NOT_SAFE
        );
    }
}

#[test]
fn locked_safe_preview_is_redacted_for_every_room_kind() {
    for room_is_encrypted in [false, true] {
        let decision = apply_policy(CandidateMeta::new(true), room_is_encrypted, true);

        assert!(decision.allowed);
        assert_eq!(decision.mode, EncryptionPrivacyMode::ShowRedacted);
        assert_eq!(decision.reason_diagnostic_id, REASON_LOCKED_SAFE_REDACTED);
    }
}

#[test]
fn unencrypted_unlocked_preview_is_redacted() {
    let decision = apply_policy(CandidateMeta::new(false), false, false);

    assert!(decision.allowed);
    assert_eq!(decision.mode, EncryptionPrivacyMode::ShowRedacted);
    assert_eq!(decision.reason_diagnostic_id, REASON_UNENCRYPTED_REDACTED);
}

#[test]
fn decision_debug_contains_only_policy_metadata() {
    let debug = format!("{:?}", apply_policy(CandidateMeta::new(false), true, true));

    assert_eq!(
        debug,
        "PrivacyDecision { allowed: false, mode: Suppress, \
         reason_diagnostic_id: \"p9.5-locked-preview-not-safe\" }"
    );
}
