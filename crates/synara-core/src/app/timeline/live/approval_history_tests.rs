use super::*;
use matrix_sdk::ruma::{room_id, RoomVersionId};
use matrix_sdk::test_utils::mocks::{MatrixMockServer, RoomMessagesResponseTemplate};
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, BOB};

#[tokio::test]
async fn approval_history_first_view_initializes_a_fresh_clients_event_cache() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let id = room_id!("!fresh-first-view:example.org");
    let f = EventFactory::new().room(id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(f.text_msg("cached conversation").sender(*BOB)),
        )
        .await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default())
        .mount()
        .await;
    // Production session subscribe is `NativeTimelineOwner::new`. This still
    // proves first view does not require a prior test-only subscribe.
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 21);
    let opened = timeout(
        Duration::from_secs(8),
        owner.open_at(NativeTimelineOpenRequest {
            room_id: id.to_string(),
            position: NativeTimelineOpenPosition::LiveBottom,
        }),
    )
    .await
    .expect("first view should finish without external cache initialization")
    .expect("the history gate must initialize its SDK cache prerequisite");
    assert_eq!(opened.snapshot.room_id, id.as_str());
    let mut registry = owner.registry.lock().await;
    let history = registry.approval_history.room(id.as_str()).unwrap();
    assert!(history.protected());
    assert_eq!(registry.view_streams.len(), 1);
    assert!(registry.close_view(NativeTimelineCloseRequest {
        stream_id: opened.stream_id,
    }));
    assert!(!history.protected());
}

#[tokio::test]
async fn approval_history_cancelled_open_releases_unpublished_view_ownership() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let id = room_id!("!cancelled-first-view:example.org");
    let f = EventFactory::new().room(id);
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(id)
                .set_timeline_prev_batch("earlier")
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(f.text_msg("sparse cached tail").sender(*BOB)),
        )
        .await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default().with_delay(Duration::from_secs(2)))
        .mount()
        .await;
    let owner = Arc::new(NativeTimelineOwner::new(&client, Arc::new(|_| {}), 22));
    let history = owner
        .registry
        .lock()
        .await
        .approval_history
        .room(id.as_str())
        .unwrap();
    let opening = {
        let owner = owner.clone();
        tokio::spawn(async move {
            owner
                .open_at(NativeTimelineOpenRequest {
                    room_id: id.to_string(),
                    position: NativeTimelineOpenPosition::LiveBottom,
                })
                .await
        })
    };
    timeout(Duration::from_secs(3), async {
        loop {
            let requests = server.server().received_requests().await.unwrap();
            if requests
                .iter()
                .any(|request| request.url.path().ends_with("/messages"))
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the protected open must reach awaited sparse-history initialization");
    assert!(history.protected());
    assert!(!opening.is_finished());
    opening.abort();
    assert!(opening.await.unwrap_err().is_cancelled());
    let registry = owner.registry.lock().await;
    assert!(
        !history.protected(),
        "cancelled open must drop its local lease"
    );
    assert!(registry.view_streams.is_empty());
    assert!(registry.view_update_tasks.is_empty());
}

#[test]
fn approval_history_stream_publication_has_no_cancellation_point_before_readback() {
    // The media/snapshot awaits are internal SDK/store operations, with no
    // external test latch. Enforce the critical final publication invariant
    // alongside the real cancellation test above.
    let source = include_str!("../live.rs");
    let publish = source
        .split_once("    fn publish_view(")
        .expect("views are published in one place")
        .1
        .split("\n    }\n")
        .next()
        .unwrap();
    assert!(publish.contains("self.view_streams.insert("));
    assert!(
        !publish.contains(".await"),
        "publication must stay synchronous"
    );
    let open = source
        .split_once("\nasync fn open_view(")
        .expect("view open")
        .1
        .split("\n}\n")
        .next()
        .unwrap();
    let (_, published) = open
        .rsplit_once("registry.publish_view(")
        .expect("the ready view is published last");
    // The only await left takes the registry lock; the closure that
    // publishes runs synchronously once it is held.
    assert_eq!(published.matches(".await").count(), 1);
}

fn history_row(
    event_id: &str,
    decision: SynaraAgentApprovalHistoryDecision,
    decided_at: f64,
) -> SynaraAgentApprovalHistoryItem {
    SynaraAgentApprovalHistoryItem {
        room_id: "!agents:example.org".to_owned(),
        event_id: event_id.to_owned(),
        sender: "@agent:example.org".to_owned(),
        decision,
        decided_at,
        origin_server_ts: 1_000.0,
        expires_at: 301_000.0,
        summary: "rm -rf build".to_owned(),
    }
}

#[test]
fn already_decided_reports_the_recorded_decision_not_the_clicked_one() {
    let items = vec![
        history_row(
            "$other",
            SynaraAgentApprovalHistoryDecision::ApproveOnce,
            5.0,
        ),
        history_row(
            "$prompt",
            SynaraAgentApprovalHistoryDecision::ApproveOnce,
            10.0,
        ),
        history_row("$prompt", SynaraAgentApprovalHistoryDecision::Deny, 20.0),
    ];
    let recorded = recorded_history_decision(&items, "!agents:example.org", "$prompt");
    assert_eq!(
        recorded,
        Some(TimelineAgentApprovalDecision {
            decision: SynaraAgentApprovalHistoryDecision::Deny,
            decided_at: Some(20),
        })
    );
    let result = already_decided_result(
        "!agents:example.org".to_owned(),
        "$prompt".to_owned(),
        recorded,
    );
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["status"], "already_decided");
    assert_eq!(json["decision"], "deny");
    assert_eq!(json["decidedAt"], 20);
    assert!(json.get("reaction").is_none());

    assert_eq!(
        recorded_history_decision(&items, "!agents:example.org", "$missing"),
        None
    );
    let unknown = already_decided_result("!r:x".to_owned(), "$e".to_owned(), None);
    let json = serde_json::to_value(&unknown).unwrap();
    assert!(json.get("decision").is_none());
    assert!(json.get("decidedAt").is_none());
}

#[test]
fn history_decision_without_a_valid_time_reports_no_time() {
    let items = vec![history_row(
        "$prompt",
        SynaraAgentApprovalHistoryDecision::ApproveOnce,
        f64::NAN,
    )];
    assert_eq!(
        recorded_history_decision(&items, "!agents:example.org", "$prompt"),
        Some(TimelineAgentApprovalDecision {
            decision: SynaraAgentApprovalHistoryDecision::ApproveOnce,
            decided_at: None,
        })
    );
}
