//! Pure protocol + fixture compatibility tests for Matrix IPC (P1.3).

use super::*;
use serde_json::json;

/// Fixture root relative to `src-tauri` package (`CARGO_MANIFEST_DIR`).
fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../docs/matrix-rust-sdk/ipc/fixtures")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("failed to read fixture {}: {e}", path.display());
    })
}

#[test]
fn protocol_version_is_one() {
    assert_eq!(MATRIX_IPC_PROTOCOL_VERSION, 1);
    assert!(FORBID_MEDIA_BYTES_OVER_JSON_IPC);
    assert!(MAX_STREAM_QUEUE_DEPTH > 0);
    assert!(MAX_ENVELOPE_PAYLOAD_JSON_BYTES > 0);
}

#[test]
fn kinds_are_exhaustive_and_stable() {
    assert_eq!(MATRIX_IPC_KINDS.len(), 13);
    assert!(MATRIX_IPC_KINDS.contains(&KIND_HELLO));
    assert!(MATRIX_IPC_KINDS.contains(&KIND_RESYNC_REQUIRED));
    assert!(MATRIX_IPC_KINDS.contains(&KIND_ERROR));
}

#[test]
fn error_categories_round_trip() {
    for category in MatrixIpcErrorCategory::ALL {
        let err = MatrixIpcError::new(*category)
            .with_diagnostic("diag-test-001")
            .with_retry_after_ms(1000);
        let value = serde_json::to_value(&err).expect("serialize");
        let back: MatrixIpcError = serde_json::from_value(value).expect("deserialize");
        assert_eq!(back.category, *category);
        assert_eq!(back.diagnostic_id.as_deref(), Some("diag-test-001"));
        // Privacy: no secret-bearing keys in serialized form.
        let raw = serde_json::to_string(&err).unwrap();
        assert!(!raw.contains("access_token"));
        assert!(!raw.contains("recovery_key"));
        assert!(!raw.contains("password"));
    }
}

#[test]
fn hello_envelope_round_trip() {
    let env = MatrixIpcEnvelope::new(
        1,
        0,
        MatrixIpcMessage::Hello(HelloPayload {
            client_protocol_version: MATRIX_IPC_PROTOCOL_VERSION,
            client_name: Some("synara-web".into()),
        }),
    )
    .with_request_id("req-hello-1");

    let json = serde_json::to_string(&env).unwrap();
    let back = MatrixIpcEnvelope::from_json_str(&json).unwrap();
    assert_eq!(back.kind(), KIND_HELLO);
    assert_eq!(back.protocol_version, 1);
    assert_eq!(back.request_id.as_deref(), Some("req-hello-1"));
}

#[test]
fn fixture_valid_hello() {
    let env = MatrixIpcEnvelope::from_json_str(&fixture("valid_hello.json")).unwrap();
    assert_eq!(env.kind(), KIND_HELLO);
    assert_eq!(env.protocol_version, MATRIX_IPC_PROTOCOL_VERSION);
    assert_eq!(env.session_generation, 1);
    assert_eq!(env.sequence, 0);
}

#[test]
fn fixture_valid_hello_ack() {
    let env = MatrixIpcEnvelope::from_json_str(&fixture("valid_hello_ack.json")).unwrap();
    assert_eq!(env.kind(), KIND_HELLO_ACK);
}

#[test]
fn fixture_valid_subscribe_snapshot_delta() {
    let sub = MatrixIpcEnvelope::from_json_str(&fixture("valid_subscribe.json")).unwrap();
    assert_eq!(sub.kind(), KIND_SUBSCRIBE);
    assert_eq!(sub.stream_id.as_deref(), Some("stream-room-list-1"));

    let snap = MatrixIpcEnvelope::from_json_str(&fixture("valid_snapshot.json")).unwrap();
    assert_eq!(snap.kind(), KIND_SNAPSHOT);
    assert_eq!(snap.sequence, 1);

    let delta = MatrixIpcEnvelope::from_json_str(&fixture("valid_delta.json")).unwrap();
    assert_eq!(delta.kind(), KIND_DELTA);
    assert_eq!(delta.sequence, 2);
}

#[test]
fn fixture_valid_error_rate_limited() {
    let env = MatrixIpcEnvelope::from_json_str(&fixture("valid_error_rate_limited.json")).unwrap();
    assert_eq!(env.kind(), KIND_ERROR);
    match env.message {
        MatrixIpcMessage::Error(err) => {
            assert_eq!(err.category, MatrixIpcErrorCategory::RateLimited);
            assert_eq!(err.retry_after_ms, Some(5000));
            assert!(err.diagnostic_id.is_some());
        }
        other => panic!("expected error message, got {:?}", other.kind()),
    }
}

#[test]
fn fixture_valid_resync_required() {
    let env = MatrixIpcEnvelope::from_json_str(&fixture("valid_resync_required.json")).unwrap();
    assert_eq!(env.kind(), KIND_RESYNC_REQUIRED);
}

#[test]
fn fixture_invalid_unknown_kind_rejected() {
    let raw = fixture("invalid_unknown_kind.json");
    let result = MatrixIpcEnvelope::from_json_str(&raw);
    assert!(
        result.is_err(),
        "unknown kind must be rejected at boundary"
    );
}

#[test]
fn fixture_invalid_missing_protocol_version_rejected() {
    let raw = fixture("invalid_missing_protocol_version.json");
    let result = MatrixIpcEnvelope::from_json_str(&raw);
    assert!(result.is_err(), "missing protocolVersion must fail");
}

#[test]
fn stale_generation_rejected() {
    check_session_generation(5, 5).unwrap();
    let err = check_session_generation(5, 4).unwrap_err();
    assert_eq!(err.category, MatrixIpcErrorCategory::StaleSessionGeneration);
}

#[test]
fn protocol_version_check() {
    check_protocol_version(1).unwrap();
    let err = check_protocol_version(99).unwrap_err();
    assert_eq!(err.category, MatrixIpcErrorCategory::UnsupportedCapability);
}

#[test]
fn sequence_accept_duplicate_gap() {
    // First message after subscribe
    assert_eq!(
        check_sequence(None, 1),
        SequenceOutcome::Accept {
            next_last_applied: 1
        }
    );

    // Ordered delta
    assert_eq!(
        check_sequence(Some(1), 2),
        SequenceOutcome::Accept {
            next_last_applied: 2
        }
    );

    // Duplicate (idempotent)
    assert_eq!(
        check_sequence(Some(2), 2),
        SequenceOutcome::Duplicate { last_applied: 2 }
    );

    // Gap
    assert_eq!(
        check_sequence(Some(2), 5),
        SequenceOutcome::Gap {
            last_applied: 2,
            observed: 5
        }
    );

    // Behind
    assert_eq!(
        check_sequence(Some(5), 3),
        SequenceOutcome::Behind {
            last_applied: 5,
            observed: 3
        }
    );
}

#[test]
fn snapshot_then_ordered_deltas_model() {
    // Snapshot establishes baseline sequence 1.
    let mut last = None;
    let snap_seq = 1u64;
    match check_sequence(last, snap_seq) {
        SequenceOutcome::Accept {
            next_last_applied,
        } => last = Some(next_last_applied),
        other => panic!("snapshot should accept: {other:?}"),
    }
    assert_eq!(last, Some(1));

    // Deltas 2, 3
    for seq in [2u64, 3] {
        match check_sequence(last, seq) {
            SequenceOutcome::Accept {
                next_last_applied,
            } => last = Some(next_last_applied),
            other => panic!("delta {seq} should accept: {other:?}"),
        }
    }
    assert_eq!(last, Some(3));

    // Duplicate of 3 is idempotent
    assert!(matches!(
        check_sequence(last, 3),
        SequenceOutcome::Duplicate { .. }
    ));

    // Gap at 6 requires resync
    let (outcome, event) = apply_delta_sequence(last, 6);
    assert!(matches!(outcome, SequenceOutcome::Gap { .. }));
    assert_eq!(event, Some(StreamLifecycleEvent::ResyncNeeded));

    let payload = resync_payload_for_gap("stream-1", 3, 6);
    assert_eq!(payload.reason, ResyncReason::SequenceGap);
}

#[test]
fn stream_lifecycle_transitions() {
    use StreamLifecycleEvent as E;
    use StreamLifecycleState as S;

    let mut state = S::Idle;
    state = transition_stream_lifecycle(state, E::SubscribeRequested).unwrap();
    assert_eq!(state, S::Subscribing);
    state = transition_stream_lifecycle(state, E::SubscribedAck).unwrap();
    assert_eq!(state, S::SnapshotPending);
    state = transition_stream_lifecycle(state, E::SnapshotApplied).unwrap();
    assert_eq!(state, S::Live);
    state = transition_stream_lifecycle(state, E::DeltaApplied).unwrap();
    assert_eq!(state, S::Live);
    state = transition_stream_lifecycle(state, E::ResyncNeeded).unwrap();
    assert_eq!(state, S::ResyncRequired);
    state = transition_stream_lifecycle(state, E::SubscribeRequested).unwrap();
    assert_eq!(state, S::Subscribing);
    // Unsubscribe path
    state = transition_stream_lifecycle(state, E::SubscribedAck).unwrap();
    state = transition_stream_lifecycle(state, E::SnapshotApplied).unwrap();
    state = transition_stream_lifecycle(state, E::UnsubscribeRequested).unwrap();
    assert_eq!(state, S::Unsubscribing);
    state = transition_stream_lifecycle(state, E::ResourcesReleased).unwrap();
    assert_eq!(state, S::Closed);

    // Illegal transition
    assert!(transition_stream_lifecycle(S::Idle, E::DeltaApplied).is_none());
}

#[test]
fn cancel_and_ping_round_trip() {
    let cancel = MatrixIpcEnvelope::new(
        1,
        10,
        MatrixIpcMessage::Cancel(CancelPayload {
            cancellation_token: "cancel-token-abc".into(),
            reason: Some(CancelReason::ClientRequest),
        }),
    );
    let json = serde_json::to_string(&cancel).unwrap();
    let back = MatrixIpcEnvelope::from_json_str(&json).unwrap();
    assert_eq!(back.kind(), KIND_CANCEL);

    let ping = MatrixIpcEnvelope::new(
        1,
        11,
        MatrixIpcMessage::Ping(PingPayload {
            nonce: Some("n1".into()),
        }),
    );
    let pong = MatrixIpcEnvelope::new(
        1,
        12,
        MatrixIpcMessage::Pong(PongPayload {
            nonce: Some("n1".into()),
        }),
    );
    assert_eq!(ping.kind(), KIND_PING);
    assert_eq!(pong.kind(), KIND_PONG);
}

#[test]
fn unknown_kind_inline_json_rejected() {
    let bad = json!({
        "protocolVersion": 1,
        "sessionGeneration": 1,
        "sequence": 0,
        "kind": "not_a_real_kind",
        "payload": {}
    });
    assert!(MatrixIpcEnvelope::from_json_value(bad).is_err());
}

#[test]
fn stream_topics_stable() {
    for topic in StreamTopic::ALL {
        let v = serde_json::to_value(topic).unwrap();
        assert_eq!(v.as_str().unwrap(), topic.as_str());
        let back: StreamTopic = serde_json::from_value(v).unwrap();
        assert_eq!(back, *topic);
    }
}
