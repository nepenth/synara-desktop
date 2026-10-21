//! Desktop timeline adapter tests: UTD index (P5.10) and pure-module mirrors.
//!
//! The P5.1-P5.4 harness tests moved to synara-core with the harness
//! modules, which are now compiled for tests only.

use super::*;
use synara_core::app::timeline::{
    TimelinePageState, TimelinePaginationState, TimelineReadState, TimelineViewCapabilities,
    TimelineViewPosition,
};

// --- P5.10 UTD / decryption update propagation ---

fn enc(
    item: &str,
    event: &str,
    room: &str,
) -> crate::matrix::dto::TimelineEncryptedUnavailableItem {
    crate::matrix::dto::TimelineEncryptedUnavailableItem {
        item_id: item.into(),
        event_id: event.into(),
        room_id: room.into(),
        reason: Some("missing_keys".into()),
    }
}

#[test]
fn p5_10_mark_retry_decrypt_flow() {
    let mut idx = UtdIndex::new(2);
    let u = idx
        .mark_unavailable(
            enc("i1", "$e1:example.org", "!r:example.org"),
            UtdReasonCode::MissingKeys,
        )
        .unwrap();
    assert!(matches!(u, UtdUpdate::MarkedUnavailable { .. }));
    assert_eq!(idx.active_utd_count(), 1);

    idx.begin_retry("!r:example.org", "$e1:example.org")
        .unwrap();
    assert_eq!(
        idx.get("!r:example.org", "$e1:example.org").unwrap().phase,
        UtdPhase::RetryPending
    );
    assert_eq!(
        idx.get("!r:example.org", "$e1:example.org")
            .unwrap()
            .retry_count,
        1
    );

    let u = idx
        .mark_decrypted("!r:example.org", "$e1:example.org")
        .unwrap();
    assert!(matches!(u, UtdUpdate::Decrypted { .. }));
    assert_eq!(idx.active_utd_count(), 0);
    assert_eq!(
        idx.get("!r:example.org", "$e1:example.org").unwrap().phase,
        UtdPhase::Decrypted
    );
}

#[test]
fn p5_10_retry_failed_then_permanent() {
    let mut idx = UtdIndex::new(1);
    idx.mark_unavailable(
        enc("i1", "$e1:example.org", "!r:example.org"),
        UtdReasonCode::Historical,
    )
    .unwrap();
    idx.begin_retry("!r:example.org", "$e1:example.org")
        .unwrap();
    idx.retry_failed("!r:example.org", "$e1:example.org", "p5.10-key-not-yet")
        .unwrap();
    assert_eq!(
        idx.get("!r:example.org", "$e1:example.org").unwrap().phase,
        UtdPhase::UnableToDecrypt
    );

    idx.begin_retry("!r:example.org", "$e1:example.org")
        .unwrap();
    let u = idx
        .mark_permanent_failure("!r:example.org", "$e1:example.org", "p5.10-unrecoverable")
        .unwrap();
    assert!(matches!(u, UtdUpdate::PermanentFailure { .. }));
    let err = idx
        .begin_retry("!r:example.org", "$e1:example.org")
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.10-permanent-no-retry");
}

#[test]
fn p5_10_forbidden_reason_and_diagnostic() {
    let mut idx = UtdIndex::new(1);
    let mut bad = enc("i1", "$e1:example.org", "!r:example.org");
    bad.reason = Some("session_key=abc".into());
    let err = idx.mark_unavailable(bad, UtdReasonCode::Other).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.10-forbidden-reason");

    idx.mark_unavailable(
        enc("i1", "$e1:example.org", "!r:example.org"),
        UtdReasonCode::MissingKeys,
    )
    .unwrap();
    idx.begin_retry("!r:example.org", "$e1:example.org")
        .unwrap();
    let err = idx
        .retry_failed("!r:example.org", "$e1:example.org", "leak-access_token")
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.10-forbidden-diagnostic");
}

#[test]
fn p5_10_list_active_and_gc_and_retire() {
    let mut idx = UtdIndex::new(1);
    idx.mark_unavailable(
        enc("i1", "$a:example.org", "!r:example.org"),
        UtdReasonCode::MissingKeys,
    )
    .unwrap();
    idx.mark_unavailable(
        enc("i2", "$b:example.org", "!r:example.org"),
        UtdReasonCode::Withheld,
    )
    .unwrap();
    idx.mark_unavailable(
        enc("i3", "$c:example.org", "!other:example.org"),
        UtdReasonCode::Other,
    )
    .unwrap();
    assert_eq!(idx.list_active_for_room("!r:example.org").len(), 2);

    idx.mark_decrypted("!r:example.org", "$a:example.org")
        .unwrap();
    assert_eq!(idx.gc_decrypted(), 1);
    assert!(idx.get("!r:example.org", "$a:example.org").is_none());

    idx.retire_generation(9);
    assert_eq!(idx.session_generation(), 9);
    assert!(idx.is_empty());
}

#[test]
fn p5_10_invalid_ids() {
    let mut idx = UtdIndex::new(1);
    let err = idx
        .mark_unavailable(
            enc("i1", "not-event", "!r:example.org"),
            UtdReasonCode::Other,
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p5.10-invalid-event-id");
}

// ---- SNC-P1-5c pure-module mirrors ------------------------------
// The pure timeline modules (actions, composer, media, view) moved into
// `synara_core::app::timeline` with their internal unit tests. These mirrors
// exercise the same product contracts through the src-tauri adapter
// re-exports so the desktop test count stays identical to the pre-move
// baseline (same shape as SNC-P1-5a/b mirrors).

mod actions_pure {
    use super::*;
    use synara_core::app::timeline::should_attach_formatted_body;

    #[test]
    fn formatted_body_attaches_only_when_it_differs_from_plain_text() {
        assert!(!should_attach_formatted_body("hello", Some("hello")));
        assert!(!should_attach_formatted_body("hello", Some("  ")));
        assert!(should_attach_formatted_body("hello", Some("<p>hello</p>")));
    }

    #[test]
    fn forward_plain_and_quote_bodies_attribute_the_source_sender() {
        assert_eq!(
            format_forwarded_plain_body("@alice:example.org", "hello", false),
            "Forwarded from @alice:example.org\n\nhello"
        );
        assert_eq!(
            format_forwarded_plain_body("@alice:example.org", "hello\nthere", true),
            "> <@alice:example.org>\n> hello\n> there"
        );
    }

    #[test]
    fn action_request_schemas_stay_room_addressed() {
        let edit: NativeTimelineEditTextRequest = serde_json::from_value(serde_json::json!({
            "roomId": "!room:example.org",
            "eventId": "$edit:example.org",
            "body": "updated"
        }))
        .unwrap();
        assert_eq!(edit.event_id, "$edit:example.org");

        let media: NativeTimelineForwardMediaRequest = serde_json::from_value(serde_json::json!({
            "sourceRoomId": "!source:example.org",
            "eventId": "$media:example.org",
            "targetRoomId": "!target:example.org",
            "confirmedEncryptionDowngrade": false
        }))
        .unwrap();
        assert_eq!(media.event_id, "$media:example.org");
        assert_eq!(
            format_forwarded_media_body("@alice:example.org", "photo.jpg"),
            "Forwarded from @alice:example.org\n\nphoto.jpg"
        );

        let vote: NativeTimelinePollVoteRequest = serde_json::from_value(serde_json::json!({
            "roomId": "!room:example.org",
            "eventId": "$poll:example.org",
            "answerIds": ["a1", "a2"]
        }))
        .unwrap();
        assert_eq!(vote.answer_ids, vec!["a1", "a2"]);

        let decline: NativeTimelineCallDeclineRequest = serde_json::from_value(serde_json::json!({
            "roomId": "!room:example.org",
            "eventId": "$rtc:example.org"
        }))
        .unwrap();
        assert_eq!(decline.event_id, "$rtc:example.org");

        let redact: NativeTimelineRedactRequest = serde_json::from_value(serde_json::json!({
            "roomId": "!room:example.org",
            "eventId": "$redact:example.org",
            "reason": "spam"
        }))
        .unwrap();
        assert_eq!(redact.reason.as_deref(), Some("spam"));

        let forward: NativeTimelineForwardTextRequest = serde_json::from_value(serde_json::json!({
            "sourceRoomId": "!source:example.org",
            "eventId": "$fwd:example.org",
            "targetRoomId": "!target:example.org",
            "asQuote": true,
            "confirmedEncryptionDowngrade": true
        }))
        .unwrap();
        assert!(forward.as_quote);
        assert!(forward.confirmed_encryption_downgrade);

        let readback = NativeTimelineActionReadback {
            schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
            action: NativeTimelineActionKind::EditText,
            room_id: "!room:example.org".into(),
            event_id: "$new:example.org".into(),
            status: "sent".into(),
        };
        let json = serde_json::to_value(readback).unwrap();
        assert_eq!(json["schemaVersion"], 1);
        assert_eq!(json["action"], "edit_text");
        assert_eq!(json["status"], "sent");
    }
}

mod composer_pure {
    use super::*;

    #[test]
    fn registry_set_get_and_clear_are_room_scoped() {
        let mut registry = ComposerDraftRegistry::new();
        let draft = NativeComposerReplyDraft {
            draft_revision: 0,
            event_id: "$evt:example.org".into(),
            sender_id: "@alice:example.org".into(),
            body: "hello".into(),
            formatted_body: Some("<p>hello</p>".into()),
            thread_root_event_id: None,
        };
        let draft = registry.set("!room:example.org".into(), draft);
        assert_eq!(registry.get("!room:example.org", None), Some(&draft));
        assert!(registry.get("!other:example.org", None).is_none());
        assert!(registry
            .compare_and_clear("!room:example.org", None, draft.draft_revision)
            .is_none());
        assert!(registry.get("!room:example.org", None).is_none());
        assert_eq!(
            reply_draft_readback(
                "!room:example.org".into(),
                synara_core::dto::ComposerDraftStatus::Cleared,
                None
            )
            .status,
            "cleared"
        );
    }

    #[test]
    fn set_reply_draft_request_accepts_optional_start_thread() {
        let request: NativeComposerSetReplyDraftRequest =
            serde_json::from_value(serde_json::json!({
                "roomId": "!room:example.org",
                "eventId": "$evt:example.org",
                "startThread": true
            }))
            .unwrap();
        assert!(request.start_thread);
        assert_eq!(request.event_id, "$evt:example.org");
    }
}

mod media_pure {
    use super::*;
    use matrix_sdk::ruma::events::room::MediaSource;

    #[test]
    fn handles_are_opaque_and_revoked_with_their_timeline_item() {
        let mut registry = TimelineMediaRegistry::new(7, "live:!room:example.org");
        let handle = registry
            .register(
                "item-1",
                MediaSource::Plain("mxc://example.org/image".into()),
                Some("image/png".into()),
                Some(32),
                Some(16),
                None,
            )
            .unwrap();
        let json = serde_json::to_string(&handle).unwrap();
        assert!(!json.contains("mxc://"));
        assert!(is_timeline_media_handle(&handle.handle_id));
        assert_eq!(
            handle.handle_id.len(),
            TIMELINE_MEDIA_HANDLE_PREFIX.len() + 64
        );
        assert_eq!(registry.len(), 1);
        assert!(registry.resolve(&handle.handle_id).is_some());
        assert_eq!(registry.revoke_item("item-1"), 1);
        assert!(registry.resolve(&handle.handle_id).is_none());
    }

    #[test]
    fn reprojection_is_stable_and_retention_is_stream_bound() {
        let mut registry = TimelineMediaRegistry::new(7, "focused:!room:example.org:$event");
        let first = registry
            .register(
                "item-1",
                MediaSource::Plain("mxc://example.org/one".into()),
                Some("image/png".into()),
                None,
                None,
                None,
            )
            .unwrap();
        let reprojected = registry
            .register(
                "item-1",
                MediaSource::Plain("mxc://example.org/one".into()),
                Some("image/png".into()),
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(first.handle_id, reprojected.handle_id);
        // Edited media is a different source and gets its own handle.
        let edited = registry
            .register(
                "item-1",
                MediaSource::Plain("mxc://example.org/two".into()),
                Some("image/jpeg".into()),
                None,
                None,
                None,
            )
            .unwrap();
        assert_ne!(first.handle_id, edited.handle_id);
        assert!(registry.resolve(&first.handle_id).is_none());
        assert_eq!(registry.session_generation(), 7);
        assert_eq!(registry.stream_id(), "focused:!room:example.org:$event");
        registry.retain_items(["another-item"]);
        assert!(registry.resolve(&edited.handle_id).is_none());
    }
}

mod view_pure {
    use super::*;
    use matrix_sdk::ruma::events::room::message::{
        EmoteMessageEventContent, NoticeMessageEventContent, TextMessageEventContent,
    };
    use std::collections::HashMap;
    use synara_core::app::timeline::TimelineForwardTransport;

    use matrix_sdk::ruma::events::room::message::MessageType;
    #[test]
    fn formatted_body_projects_distinct_html_only() {
        let rich = MessageType::Text(TextMessageEventContent::html(
            "hello",
            "<p><strong>hello</strong></p>",
        ));
        assert_eq!(
            project_formatted_body(&rich).as_deref(),
            Some("<p><strong>hello</strong></p>")
        );

        let plain = MessageType::Text(TextMessageEventContent::plain("hello"));
        assert_eq!(project_formatted_body(&plain), None);

        let same = MessageType::Notice(NoticeMessageEventContent::html("note", "note"));
        assert_eq!(project_formatted_body(&same), None);

        let emote = MessageType::Emote(EmoteMessageEventContent::html("waves", "<em>waves</em>"));
        assert_eq!(
            project_formatted_body(&emote).as_deref(),
            Some("<em>waves</em>")
        );
    }

    #[test]
    fn message_type_labels_cover_text_notice_and_emote() {
        assert_eq!(
            project_message_type_and_media(
                "item",
                &MessageType::Text(TextMessageEventContent::plain("hi")),
                None
            )
            .0
            .as_deref(),
            Some("text")
        );
        assert_eq!(
            project_message_type_and_media(
                "item",
                &MessageType::Notice(NoticeMessageEventContent::plain("hi")),
                None
            )
            .0
            .as_deref(),
            Some("notice")
        );
        assert_eq!(
            project_message_type_and_media(
                "item",
                &MessageType::Emote(EmoteMessageEventContent::plain("hi")),
                None
            )
            .0
            .as_deref(),
            Some("emote")
        );
    }

    #[test]
    fn poll_answers_project_counts_and_own_without_voter_ids() {
        let mut votes = HashMap::new();
        votes.insert(
            "a1".into(),
            vec!["@alice:example.org".into(), "@bob:example.org".into()],
        );
        votes.insert("a2".into(), vec!["@carol:example.org".into()]);

        let answers = project_poll_answers(
            [
                ("a1".into(), "Yes".into()),
                ("a2".into(), "No".into()),
                ("a3".into(), "Maybe".into()),
            ],
            &votes,
            Some("@alice:example.org"),
        );

        assert_eq!(
            answers,
            vec![
                TimelinePollAnswer {
                    id: "a1".into(),
                    text: "Yes".into(),
                    vote_count: 2,
                    own: true,
                },
                TimelinePollAnswer {
                    id: "a2".into(),
                    text: "No".into(),
                    vote_count: 1,
                    own: false,
                },
                TimelinePollAnswer {
                    id: "a3".into(),
                    text: "Maybe".into(),
                    vote_count: 0,
                    own: false,
                },
            ]
        );

        let row = TimelinePollRow {
            event: TimelineEventRowBase {
                item_id: "poll-item".into(),
                event_id: Some("$poll:example.org".into()),
                sender_id: "@alice:example.org".into(),
                sender_name: "@alice:example.org".into(),
                sender_avatar_url: None,
                origin_server_ts: 1,
                local_echo_state: None,
                send_failure: None,
                send_failure_retryable: None,
                transaction_id: None,
                encryption_shield: None,
                capabilities: TimelineRowCapabilities {
                    react: true,
                    reply: false,
                    edit: false,
                    redact: true,
                    report: false,
                    pin: true,
                    forward: false,
                    vote: true,
                    decline_call: false,
                },
            },
            question: "Lunch?".into(),
            closed: false,
            max_selections: 1,
            answers,
            reply: None,
            thread_root: None,
            thread: None,
            reactions: Vec::new(),
        };
        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("\"voteCount\":2"));
        assert!(json.contains("\"own\":true"));
        assert!(!json.contains("@bob:example.org"));
        assert!(!json.contains("@carol:example.org"));
        assert!(!json.contains("token"));
        assert!(!json.contains("ciphertext"));
    }

    #[test]
    fn reply_and_thread_summary_serialize_product_shape_without_secrets() {
        let reply = TimelineReplyPreview {
            event_id: "$parent:example.org".into(),
            sender_id: Some("@alice:example.org".into()),
            sender_name: "alice".into(),
            body: "Earlier message".into(),
        };
        let thread = TimelineThreadSummary {
            root_event_id: "$root:example.org".into(),
            reply_count: 3,
            latest_event_id: Some("$latest:example.org".into()),
        };
        let message = TimelineMessageRow {
            event: TimelineEventRowBase {
                item_id: "msg-item".into(),
                event_id: Some("$msg:example.org".into()),
                sender_id: "@bob:example.org".into(),
                sender_name: "@bob:example.org".into(),
                sender_avatar_url: None,
                origin_server_ts: 1,
                local_echo_state: None,
                send_failure: None,
                send_failure_retryable: None,
                transaction_id: None,
                encryption_shield: None,
                capabilities: TimelineRowCapabilities {
                    react: true,
                    reply: true,
                    edit: true,
                    redact: true,
                    report: false,
                    pin: true,
                    forward: true,
                    vote: false,
                    decline_call: false,
                },
            },
            body: "Reply body".into(),
            formatted_body: None,
            is_agent_approval: false,
            agent_approval_decision: None,
            message_type: Some("text".into()),
            forward_transport: Some(TimelineForwardTransport::Text),
            media_filename: None,
            media_caption: None,
            edited: false,
            reply: Some(reply),
            thread_root: None,
            thread: Some(thread),
            reactions: Vec::new(),
            media: None,
            agent_card_json: None,
        };
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"eventId\":\"$parent:example.org\""));
        assert!(json.contains("\"rootEventId\":\"$root:example.org\""));
        assert!(json.contains("\"replyCount\":3"));
        assert!(json.contains("\"latestEventId\":\"$latest:example.org\""));
        assert!(!json.contains("ciphertext"));
        assert!(!json.contains("access_token"));
        assert!(!json.contains("mxc://"));
    }

    #[test]
    fn snapshot_and_delta_project_pinned_event_ids_without_secrets() {
        let snapshot = TimelineViewSnapshot {
            schema_version: TIMELINE_VIEW_SCHEMA_VERSION,
            session_generation: 1,
            room_id: "!room:example.org".into(),
            revision: 0,
            position: TimelineViewPosition::LiveBottom,
            pagination: TimelinePaginationState {
                backward: TimelinePageState::Available,
                forward: TimelinePageState::Available,
            },
            read_state: TimelineReadState {
                visible_tail_event_id: None,
                receipt_tail_event_id: None,
                own_read_event_id: None,
                unread_anchor_event_id: None,
                is_marked_unread: false,
            },
            pinned_event_ids: vec!["$pin:example.org".into(), "$pin2:example.org".into()],
            rows: Vec::new(),
            capabilities: TimelineViewCapabilities {
                mark_read: true,
                mark_unread: true,
                paginate_backward: true,
                paginate_forward: true,
                can_redact_own: false,
                can_redact_other: false,
            },
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("\"pinnedEventIds\""));
        assert!(json.contains("$pin:example.org"));
        assert!(!json.contains("ciphertext"));
        assert!(!json.contains("access_token"));

        let batch = TimelineViewDeltaBatch {
            schema_version: TIMELINE_VIEW_SCHEMA_VERSION,
            session_generation: 1,
            stream_id: "live:!room:example.org:1".into(),
            room_id: "!room:example.org".into(),
            revision: 1,
            ops: Vec::new(),
            read_state: None,
            pagination: None,
            pinned_event_ids: Some(vec!["$pin:example.org".into()]),
        };
        let batch_json = serde_json::to_string(&batch).unwrap();
        assert!(batch_json.contains("\"pinnedEventIds\""));
        assert!(batch_json.contains("$pin:example.org"));
    }
}

// --- L20 timeline view delta channels ---

#[test]
fn l20_timeline_view_channel_commands_are_registered_for_the_main_window_only() {
    let commands = include_str!("product_commands.rs");
    let lib = include_str!("../../lib.rs");
    let build = include_str!("../../../build.rs");
    let main = include_str!("../../../capabilities/main.json");
    let widget = include_str!("../../../capabilities/widget.json");
    for (command, permission, file) in [
        (
            "desktop_timeline_subscribe",
            "allow-desktop-timeline-subscribe",
            include_str!("../../../permissions/autogenerated/desktop_timeline_subscribe.toml"),
        ),
        (
            "desktop_timeline_unsubscribe",
            "allow-desktop-timeline-unsubscribe",
            include_str!("../../../permissions/autogenerated/desktop_timeline_unsubscribe.toml"),
        ),
    ] {
        assert!(commands.contains(&format!("fn {command}(")));
        assert!(lib.contains(&format!("matrix::auth::product::{command},")));
        assert!(build.contains(&format!("\"{command}\"")));
        assert!(main.contains(&format!("\"{permission}\"")));
        assert!(!widget.contains(permission));
        assert!(file.contains(&format!("commands.allow = [\"{command}\"]")));
        for schema in [
            include_str!("../../../gen/schemas/desktop-schema.json"),
            include_str!("../../../gen/schemas/linux-schema.json"),
            include_str!("../../../gen/schemas/macOS-schema.json"),
        ] {
            assert!(schema.contains(permission));
        }
    }
    // The event path is gone: batches only reach the views' channels.
    let live = include_str!("live.rs");
    assert!(!live.contains("emit_to"));
    assert!(live.contains("channels.deliver(&batch)"));
    // A reload closes the old page's channels.
    assert!(lib.contains("PageLoadEvent::Started"));
    assert!(lib.contains("channels.close_webview(window.label())"));
    // Opens and jumps claim their stream for the view's channel; a close
    // releases it.
    assert_eq!(
        commands
            .matches("channels.claim_stream(subscription_id, &opened.stream_id)")
            .count(),
        2
    );
    assert!(commands.contains("channels.release_stream(&request.stream_id)"));
}
