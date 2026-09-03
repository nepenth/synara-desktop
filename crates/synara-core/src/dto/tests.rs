//! Fixture + serde round-trip tests for Synara domain DTOs (P1.4).

use super::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

/// Fixture root relative to this package (`CARGO_MANIFEST_DIR`). The fixtures
/// live at the repository root under `docs/matrix-rust-sdk/dto/fixtures/`.
fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/matrix-rust-sdk/dto/fixtures")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("failed to read fixture {}: {e}", path.display());
    })
}

fn assert_no_forbidden_fields(raw: &str) {
    for name in FORBIDDEN_WIRE_FIELD_NAMES {
        assert!(
            !raw.contains(name),
            "forbidden wire field name `{name}` found in JSON: {raw}"
        );
    }
}

fn round_trip_json<T>(value: &T) -> T
where
    T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let json = serde_json::to_string(value).expect("serialize");
    assert_no_forbidden_fields(&json);
    let back: T = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(&back, value);
    // Also via Value
    let v = serde_json::to_value(value).expect("to_value");
    let back2: T = serde_json::from_value(v).expect("from_value");
    assert_eq!(&back2, value);
    back
}

fn load_fixture_as<T: DeserializeOwned>(name: &str) -> T {
    let raw = fixture(name);
    assert_no_forbidden_fields(&raw);
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("fixture {name} parse failed: {e}"))
}

#[test]
fn marker_and_policy_constants() {
    assert_eq!(MATRIX_DTO_MARKER, "matrix-domain-dtos-p1.4");
    const { assert!(FORBID_MEDIA_BYTES_OVER_JSON_IPC) };
    assert!(!FORBIDDEN_WIRE_FIELD_NAMES.is_empty());
}

#[test]
fn session_lifecycle_wire_names() {
    for life in SessionLifecycle::ALL {
        let v = serde_json::to_value(life).unwrap();
        assert_eq!(v.as_str().unwrap(), life.as_str());
    }
}

#[test]
fn session_snapshot_round_trip_and_fixture() {
    let s = SessionSnapshot {
        session_generation: 3,
        user_id: "@alice:example.org".into(),
        device_id: "DEVICEABC".into(),
        homeserver_url: "https://matrix.example.org".into(),
        display_name: Some("Alice".into()),
        avatar_url: Some("mxc://example.org/avatar1".into()),
        lifecycle: SessionLifecycle::Ready,
        crypto_ready: true,
    };
    round_trip_json(&s);

    let from_fix: SessionSnapshot = load_fixture_as("valid_session.json");
    assert_eq!(from_fix.user_id, "@alice:example.org");
    assert_eq!(from_fix.lifecycle, SessionLifecycle::Ready);
    assert!(from_fix.crypto_ready);
    // Explicit: no token keys in fixture JSON object.
    let v: Value = serde_json::from_str(&fixture("valid_session.json")).unwrap();
    let obj = v.as_object().unwrap();
    assert!(!obj.contains_key("accessToken"));
    assert!(!obj.contains_key("access_token"));
    assert!(!obj.contains_key("refreshToken"));
    assert!(!obj.contains_key("refresh_token"));
}

#[test]
fn room_summary_round_trip_and_fixture() {
    let r = RoomSummary {
        room_id: "!room:example.org".into(),
        name: Some("General".into()),
        canonical_alias: Some("#general:example.org".into()),
        avatar_url: None,
        membership: Membership::Join,
        is_direct: false,
        is_space: false,
        is_call: false,
        is_favorite: false,
        is_low_priority: false,
        folder_id: None,
        encryption_status: RoomEncryptionStatus::Encrypted,
        join_rule: Some("invite".into()),
        unread_count: 2,
        highlight_count: 1,
        marked_unread: false,
        notification_mode: Some(NotificationMode::Mentions),
        last_activity_ts: Some(1_720_000_000_000),
        last_message_preview: Some("Hello from Alice".into()),
        heroes: Some(vec![RoomHero {
            user_id: "@bob:example.org".into(),
            display_name: Some("Bob".into()),
        }]),
        tombstone_successor_room_id: None,
    };
    round_trip_json(&r);
    let from_fix: RoomSummary = load_fixture_as("valid_room_summary.json");
    assert_eq!(from_fix.room_id, "!room:example.org");
    assert_eq!(from_fix.membership, Membership::Join);
}

#[test]
fn room_summary_rejects_missing_or_contradictory_encryption_authority() {
    let missing = r#"{ "roomId": "!room:example.org", "membership": "join", "isDirect": false, "isEncrypted": false, "unreadCount": 0, "highlightCount": 0, "markedUnread": false }"#;
    assert!(serde_json::from_str::<RoomSummary>(missing).is_err());

    let contradictory = r#"{ "roomId": "!room:example.org", "membership": "join", "isDirect": false, "isEncrypted": false, "encryptionStatus": "encrypted", "unreadCount": 0, "highlightCount": 0, "markedUnread": false }"#;
    assert!(serde_json::from_str::<RoomSummary>(contradictory).is_err());

    let inverse_contradiction = r#"{ "roomId": "!room:example.org", "membership": "join", "isDirect": false, "isEncrypted": true, "encryptionStatus": "not_encrypted", "unreadCount": 0, "highlightCount": 0, "markedUnread": false }"#;
    assert!(serde_json::from_str::<RoomSummary>(inverse_contradiction).is_err());

    let unknown = r#"{ "roomId": "!room:example.org", "membership": "join", "isDirect": false, "isEncrypted": false, "encryptionStatus": "unknown", "unreadCount": 0, "highlightCount": 0, "markedUnread": false }"#;
    let decoded = serde_json::from_str::<RoomSummary>(unknown).expect("unknown remains distinct");
    assert_eq!(decoded.encryption_status, RoomEncryptionStatus::Unknown);
    let encoded = serde_json::to_value(decoded).expect("unknown serializes");
    assert_eq!(encoded["isEncrypted"], false);
    assert_eq!(encoded["encryptionStatus"], "unknown");
}

#[test]
fn member_round_trip_and_fixture() {
    let m = RoomMember {
        room_id: "!room:example.org".into(),
        user_id: "@bob:example.org".into(),
        display_name: Some("Bob".into()),
        avatar_url: None,
        membership: Membership::Join,
        power_level: 50,
        is_direct_target: Some(true),
    };
    round_trip_json(&m);
    let from_fix: RoomMember = load_fixture_as("valid_member.json");
    assert_eq!(from_fix.power_level, 50);
}

#[test]
fn timeline_message_round_trip_and_fixture() {
    let item = TimelineItem::Message(TimelineMessageItem {
        item_id: "$event1".into(),
        event_id: "$event1".into(),
        room_id: "!room:example.org".into(),
        sender: "@alice:example.org".into(),
        origin_server_ts: 1_720_000_000_000,
        body: "hello".into(),
        msgtype: Some("m.text".into()),
        relates_to: None,
        local_echo_state: None,
        is_edited: Some(false),
        is_redacted: Some(false),
        thread_root_id: None,
    });
    assert_eq!(item.kind(), "message");
    round_trip_json(&item);
    let from_fix: TimelineItem = load_fixture_as("valid_timeline_item_message.json");
    assert_eq!(from_fix.kind(), "message");
    assert_eq!(from_fix.item_id(), "$msg1");
}

#[test]
fn timeline_state_fixture() {
    let from_fix: TimelineItem = load_fixture_as("valid_timeline_item_state.json");
    assert_eq!(from_fix.kind(), "state");
    match from_fix {
        TimelineItem::State(s) => {
            assert_eq!(s.state_type, "m.room.name");
        }
        other => panic!("expected state, got {:?}", other.kind()),
    }
}

#[test]
fn timeline_all_kinds_round_trip() {
    let samples = vec![
        TimelineItem::Membership(TimelineMembershipItem {
            item_id: "m1".into(),
            event_id: "$m1".into(),
            room_id: "!r:e".into(),
            sender: "@a:e".into(),
            origin_server_ts: 1,
            target_user_id: "@b:e".into(),
            summary: "joined".into(),
        }),
        TimelineItem::ReactionSummary(TimelineReactionSummaryItem {
            item_id: "rx1".into(),
            event_id: "$target".into(),
            room_id: "!r:e".into(),
            key: "👍".into(),
            count: 3,
            me: Some(true),
        }),
        TimelineItem::Redacted(TimelineRedactedItem {
            item_id: "rd1".into(),
            event_id: "$rd1".into(),
            room_id: "!r:e".into(),
            redacted_by: Some("$by".into()),
        }),
        TimelineItem::EncryptedUnavailable(TimelineEncryptedUnavailableItem {
            item_id: "enc1".into(),
            event_id: "$enc1".into(),
            room_id: "!r:e".into(),
            reason: Some("missing_keys".into()),
        }),
        TimelineItem::DateSeparator(TimelineDateSeparatorItem {
            item_id: "day-2026-07-24".into(),
            day_key: "2026-07-24".into(),
        }),
        TimelineItem::ReadMarker(TimelineReadMarkerItem {
            item_id: "read-marker".into(),
        }),
        TimelineItem::Other(TimelineOtherItem {
            item_id: "other1".into(),
            event_id: Some("$o1".into()),
            event_type: Some("m.sticker".into()),
            summary: Some("sticker".into()),
        }),
    ];
    for item in samples {
        round_trip_json(&item);
    }
}

#[test]
fn relation_fixture() {
    let r: RelationRef = load_fixture_as("valid_relation_reaction.json");
    assert_eq!(r.rel_type, REL_TYPE_ANNOTATION);
    assert_eq!(r.key.as_deref(), Some("👍"));
    round_trip_json(&r);
}

#[test]
fn receipt_fixture() {
    let r: Receipt = load_fixture_as("valid_receipt.json");
    assert_eq!(r.receipt_type, ReceiptType::Read);
    round_trip_json(&r);
}

#[test]
fn typing_fixture() {
    let t: TypingSnapshot = load_fixture_as("valid_typing.json");
    assert_eq!(t.user_ids.len(), 2);
    round_trip_json(&t);
}

#[test]
fn upload_fixture_no_bytes() {
    let u: UploadJob = load_fixture_as("valid_upload.json");
    assert_eq!(u.state, UploadState::Uploading);
    let raw = fixture("valid_upload.json");
    assert!(!raw.contains("fileBytes"));
    assert!(!raw.contains("mediaBytes"));
    assert!(!raw.contains("base64"));
    round_trip_json(&u);
}

#[test]
fn media_handle_fixture_no_bytes_no_keys() {
    let m: MediaHandle = load_fixture_as("valid_media_handle.json");
    assert_eq!(m.source, Some(MediaSource::Mxc));
    let raw = fixture("valid_media_handle.json");
    assert!(!raw.contains("mediaBytes"));
    assert!(!raw.contains("session_key"));
    assert!(!raw.contains("encryption"));
    round_trip_json(&m);
}

#[test]
fn security_status_fixture() {
    let s: SecurityStatus = load_fixture_as("valid_security_status.json");
    assert!(s.cross_signing_active);
    assert_eq!(s.backup_status, BackupStatus::Enabled);
    assert_eq!(s.recovery_status, RecoveryStatus::Ready);
    assert_eq!(s.verification_state, VerificationState::Verified);
    round_trip_json(&s);
}

#[test]
fn notification_candidate_fixture() {
    let n: NotificationCandidate = load_fixture_as("valid_notification_candidate.json");
    assert_eq!(n.kind, NotificationKind::Message);
    assert!(n.suppress_if_focused_room);
    round_trip_json(&n);
}

#[test]
fn search_result_fixture() {
    let s: SearchResult = load_fixture_as("valid_search_result.json");
    assert_eq!(s.query, "hello");
    assert_eq!(s.results.len(), 1);
    round_trip_json(&s);
}

#[test]
fn space_summary_fixture() {
    let s: SpaceSummary = load_fixture_as("valid_space_summary.json");
    assert_eq!(s.children.len(), 2);
    round_trip_json(&s);
}

#[test]
fn thread_summary_fixture() {
    let t: ThreadSummary = load_fixture_as("valid_thread_summary.json");
    assert_eq!(t.reply_count, 4);
    assert!(t.participated);
    round_trip_json(&t);
}

#[test]
fn all_valid_fixtures_parse_as_json_objects() {
    let names = [
        "valid_session.json",
        "valid_room_summary.json",
        "valid_member.json",
        "valid_timeline_item_message.json",
        "valid_timeline_item_state.json",
        "valid_relation_reaction.json",
        "valid_receipt.json",
        "valid_typing.json",
        "valid_upload.json",
        "valid_media_handle.json",
        "valid_security_status.json",
        "valid_notification_candidate.json",
        "valid_search_result.json",
        "valid_space_summary.json",
        "valid_thread_summary.json",
    ];
    for name in names {
        let raw = fixture(name);
        assert_no_forbidden_fields(&raw);
        let v: Value = serde_json::from_str(&raw).unwrap_or_else(|e| {
            panic!("{name}: invalid JSON: {e}");
        });
        assert!(v.is_object(), "{name} must be a JSON object");
    }
}

#[test]
fn room_summary_is_call_defaults_and_round_trips() {
    let s = RoomSummary {
        room_id: "!room:example.org".into(),
        name: Some("Voice HQ".into()),
        canonical_alias: None,
        avatar_url: None,
        membership: Membership::Join,
        is_direct: false,
        is_call: false,
        is_space: false,
        is_favorite: false,
        is_low_priority: false,
        folder_id: None,
        encryption_status: RoomEncryptionStatus::Encrypted,
        join_rule: Some("invite".into()),
        unread_count: 3,
        highlight_count: 0,
        marked_unread: false,
        notification_mode: None,
        last_activity_ts: None,
        last_message_preview: None,
        heroes: None,
        tombstone_successor_room_id: None,
    };
    // Absent field deserializes to false (serde default) — backward compatible.
    let raw = r#"{ "roomId": "!room:example.org", "membership": "join", "isDirect": false, "isEncrypted": true, "encryptionStatus": "encrypted", "unreadCount": 0, "highlightCount": 0, "markedUnread": false }"#;
    let parsed: RoomSummary = serde_json::from_str(raw).expect("default is_call");
    assert!(!parsed.is_call);
    // Explicit true round-trips.
    let s = RoomSummary { is_call: true, ..s };
    let wire = serde_json::to_string(&s).expect("serialize");
    assert!(wire.contains("\"isCall\":true"));
    let back: RoomSummary = serde_json::from_str(&wire).expect("deserialize");
    assert!(back.is_call);
}
