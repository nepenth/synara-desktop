//! P1.5 — Expanded IPC protocol contract tests.
//!
//! Covers: serialization round trips (envelope + kinds + error categories),
//! unknown variants / unknown kind rejection, invalid payloads, policy bounds,
//! sequence gaps → resync_required, stale session generation rejection, and
//! shared JSON fixture compatibility (Rust loads docs/matrix-rust-sdk/ipc/fixtures).
//!
//! No matrix_sdk / Ruma types. No production login/sync/Client session.

use super::*;
use serde_json::{json, Value};

/// Fixture root relative to `src-tauri` package (`CARGO_MANIFEST_DIR`).
fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/matrix-rust-sdk/ipc/fixtures")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("failed to read fixture {}: {e}", path.display());
    })
}

fn parse_ok(name: &str) -> MatrixIpcEnvelope {
    MatrixIpcEnvelope::from_json_str(&fixture(name)).unwrap_or_else(|e| {
        panic!("fixture {name} must parse: {e}");
    })
}

fn parse_err(name: &str) {
    let result = MatrixIpcEnvelope::from_json_str(&fixture(name));
    assert!(
        result.is_err(),
        "fixture {name} must be rejected, got: {:?}",
        result.ok().map(|e| e.kind().to_string())
    );
}

// ---------------------------------------------------------------------------
// Policy constants / bounds
// ---------------------------------------------------------------------------

#[test]
fn policy_constants_exact_values() {
    assert_eq!(MATRIX_IPC_PROTOCOL_VERSION, 1);
    assert_eq!(MAX_ENVELOPE_PAYLOAD_JSON_BYTES, 1_048_576);
    assert_eq!(MAX_STREAM_QUEUE_DEPTH, 256);
    assert_eq!(STREAM_COALESCE_WINDOW_MS, 16);
    assert_eq!(MAX_OPEN_STREAMS_PER_SESSION, 64);
    assert_eq!(MAX_WIRE_COUNTER, 9_007_199_254_740_991);
    const { assert!(FORBID_MEDIA_BYTES_OVER_JSON_IPC) };
}

#[test]
fn r0_3_wire_counter_and_stream_id_authority_fixtures() {
    parse_err("invalid_sequence_above_wire_max.json");
    parse_err("invalid_stream_id_mismatch.json");
    // Boundary value MAX_WIRE_COUNTER is accepted.
    let max_ok = json!({
        "protocolVersion": 1,
        "sessionGeneration": MAX_WIRE_COUNTER,
        "sequence": MAX_WIRE_COUNTER,
        "kind": "ping",
        "payload": {}
    });
    MatrixIpcEnvelope::from_json_value(max_ok).expect("max wire counter must parse");
    // Checked next at max is None → sequence gap, not wrap.
    assert!(matches!(
        check_sequence(Some(MAX_WIRE_COUNTER), MAX_WIRE_COUNTER + 1),
        SequenceOutcome::Gap { .. }
    ));
    assert!(matches!(
        check_sequence(Some(MAX_WIRE_COUNTER - 1), MAX_WIRE_COUNTER),
        SequenceOutcome::Accept { .. }
    ));
}

#[test]
fn payload_size_bounds_policy() {
    assert!(payload_within_bounds(0));
    assert!(payload_within_bounds(MAX_ENVELOPE_PAYLOAD_JSON_BYTES));
    assert!(!payload_within_bounds(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 1));
    // Oversized synthetic body must fail the policy helper (no silent accept).
    let huge_len = MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 1024;
    assert!(!payload_within_bounds(huge_len));
}

#[test]
fn stream_queue_and_open_stream_bounds_policy() {
    assert!(stream_queue_depth_within_bounds(0));
    assert!(stream_queue_depth_within_bounds(MAX_STREAM_QUEUE_DEPTH));
    assert!(!stream_queue_depth_within_bounds(
        MAX_STREAM_QUEUE_DEPTH + 1
    ));
    assert!(open_streams_within_bounds(0));
    assert!(open_streams_within_bounds(MAX_OPEN_STREAMS_PER_SESSION));
    assert!(!open_streams_within_bounds(
        MAX_OPEN_STREAMS_PER_SESSION + 1
    ));
}

#[test]
fn typical_envelope_payload_within_bounds() {
    let env = parse_ok("valid_snapshot.json");
    let bytes = serde_json::to_vec(&env).expect("serialize");
    assert!(
        payload_within_bounds(bytes.len()),
        "canonical fixture envelopes must fit under MAX_ENVELOPE_PAYLOAD_JSON_BYTES"
    );
}

// ---------------------------------------------------------------------------
// Exhaustive kind / error-category / topic / reason wire names
// ---------------------------------------------------------------------------

#[test]
fn all_kinds_round_trip_as_envelopes() {
    let samples: Vec<MatrixIpcMessage> = vec![
        MatrixIpcMessage::Hello(HelloPayload {
            client_protocol_version: 1,
            client_name: Some("synara-web".into()),
        }),
        MatrixIpcMessage::HelloAck(HelloAckPayload {
            protocol_version: 1,
            session_generation: 1,
        }),
        MatrixIpcMessage::Subscribe(SubscribePayload {
            topic: StreamTopic::RoomList,
            stream_id: "s1".into(),
            params: Some(json!({})),
        }),
        MatrixIpcMessage::Unsubscribe(UnsubscribePayload {
            stream_id: "s1".into(),
        }),
        MatrixIpcMessage::Subscribed(SubscribedPayload {
            stream_id: "s1".into(),
            topic: StreamTopic::RoomList,
        }),
        MatrixIpcMessage::Unsubscribed(UnsubscribedPayload {
            stream_id: "s1".into(),
            resources_released: true,
        }),
        MatrixIpcMessage::Snapshot(SnapshotPayload {
            stream_id: "s1".into(),
            topic: StreamTopic::Timeline,
            snapshot_id: "snap-1".into(),
            body: json!({"items": []}),
        }),
        MatrixIpcMessage::Delta(DeltaPayload {
            stream_id: "s1".into(),
            topic: StreamTopic::Timeline,
            idempotency_key: Some("idem-1".into()),
            body: json!({"items": []}),
        }),
        MatrixIpcMessage::ResyncRequired(ResyncRequiredPayload {
            stream_id: Some("s1".into()),
            reason: ResyncReason::SequenceGap,
            last_applied_sequence: Some(2),
            observed_sequence: Some(5),
        }),
        MatrixIpcMessage::Cancel(CancelPayload {
            cancellation_token: "tok".into(),
            reason: Some(CancelReason::Timeout),
        }),
        MatrixIpcMessage::Error(
            MatrixIpcError::new(MatrixIpcErrorCategory::Connectivity).with_diagnostic("diag-c"),
        ),
        MatrixIpcMessage::Ping(PingPayload {
            nonce: Some("n".into()),
        }),
        MatrixIpcMessage::Pong(PongPayload {
            nonce: Some("n".into()),
        }),
    ];

    assert_eq!(samples.len(), MATRIX_IPC_KINDS.len());
    for (i, msg) in samples.into_iter().enumerate() {
        let expected_kind = MATRIX_IPC_KINDS[i];
        assert_eq!(msg.kind(), expected_kind);
        // R0.3: stream-scoped kinds require matching envelope.streamId.
        let mut env = MatrixIpcEnvelope::new(1, i as u64, msg).with_request_id(format!("req-{i}"));
        if matches!(
            expected_kind,
            KIND_SUBSCRIBE
                | KIND_UNSUBSCRIBE
                | KIND_SUBSCRIBED
                | KIND_UNSUBSCRIBED
                | KIND_SNAPSHOT
                | KIND_DELTA
                | KIND_RESYNC_REQUIRED
        ) {
            env = env.with_stream_id("s1");
        }
        let json = serde_json::to_string(&env).expect("serialize");
        let back = MatrixIpcEnvelope::from_json_str(&json).expect("deserialize");
        assert_eq!(back.kind(), expected_kind);
        assert_eq!(back.protocol_version, MATRIX_IPC_PROTOCOL_VERSION);
        assert_eq!(back.session_generation, 1);
        assert_eq!(back.sequence, i as u64);
        // Wire kind is snake_case string
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["kind"].as_str().unwrap(), expected_kind);
        assert!(v.get("payload").is_some());
        assert!(v.get("protocolVersion").is_some());
        assert!(v.get("sessionGeneration").is_some());
    }
}

#[test]
fn all_error_categories_wire_names_stable() {
    assert_eq!(MatrixIpcErrorCategory::ALL.len(), 21);
    for category in MatrixIpcErrorCategory::ALL {
        let err = MatrixIpcError::new(*category);
        let v = serde_json::to_value(&err).unwrap();
        assert_eq!(v["category"].as_str().unwrap(), category.as_str());
        let back: MatrixIpcError = serde_json::from_value(v).unwrap();
        assert_eq!(back.category, *category);
        // Privacy: serialized form has no secret field names
        let raw = serde_json::to_string(&err).unwrap();
        for forbidden in [
            "access_token",
            "accessToken",
            "refresh_token",
            "password",
            "recovery_key",
            "recoveryKey",
            "plaintext",
            "mediaBytes",
        ] {
            assert!(
                !raw.contains(forbidden),
                "error category {:?} must not embed forbidden key {forbidden}",
                category
            );
        }
    }
}

#[test]
fn resync_and_cancel_reasons_round_trip() {
    for reason in [
        ResyncReason::SequenceGap,
        ResyncReason::StaleSessionGeneration,
        ResyncReason::UnknownKind,
        ResyncReason::SnapshotRequired,
        ResyncReason::SupervisorReset,
    ] {
        let p = ResyncRequiredPayload {
            stream_id: Some("s".into()),
            reason,
            last_applied_sequence: None,
            observed_sequence: None,
        };
        let v = serde_json::to_value(&p).unwrap();
        let back: ResyncRequiredPayload = serde_json::from_value(v).unwrap();
        assert_eq!(back.reason, reason);
    }
    for reason in [
        CancelReason::ClientRequest,
        CancelReason::Timeout,
        CancelReason::SessionEnded,
        CancelReason::StreamClosed,
        CancelReason::Superseded,
    ] {
        let p = CancelPayload {
            cancellation_token: "t".into(),
            reason: Some(reason),
        };
        let v = serde_json::to_value(&p).unwrap();
        let back: CancelPayload = serde_json::from_value(v).unwrap();
        assert_eq!(back.reason, Some(reason));
    }
}

// ---------------------------------------------------------------------------
// Shared fixtures — valid
// ---------------------------------------------------------------------------

const VALID_FIXTURES: &[&str] = &[
    "valid_hello.json",
    "valid_hello_ack.json",
    "valid_subscribe.json",
    "valid_unsubscribe.json",
    "valid_subscribed.json",
    "valid_unsubscribed.json",
    "valid_snapshot.json",
    "valid_delta.json",
    "valid_resync_required.json",
    "valid_resync_stale_generation.json",
    "valid_cancel.json",
    "valid_error_rate_limited.json",
    "valid_error_stale_session.json",
    "valid_ping.json",
    "valid_pong.json",
    "valid_snapshot_with_room_summary_body.json",
];

#[test]
fn all_valid_fixtures_parse() {
    for name in VALID_FIXTURES {
        let env = parse_ok(name);
        assert_eq!(env.protocol_version, MATRIX_IPC_PROTOCOL_VERSION);
        assert!(
            MATRIX_IPC_KINDS.contains(&env.kind()),
            "{name}: kind {} not in MATRIX_IPC_KINDS",
            env.kind()
        );
        // Round-trip: fixture → typed → JSON → typed
        let json = serde_json::to_string(&env).expect("serialize");
        let back = MatrixIpcEnvelope::from_json_str(&json).expect("re-parse");
        assert_eq!(back.kind(), env.kind());
        assert_eq!(back.sequence, env.sequence);
        assert_eq!(back.session_generation, env.session_generation);
    }
}

#[test]
fn fixture_valid_lifecycle_control_kinds() {
    assert_eq!(parse_ok("valid_unsubscribe.json").kind(), KIND_UNSUBSCRIBE);
    assert_eq!(parse_ok("valid_subscribed.json").kind(), KIND_SUBSCRIBED);
    assert_eq!(
        parse_ok("valid_unsubscribed.json").kind(),
        KIND_UNSUBSCRIBED
    );
    assert_eq!(parse_ok("valid_cancel.json").kind(), KIND_CANCEL);
    assert_eq!(parse_ok("valid_ping.json").kind(), KIND_PING);
    assert_eq!(parse_ok("valid_pong.json").kind(), KIND_PONG);
}

#[test]
fn fixture_snapshot_with_dto_shaped_body() {
    let env = parse_ok("valid_snapshot_with_room_summary_body.json");
    assert_eq!(env.kind(), KIND_SNAPSHOT);
    match env.message {
        MatrixIpcMessage::Snapshot(snap) => {
            assert_eq!(snap.topic, StreamTopic::RoomList);
            let rooms = snap
                .body
                .get("rooms")
                .and_then(|v| v.as_array())
                .expect("rooms array");
            assert_eq!(rooms.len(), 1);
            assert_eq!(
                rooms[0].get("roomId").and_then(|v| v.as_str()),
                Some("!room:example.org")
            );
            // DTO composition: body room summary parses as domain DTO
            let summary: crate::dto::RoomSummary =
                serde_json::from_value(rooms[0].clone()).expect("RoomSummary DTO");
            assert_eq!(summary.room_id, "!room:example.org");
            assert_eq!(summary.membership, crate::dto::Membership::Join);
        }
        other => panic!("expected snapshot, got {}", other.kind()),
    }
}

// ---------------------------------------------------------------------------
// Shared fixtures — invalid / unknown variants
// ---------------------------------------------------------------------------

const INVALID_FIXTURES: &[&str] = &[
    "invalid_unknown_kind.json",
    "invalid_missing_protocol_version.json",
    "invalid_missing_session_generation.json",
    "invalid_missing_sequence.json",
    "invalid_missing_kind.json",
    "invalid_missing_payload.json",
    "invalid_wrong_type_protocol_version.json",
    "invalid_wrong_type_sequence.json",
    "invalid_unknown_error_category.json",
    "invalid_unknown_topic.json",
    "invalid_subscribe_missing_stream_id.json",
    "invalid_hello_missing_client_protocol_version.json",
    "invalid_unknown_resync_reason.json",
    "invalid_error_with_secret_field.json",
    "invalid_sequence_above_wire_max.json",
    "invalid_stream_id_mismatch.json",
    "invalid_snapshot_body_secret_field.json",
    "invalid_snapshot_body_wrong_topic_shape.json",
    "invalid_delta_body_media_bytes.json",
];

#[test]
fn all_invalid_fixtures_rejected() {
    for name in INVALID_FIXTURES {
        parse_err(name);
    }
}

#[test]
fn unknown_kind_rejected_at_boundary() {
    parse_err("invalid_unknown_kind.json");
    let bad = json!({
        "protocolVersion": 1,
        "sessionGeneration": 1,
        "sequence": 0,
        "kind": "login",
        "payload": {}
    });
    assert!(MatrixIpcEnvelope::from_json_value(bad).is_err());
}

#[test]
fn unknown_error_category_rejected() {
    parse_err("invalid_unknown_error_category.json");
    let err_only = json!({ "category": "sdk_error_string_dump" });
    assert!(serde_json::from_value::<MatrixIpcError>(err_only).is_err());
}

#[test]
fn unknown_topic_and_resync_reason_rejected() {
    parse_err("invalid_unknown_topic.json");
    parse_err("invalid_unknown_resync_reason.json");
    assert!(serde_json::from_value::<StreamTopic>(json!("rooms")).is_err());
    assert!(serde_json::from_value::<ResyncReason>(json!("gap")).is_err());
}

// ---------------------------------------------------------------------------
// Invalid payloads — missing required fields / wrong types (inline)
// ---------------------------------------------------------------------------

#[test]
fn invalid_missing_required_envelope_fields() {
    // Missing sessionGeneration
    assert!(MatrixIpcEnvelope::from_json_value(json!({
        "protocolVersion": 1,
        "sequence": 0,
        "kind": "ping",
        "payload": {}
    }))
    .is_err());
    // Missing sequence
    assert!(MatrixIpcEnvelope::from_json_value(json!({
        "protocolVersion": 1,
        "sessionGeneration": 1,
        "kind": "ping",
        "payload": {}
    }))
    .is_err());
    // Missing kind
    assert!(MatrixIpcEnvelope::from_json_value(json!({
        "protocolVersion": 1,
        "sessionGeneration": 1,
        "sequence": 0,
        "payload": {}
    }))
    .is_err());
    // Missing payload
    assert!(MatrixIpcEnvelope::from_json_value(json!({
        "protocolVersion": 1,
        "sessionGeneration": 1,
        "sequence": 0,
        "kind": "ping"
    }))
    .is_err());
}

#[test]
fn invalid_wrong_types_rejected() {
    // protocolVersion as string
    assert!(MatrixIpcEnvelope::from_json_value(json!({
        "protocolVersion": "1",
        "sessionGeneration": 1,
        "sequence": 0,
        "kind": "ping",
        "payload": {}
    }))
    .is_err());
    // sequence as string
    assert!(MatrixIpcEnvelope::from_json_value(json!({
        "protocolVersion": 1,
        "sessionGeneration": 1,
        "sequence": "0",
        "kind": "ping",
        "payload": {}
    }))
    .is_err());
    // sessionGeneration as bool
    assert!(MatrixIpcEnvelope::from_json_value(json!({
        "protocolVersion": 1,
        "sessionGeneration": true,
        "sequence": 0,
        "kind": "ping",
        "payload": {}
    }))
    .is_err());
    // payload as string (must be object for adjacent-tagged kinds)
    assert!(MatrixIpcEnvelope::from_json_value(json!({
        "protocolVersion": 1,
        "sessionGeneration": 1,
        "sequence": 0,
        "kind": "ping",
        "payload": "not-an-object"
    }))
    .is_err());
    // payload as number
    assert!(MatrixIpcEnvelope::from_json_value(json!({
        "protocolVersion": 1,
        "sessionGeneration": 1,
        "sequence": 0,
        "kind": "hello",
        "payload": 42
    }))
    .is_err());
    // payload as array (explicit boundary guard; serde empty-struct quirk)
    assert!(MatrixIpcEnvelope::from_json_value(json!({
        "protocolVersion": 1,
        "sessionGeneration": 1,
        "sequence": 0,
        "kind": "ping",
        "payload": []
    }))
    .is_err());
}

#[test]
fn invalid_payload_missing_kind_required_fields() {
    // hello without clientProtocolVersion
    parse_err("invalid_hello_missing_client_protocol_version.json");
    // subscribe without streamId
    parse_err("invalid_subscribe_missing_stream_id.json");
    // snapshot missing snapshotId
    assert!(MatrixIpcEnvelope::from_json_value(json!({
        "protocolVersion": 1,
        "sessionGeneration": 1,
        "sequence": 1,
        "kind": "snapshot",
        "payload": {
            "streamId": "s1",
            "topic": "room_list",
            "body": {}
        }
    }))
    .is_err());
    // cancel without cancellationToken
    assert!(MatrixIpcEnvelope::from_json_value(json!({
        "protocolVersion": 1,
        "sessionGeneration": 1,
        "sequence": 0,
        "kind": "cancel",
        "payload": {
            "reason": "timeout"
        }
    }))
    .is_err());
}

// ---------------------------------------------------------------------------
// Sequence gaps → resync_required path
// ---------------------------------------------------------------------------

#[test]
fn sequence_gap_produces_resync_required_envelope() {
    let last = Some(2u64);
    let observed = 5u64;
    let (outcome, event) = apply_delta_sequence(last, observed);
    assert!(matches!(
        outcome,
        SequenceOutcome::Gap {
            last_applied: 2,
            observed: 5
        }
    ));
    assert_eq!(event, Some(StreamLifecycleEvent::ResyncNeeded));

    let payload = resync_payload_for_gap("stream-room-list-1", 2, 5);
    assert_eq!(payload.reason, ResyncReason::SequenceGap);
    assert_eq!(payload.last_applied_sequence, Some(2));
    assert_eq!(payload.observed_sequence, Some(5));

    let env = MatrixIpcEnvelope::new(1, 0, MatrixIpcMessage::ResyncRequired(payload))
        .with_stream_id("stream-room-list-1");
    let json = serde_json::to_string(&env).unwrap();
    let back = MatrixIpcEnvelope::from_json_str(&json).unwrap();
    assert_eq!(back.kind(), KIND_RESYNC_REQUIRED);
    match back.message {
        MatrixIpcMessage::ResyncRequired(p) => {
            assert_eq!(p.reason, ResyncReason::SequenceGap);
        }
        other => panic!("expected resync_required, got {}", other.kind()),
    }

    // Fixture parity with gap path
    let fixture_env = parse_ok("valid_resync_required.json");
    match fixture_env.message {
        MatrixIpcMessage::ResyncRequired(p) => {
            assert_eq!(p.reason, ResyncReason::SequenceGap);
            assert_eq!(p.last_applied_sequence, Some(2));
            assert_eq!(p.observed_sequence, Some(5));
        }
        other => panic!("fixture mismatch: {}", other.kind()),
    }
}

#[test]
fn behind_sequence_also_forces_resync_event() {
    let (outcome, event) = apply_delta_sequence(Some(10), 3);
    assert!(matches!(outcome, SequenceOutcome::Behind { .. }));
    assert_eq!(event, Some(StreamLifecycleEvent::ResyncNeeded));
}

#[test]
fn gap_drives_lifecycle_to_resync_required() {
    use StreamLifecycleEvent as E;
    use StreamLifecycleState as S;

    let mut state = S::Live;
    let (_outcome, event) = apply_delta_sequence(Some(1), 9);
    assert_eq!(event, Some(E::ResyncNeeded));
    state = transition_stream_lifecycle(state, event.unwrap()).unwrap();
    assert_eq!(state, S::ResyncRequired);
    // Client must resubscribe for a fresh snapshot
    state = transition_stream_lifecycle(state, E::SubscribeRequested).unwrap();
    assert_eq!(state, S::Subscribing);
}

// ---------------------------------------------------------------------------
// Stale session generation rejection
// ---------------------------------------------------------------------------

#[test]
fn stale_session_generation_rejected_and_resync_payload() {
    // Matching generation OK
    check_session_generation(7, 7).unwrap();

    // Stale (lower) rejected
    let err = check_session_generation(7, 3).unwrap_err();
    assert_eq!(err.category, MatrixIpcErrorCategory::StaleSessionGeneration);
    assert!(err.diagnostic_id.is_some());

    // Future/higher generation also rejected (not equal → stale from peer view)
    let err2 = check_session_generation(7, 99).unwrap_err();
    assert_eq!(
        err2.category,
        MatrixIpcErrorCategory::StaleSessionGeneration
    );

    let payload = resync_payload_for_stale_generation(Some("stream-timeline-1".into()));
    assert_eq!(payload.reason, ResyncReason::StaleSessionGeneration);
    assert_eq!(payload.stream_id.as_deref(), Some("stream-timeline-1"));

    let env = MatrixIpcEnvelope::new(2, 0, MatrixIpcMessage::ResyncRequired(payload))
        .with_stream_id("stream-timeline-1");
    assert_eq!(env.kind(), KIND_RESYNC_REQUIRED);

    // Fixture parity
    let fixture_env = parse_ok("valid_resync_stale_generation.json");
    match fixture_env.message {
        MatrixIpcMessage::ResyncRequired(p) => {
            assert_eq!(p.reason, ResyncReason::StaleSessionGeneration);
        }
        other => panic!("expected resync_required, got {}", other.kind()),
    }

    let err_env = parse_ok("valid_error_stale_session.json");
    match err_env.message {
        MatrixIpcMessage::Error(e) => {
            assert_eq!(e.category, MatrixIpcErrorCategory::StaleSessionGeneration);
        }
        other => panic!("expected error, got {}", other.kind()),
    }
}

#[test]
fn envelope_stale_generation_check_on_parsed_fixture() {
    let live = 5u64;
    let env = parse_ok("valid_delta.json"); // sessionGeneration: 1
    assert!(check_session_generation(live, env.session_generation).is_err());
    assert!(check_session_generation(env.session_generation, env.session_generation).is_ok());
}

// ---------------------------------------------------------------------------
// Protocol version negotiation contract
// ---------------------------------------------------------------------------

#[test]
fn unsupported_protocol_version_rejected() {
    check_protocol_version(1).unwrap();
    let err = check_protocol_version(0).unwrap_err();
    assert_eq!(err.category, MatrixIpcErrorCategory::UnsupportedCapability);
    let err2 = check_protocol_version(2).unwrap_err();
    assert_eq!(err2.category, MatrixIpcErrorCategory::UnsupportedCapability);
}

// ---------------------------------------------------------------------------
// Schema compatibility index (fixture inventory)
// ---------------------------------------------------------------------------

#[test]
fn fixture_inventory_matches_contract_lists() {
    // Ensure every named fixture file exists and is valid JSON.
    for name in VALID_FIXTURES.iter().chain(INVALID_FIXTURES.iter()) {
        let raw = fixture(name);
        let v: Value =
            serde_json::from_str(&raw).unwrap_or_else(|e| panic!("{name}: invalid JSON: {e}"));
        assert!(v.is_object(), "{name} must be a JSON object");
    }
}

// ---------------------------------------------------------------------------
// No SDK types on IPC wire modules (compile-time + marker)
// ---------------------------------------------------------------------------

#[test]
fn ipc_module_marker_has_no_sdk_surface() {
    // SNC-P1-3: `matrix_ipc_schema_markers` is composed in the src-tauri shell
    // from these transport constants via the `crate::matrix::ipc` re-export;
    // from synara-core we assert the transport module's own protocol marker
    // identity instead — the wire modules stay free of matrix_sdk types either
    // way (compile-time + marker).
    assert_eq!(MATRIX_IPC_PROTOCOL_VERSION, 1);
    assert!(!MATRIX_IPC_KINDS.is_empty());
    assert!(!MatrixIpcErrorCategory::ALL.is_empty());
    assert!(!StreamTopic::ALL.is_empty());
    const { assert!(FORBID_MEDIA_BYTES_OVER_JSON_IPC) };
    const { assert!(MAX_STREAM_QUEUE_DEPTH > 0) };
}
