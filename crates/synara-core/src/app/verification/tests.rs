//! Unit tests for P8.3 verification inbox.

use super::*;
use crate::transport::MatrixIpcErrorCategory;

fn flow(id: &str, phase: VerificationPhase) -> VerificationFlow {
    VerificationFlow {
        flow_id: id.into(),
        other_user_id: "@alice:example.org".into(),
        other_device_id: "DEVICEA".into(),
        direction: VerificationDirection::Incoming,
        phase,
        started_ts: Some(1),
        sas_emoji: None,
        failure_diagnostic_id: None,
    }
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_verification_markers(), MATRIX_VERIFICATION_MARKER);
}

#[test]
fn upsert_list_open_order() {
    let mut inbox = VerificationInbox::new(1);
    inbox
        .upsert(flow("f-ready", VerificationPhase::Ready))
        .unwrap();
    inbox
        .upsert(flow("f-req", VerificationPhase::Requested))
        .unwrap();
    inbox
        .upsert(flow("f-done", VerificationPhase::Done))
        .unwrap();
    let open = inbox.list_open();
    assert_eq!(open.len(), 2);
    assert_eq!(open[0].flow_id, "f-req");
    assert_eq!(open[1].flow_id, "f-ready");
    assert!(inbox.has_pending_attention());
    assert_eq!(inbox.open_count(), 2);
}

#[test]
fn sas_confirm_complete() {
    let mut inbox = VerificationInbox::new(1);
    inbox
        .upsert(flow("f1", VerificationPhase::Requested))
        .unwrap();
    inbox
        .mark_ready(
            "f1",
            Some(vec![
                "dog".into(),
                "cat".into(),
                "tree".into(),
                "book".into(),
                "clock".into(),
                "smiley".into(),
                "heart".into(),
            ]),
        )
        .unwrap();
    let f = inbox.get("f1").unwrap();
    assert_eq!(f.phase, VerificationPhase::Ready);
    assert_eq!(f.sas_emoji.as_ref().unwrap().len(), 7);
    inbox.confirm("f1").unwrap();
    assert_eq!(inbox.get("f1").unwrap().phase, VerificationPhase::Confirmed);
    inbox.complete("f1").unwrap();
    let f = inbox.get("f1").unwrap();
    assert_eq!(f.phase, VerificationPhase::Done);
    assert!(f.sas_emoji.is_none());
    assert!(!inbox.has_pending_attention());
}

#[test]
fn mismatch_cancel_fail() {
    let mut inbox = VerificationInbox::new(1);
    inbox
        .upsert(flow("a", VerificationPhase::Requested))
        .unwrap();
    inbox.mark_ready("a", None).unwrap();
    inbox.mismatch("a").unwrap();
    assert_eq!(inbox.get("a").unwrap().phase, VerificationPhase::Mismatched);
    inbox.cancel("a").unwrap();
    assert_eq!(inbox.get("a").unwrap().phase, VerificationPhase::Cancelled);

    inbox
        .upsert(flow("b", VerificationPhase::Requested))
        .unwrap();
    inbox.fail("b", "p8.3-host-timeout").unwrap();
    let f = inbox.get("b").unwrap();
    assert_eq!(f.phase, VerificationPhase::Failed);
    assert_eq!(f.failure_diagnostic_id, Some("p8.3-host-timeout"));
}

#[test]
fn invalid_ids_and_phase() {
    let mut inbox = VerificationInbox::new(1);
    let err = inbox
        .upsert(VerificationFlow {
            flow_id: "".into(),
            other_user_id: "@a:ex".into(),
            other_device_id: "D".into(),
            direction: VerificationDirection::Outgoing,
            phase: VerificationPhase::Requested,
            started_ts: None,
            sas_emoji: None,
            failure_diagnostic_id: None,
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.3-invalid-flow-id");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);

    let err = inbox
        .upsert(VerificationFlow {
            flow_id: "ok".into(),
            other_user_id: "not-a-mxid".into(),
            other_device_id: "D".into(),
            direction: VerificationDirection::Outgoing,
            phase: VerificationPhase::Requested,
            started_ts: None,
            sas_emoji: None,
            failure_diagnostic_id: None,
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.3-invalid-user-id");

    inbox
        .upsert(flow("f", VerificationPhase::Requested))
        .unwrap();
    let err = inbox.confirm("f").unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.3-invalid-phase-transition");
}

#[test]
fn open_cap() {
    let mut inbox = VerificationInbox::new(1);
    for i in 0..MAX_OPEN_FLOWS {
        inbox
            .upsert(flow(&format!("f{i}"), VerificationPhase::Requested))
            .unwrap();
    }
    let err = inbox
        .upsert(flow("overflow", VerificationPhase::Requested))
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.3-open-flow-cap");
    // Terminal flows do not count toward open cap.
    inbox
        .upsert(flow("done-extra", VerificationPhase::Done))
        .unwrap();
}

#[test]
fn retire_generation() {
    let mut inbox = VerificationInbox::new(2);
    inbox.upsert(flow("x", VerificationPhase::Ready)).unwrap();
    inbox.retire_generation(3);
    assert_eq!(inbox.session_generation(), 3);
    assert!(inbox.is_empty());
    assert!(!inbox.has_pending_attention());
}

#[test]
fn self_verification_start_falls_back_to_a_peer_device() {
    let source = include_str!("live.rs");
    assert!(source.contains("fn start_self_verification"));
    assert!(source.contains("query_own_identity"));
    assert!(source.contains("get_user_devices"));
    assert!(source.contains("v-crypto.1-no-peer-device"));
    let helper = source
        .split("async fn start_self_verification")
        .nth(1)
        .and_then(|rest| rest.split("async fn register_incoming_request").next())
        .expect("start_self_verification helper");
    let identity = helper.find("query_own_identity").expect("identity first");
    let peer = helper.find("get_user_devices").expect("peer fallback");
    assert!(
        identity < peer,
        "own identity must be tried before a peer device"
    );
}

#[test]
fn verification_update_signal_is_session_wake_only() {
    assert_eq!(VERIFICATION_UPDATED_EVENT, "matrix-verification-updated");
    let encoded = serde_json::to_string(&NativeVerificationUpdateSignal {
        session_generation: 7,
    })
    .expect("serialize wake-up");
    assert_eq!(encoded, "{\"sessionGeneration\":7}");
    for forbidden in ["key", "token", "mac", "secret", "recovery", "ciphertext"] {
        assert!(!encoded.to_ascii_lowercase().contains(forbidden));
    }
}
