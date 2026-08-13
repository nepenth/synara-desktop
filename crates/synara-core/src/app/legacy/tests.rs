//! Unit tests for P3.7 legacy transition coordinator.

use super::*;
use crate::transport::MatrixIpcErrorCategory;

fn signal(kind: LegacySignalKind, label: &str) -> LegacyDetectionSignal {
    LegacyDetectionSignal {
        kind,
        label: label.into(),
    }
}

#[test]
fn marker_and_policy() {
    assert_eq!(matrix_legacy_markers(), MATRIX_LEGACY_MARKER);
    let t = LegacyTransition::new(1);
    assert!(t.forbids_js_client_start());
    assert!(t.forbids_token_continuity());
    assert!(t.forbids_dual_backend());
}

#[test]
fn detect_reauth_complete() {
    let mut t = LegacyTransition::new(1);
    t.apply_detection(
        vec![
            signal(LegacySignalKind::WebSyncStore, "web-sync-store"),
            signal(LegacySignalKind::JsCryptoStore, "crypto-store"),
        ],
        true,
    )
    .unwrap();
    assert!(t.legacy_detected());
    assert!(t.legacy_data_retained());
    assert_eq!(t.phase(), TransitionPhase::AwaitingReauth);
    assert!(!t.copy_keys_for_phase().is_empty());

    let op = t.begin_reauth().unwrap();
    assert_eq!(t.phase(), TransitionPhase::Reauthing);
    t.mark_establishing(op).unwrap();
    assert_eq!(t.phase(), TransitionPhase::EstablishingRustSession);
    t.complete(op).unwrap();
    assert_eq!(t.phase(), TransitionPhase::Complete);
    // Inert legacy may still be retained until cleanup.
    assert!(t.legacy_data_retained());
    t.mark_legacy_cleaned().unwrap();
    assert!(!t.legacy_data_retained());
    assert!(!t.legacy_detected());
}

#[test]
fn fail_preserves_legacy() {
    let mut t = LegacyTransition::new(1);
    t.apply_detection(
        vec![signal(
            LegacySignalKind::LegacyCredentialEnvelope,
            "legacy-envelope",
        )],
        true,
    )
    .unwrap();
    let op = t.begin_reauth().unwrap();
    t.fail(op, "p3.7-login-cancelled").unwrap();
    assert_eq!(t.phase(), TransitionPhase::Failed);
    assert!(t.legacy_data_retained());
    assert_eq!(t.failure_diagnostic_id(), Some("p3.7-login-cancelled"));
    // Retry allowed from Failed.
    let op2 = t.begin_reauth().unwrap();
    t.complete(op2).unwrap();
    assert_eq!(t.phase(), TransitionPhase::Complete);
}

#[test]
fn secret_label_rejected_and_cap() {
    let mut t = LegacyTransition::new(1);
    let err = t
        .apply_detection(
            vec![signal(LegacySignalKind::Other, "access_token=syt_xxx")],
            false,
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p3.7-signal-looks-like-secret");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);

    let many: Vec<_> = (0..MAX_DETECTION_SIGNALS + 1)
        .map(|i| signal(LegacySignalKind::Other, &format!("sig-{i}")))
        .collect();
    let err = t.apply_detection(many, false).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p3.7-signal-cap");
}

#[test]
fn defer_and_stale_op() {
    let mut t = LegacyTransition::new(1);
    t.apply_detection(
        vec![signal(LegacySignalKind::CutoverMarkerAbsent, "no-cutover")],
        true,
    )
    .unwrap();
    t.defer().unwrap();
    assert_eq!(t.phase(), TransitionPhase::Deferred);
    let op = t.begin_reauth().unwrap();
    let err = t.complete(op + 1).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p3.7-stale-op-id");
}

#[test]
fn retire_generation() {
    let mut t = LegacyTransition::new(2);
    t.apply_detection(
        vec![signal(LegacySignalKind::WebSyncStore, "web-sync-store")],
        true,
    )
    .unwrap();
    t.retire_generation(3);
    assert_eq!(t.session_generation(), 3);
    assert_eq!(t.phase(), TransitionPhase::Idle);
    assert!(!t.legacy_detected());
}
