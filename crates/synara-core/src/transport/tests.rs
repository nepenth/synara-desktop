//! Pure protocol + fixture + contract tests for Matrix IPC (P1.3 + P1.5).

use super::*;
use serde_json::json;

/// Fixture root relative to this package (`CARGO_MANIFEST_DIR`). The fixtures
/// live at the repository root under `docs/matrix-rust-sdk/ipc/fixtures`.
fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/matrix-rust-sdk/ipc/fixtures")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("failed to read fixture {}: {e}", path.display());
    })
}

/// Schema catalog root (shared Rust/TS compatibility oracle).
fn schema_catalog() -> serde_json::Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/matrix-rust-sdk/ipc/schema_catalog_v1.json");
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("failed to read schema catalog {}: {e}", path.display());
    });
    serde_json::from_str(&raw).expect("schema catalog must be valid JSON")
}

#[test]
fn protocol_version_is_one() {
    assert_eq!(MATRIX_IPC_PROTOCOL_VERSION, 1);
    const { assert!(FORBID_MEDIA_BYTES_OVER_JSON_IPC) };
    const { assert!(MAX_STREAM_QUEUE_DEPTH > 0) };
    const { assert!(MAX_ENVELOPE_PAYLOAD_JSON_BYTES > 0) };
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
    assert!(result.is_err(), "unknown kind must be rejected at boundary");
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
        SequenceOutcome::Accept { next_last_applied } => last = Some(next_last_applied),
        other => panic!("snapshot should accept: {other:?}"),
    }
    assert_eq!(last, Some(1));

    // Deltas 2, 3
    for seq in [2u64, 3] {
        match check_sequence(last, seq) {
            SequenceOutcome::Accept { next_last_applied } => last = Some(next_last_applied),
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

// ---------------------------------------------------------------------------
// P1.5 — expanded contract suite
// ---------------------------------------------------------------------------

/// Every control kind serializes and deserializes without loss of kind/fields.
#[test]
fn all_control_kinds_json_round_trip() {
    let samples: Vec<MatrixIpcEnvelope> = vec![
        MatrixIpcEnvelope::new(
            1,
            0,
            MatrixIpcMessage::Hello(HelloPayload {
                client_protocol_version: MATRIX_IPC_PROTOCOL_VERSION,
                client_name: Some("synara-web".into()),
            }),
        )
        .with_request_id("req-hello"),
        MatrixIpcEnvelope::new(
            1,
            0,
            MatrixIpcMessage::HelloAck(HelloAckPayload {
                protocol_version: MATRIX_IPC_PROTOCOL_VERSION,
                session_generation: 1,
            }),
        ),
        MatrixIpcEnvelope::new(
            1,
            0,
            MatrixIpcMessage::Subscribe(SubscribePayload {
                topic: StreamTopic::RoomList,
                stream_id: "s1".into(),
                params: Some(json!({})),
            }),
        )
        .with_stream_id("s1"),
        MatrixIpcEnvelope::new(
            1,
            0,
            MatrixIpcMessage::Unsubscribe(UnsubscribePayload {
                stream_id: "s1".into(),
            }),
        )
        .with_stream_id("s1"),
        MatrixIpcEnvelope::new(
            1,
            0,
            MatrixIpcMessage::Subscribed(SubscribedPayload {
                stream_id: "s1".into(),
                topic: StreamTopic::RoomList,
            }),
        )
        .with_stream_id("s1"),
        MatrixIpcEnvelope::new(
            1,
            0,
            MatrixIpcMessage::Unsubscribed(UnsubscribedPayload {
                stream_id: "s1".into(),
                resources_released: true,
            }),
        )
        .with_stream_id("s1"),
        MatrixIpcEnvelope::new(
            1,
            1,
            MatrixIpcMessage::Snapshot(SnapshotPayload {
                stream_id: "s1".into(),
                topic: StreamTopic::Timeline,
                snapshot_id: "snap-1".into(),
                body: json!({"items": []}),
            }),
        )
        .with_stream_id("s1"),
        MatrixIpcEnvelope::new(
            1,
            2,
            MatrixIpcMessage::Delta(DeltaPayload {
                stream_id: "s1".into(),
                topic: StreamTopic::Timeline,
                idempotency_key: Some("idem-1".into()),
                body: json!({"items": []}),
            }),
        )
        .with_stream_id("s1"),
        MatrixIpcEnvelope::new(
            1,
            0,
            MatrixIpcMessage::ResyncRequired(resync_payload_for_gap("s1", 2, 5)),
        )
        .with_stream_id("s1"),
        MatrixIpcEnvelope::new(
            1,
            0,
            MatrixIpcMessage::Cancel(CancelPayload {
                cancellation_token: "tok".into(),
                reason: Some(CancelReason::Timeout),
            }),
        ),
        MatrixIpcEnvelope::new(
            1,
            0,
            MatrixIpcMessage::Error(
                MatrixIpcError::new(MatrixIpcErrorCategory::Connectivity)
                    .with_diagnostic("diag-net"),
            ),
        ),
        MatrixIpcEnvelope::new(
            1,
            0,
            MatrixIpcMessage::Ping(PingPayload {
                nonce: Some("n".into()),
            }),
        ),
        MatrixIpcEnvelope::new(
            1,
            0,
            MatrixIpcMessage::Pong(PongPayload {
                nonce: Some("n".into()),
            }),
        ),
    ];

    assert_eq!(samples.len(), MATRIX_IPC_KINDS.len());
    let mut seen = std::collections::BTreeSet::new();
    for env in samples {
        let kind = env.kind().to_string();
        seen.insert(kind.clone());
        let json = serde_json::to_string(&env).expect("serialize");
        let back = MatrixIpcEnvelope::from_json_str(&json).expect("deserialize");
        assert_eq!(back.kind(), env.kind());
        assert_eq!(back.protocol_version, env.protocol_version);
        assert_eq!(back.session_generation, env.session_generation);
        assert_eq!(back.sequence, env.sequence);
        assert_eq!(back, env, "round-trip equality for kind {kind}");
    }
    for k in MATRIX_IPC_KINDS {
        assert!(seen.contains(*k), "missing round-trip sample for kind {k}");
    }
}

#[test]
fn fixture_valid_remaining_control_kinds() {
    for (name, kind) in [
        ("valid_unsubscribe.json", KIND_UNSUBSCRIBE),
        ("valid_subscribed.json", KIND_SUBSCRIBED),
        ("valid_unsubscribed.json", KIND_UNSUBSCRIBED),
        ("valid_cancel.json", KIND_CANCEL),
        ("valid_ping.json", KIND_PING),
        ("valid_pong.json", KIND_PONG),
    ] {
        let env = MatrixIpcEnvelope::from_json_str(&fixture(name)).unwrap_or_else(|e| {
            panic!("{name} must parse: {e}");
        });
        assert_eq!(env.kind(), kind, "{name}");
        // Round-trip fixture bytes → value → JSON → value
        let v = env.to_json_value().unwrap();
        let again = MatrixIpcEnvelope::from_json_value(v).unwrap();
        assert_eq!(again.kind(), kind);
    }
}

#[test]
fn fixture_invalid_payloads_rejected() {
    for name in [
        "invalid_missing_kind.json",
        "invalid_missing_sequence.json",
        "invalid_wrong_type_protocol_version.json",
        "invalid_unknown_topic.json",
        "invalid_unknown_error_category.json",
        "invalid_error_with_secret_field.json",
        "invalid_hello_missing_client_protocol_version.json",
        "invalid_unknown_kind.json",
        "invalid_missing_protocol_version.json",
    ] {
        let raw = fixture(name);
        let result = MatrixIpcEnvelope::from_json_str(&raw);
        assert!(
            result.is_err(),
            "{name} must be rejected at boundary, got {:?}",
            result.map(|e| e.kind().to_string())
        );
    }
}

#[test]
fn bounds_payload_queue_streams() {
    assert!(check_payload_json_bounds(0).is_ok());
    assert!(check_payload_json_bounds(MAX_ENVELOPE_PAYLOAD_JSON_BYTES).is_ok());
    let over = check_payload_json_bounds(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 1).unwrap_err();
    assert_eq!(over.category, MatrixIpcErrorCategory::SdkInvariant);
    assert!(over
        .diagnostic_id
        .as_deref()
        .unwrap_or("")
        .contains("payload_too_large"));

    assert!(check_stream_queue_depth(0).is_ok());
    assert!(check_stream_queue_depth(MAX_STREAM_QUEUE_DEPTH).is_ok());
    let q = check_stream_queue_depth(MAX_STREAM_QUEUE_DEPTH + 1).unwrap_err();
    assert_eq!(q.category, MatrixIpcErrorCategory::SdkInvariant);

    assert!(check_open_streams(0).is_ok());
    assert!(check_open_streams(MAX_OPEN_STREAMS_PER_SESSION).is_ok());
    let s = check_open_streams(MAX_OPEN_STREAMS_PER_SESSION + 1).unwrap_err();
    assert_eq!(s.category, MatrixIpcErrorCategory::SdkInvariant);
}

#[test]
fn sequence_gap_and_stale_generation_compose_resync() {
    // Gap → resync payload reason
    let gap = resync_payload_for_gap("stream-t", 10, 14);
    assert_eq!(gap.reason, ResyncReason::SequenceGap);
    assert_eq!(gap.last_applied_sequence, Some(10));
    assert_eq!(gap.observed_sequence, Some(14));

    let env = MatrixIpcEnvelope::new(7, 0, MatrixIpcMessage::ResyncRequired(gap))
        .with_stream_id("stream-t");
    let back = MatrixIpcEnvelope::from_json_str(&serde_json::to_string(&env).unwrap()).unwrap();
    match back.message {
        MatrixIpcMessage::ResyncRequired(p) => {
            assert_eq!(p.reason, ResyncReason::SequenceGap);
        }
        other => panic!("expected resync_required, got {:?}", other.kind()),
    }

    // Stale generation → error category + resync reason
    let stale = check_session_generation(3, 1).unwrap_err();
    assert_eq!(
        stale.category,
        MatrixIpcErrorCategory::StaleSessionGeneration
    );
    let resync = resync_payload_for_stale_generation(Some("stream-t".into()));
    assert_eq!(resync.reason, ResyncReason::StaleSessionGeneration);

    // Behind sequence also forces resync lifecycle event
    let (outcome, event) = apply_delta_sequence(Some(9), 4);
    assert!(matches!(outcome, SequenceOutcome::Behind { .. }));
    assert_eq!(event, Some(StreamLifecycleEvent::ResyncNeeded));
}

#[test]
fn schema_catalog_compatible_with_rust_constants() {
    let catalog = schema_catalog();
    assert_eq!(
        catalog["protocolVersion"].as_u64().unwrap() as u32,
        MATRIX_IPC_PROTOCOL_VERSION
    );

    let kinds = catalog["kinds"]
        .as_array()
        .expect("kinds array")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert_eq!(kinds.len(), MATRIX_IPC_KINDS.len());
    for (i, k) in MATRIX_IPC_KINDS.iter().enumerate() {
        assert_eq!(&kinds[i], k, "kind order must match catalog");
    }

    let categories = catalog["errorCategories"]
        .as_array()
        .expect("errorCategories")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert_eq!(categories.len(), MatrixIpcErrorCategory::ALL.len());
    for (i, cat) in MatrixIpcErrorCategory::ALL.iter().enumerate() {
        assert_eq!(categories[i], cat.as_str());
    }

    let topics = catalog["streamTopics"]
        .as_array()
        .expect("streamTopics")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert_eq!(topics.len(), StreamTopic::ALL.len());
    for (i, t) in StreamTopic::ALL.iter().enumerate() {
        assert_eq!(topics[i], t.as_str());
    }

    let bounds = &catalog["bounds"];
    assert_eq!(
        bounds["maxEnvelopePayloadJsonBytes"].as_u64().unwrap() as usize,
        MAX_ENVELOPE_PAYLOAD_JSON_BYTES
    );
    assert_eq!(
        bounds["maxStreamQueueDepth"].as_u64().unwrap() as usize,
        MAX_STREAM_QUEUE_DEPTH
    );
    assert_eq!(
        bounds["streamCoalesceWindowMs"].as_u64().unwrap(),
        STREAM_COALESCE_WINDOW_MS
    );
    assert_eq!(
        bounds["maxOpenStreamsPerSession"].as_u64().unwrap() as usize,
        MAX_OPEN_STREAMS_PER_SESSION
    );
    assert_eq!(
        bounds["forbidMediaBytesOverJsonIpc"].as_bool().unwrap(),
        FORBID_MEDIA_BYTES_OVER_JSON_IPC
    );
    assert_eq!(bounds["maxWireCounter"].as_u64().unwrap(), MAX_WIRE_COUNTER);
}

#[test]
fn unknown_resync_and_cancel_reasons_rejected() {
    let bad_resync = json!({
        "protocolVersion": 1,
        "sessionGeneration": 1,
        "sequence": 0,
        "kind": "resync_required",
        "payload": { "reason": "not_a_reason" }
    });
    assert!(MatrixIpcEnvelope::from_json_value(bad_resync).is_err());

    let bad_cancel = json!({
        "protocolVersion": 1,
        "sessionGeneration": 1,
        "sequence": 0,
        "kind": "cancel",
        "payload": {
            "cancellationToken": "t",
            "reason": "not_a_cancel_reason"
        }
    });
    assert!(MatrixIpcEnvelope::from_json_value(bad_cancel).is_err());
}

#[test]
fn error_categories_exhaustive_count_matches_catalog() {
    assert_eq!(MatrixIpcErrorCategory::ALL.len(), 21);
    for cat in MatrixIpcErrorCategory::ALL {
        let v = serde_json::to_value(cat).unwrap();
        let back: MatrixIpcErrorCategory = serde_json::from_value(v).unwrap();
        assert_eq!(back, *cat);
        // Unknown wire string rejected
    }
    assert!(serde_json::from_str::<MatrixIpcErrorCategory>("\"not_real\"").is_err());
}

#[test]
fn protocol_version_zero_and_future_rejected() {
    assert!(check_protocol_version(0).is_err());
    assert!(check_protocol_version(2).is_err());
    assert!(check_protocol_version(MATRIX_IPC_PROTOCOL_VERSION).is_ok());
}

#[test]
fn stale_generation_higher_and_lower_both_rejected() {
    // Any mismatch is stale — including "future" generations from another session.
    assert!(check_session_generation(5, 6).is_err());
    assert!(check_session_generation(5, 4).is_err());
    assert!(check_session_generation(5, 5).is_ok());
}
