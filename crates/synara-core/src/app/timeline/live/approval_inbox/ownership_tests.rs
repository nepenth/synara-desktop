use super::*;
use matrix_sdk::ruma::{event_id, room_id, RoomVersionId};
use matrix_sdk::test_utils::mocks::{MatrixMockServer, RoomMessagesResponseTemplate};
use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, BOB};

async fn until(mut condition: impl FnMut() -> bool) {
    timeout(Duration::from_secs(5), async {
        while !condition() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("native observation should settle");
}
async fn message_requests(server: &MatrixMockServer) -> usize {
    server
        .server()
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| request.url.path().ends_with("/messages"))
        .count()
}

#[tokio::test]
async fn borrowed_covered_and_uncovered_windows_are_read_only_until_view_release() {
    for (tail, network) in [(0, false), (80, false), (3, true)] {
        let server = MatrixMockServer::new().await;
        let client = server.client_builder().build().await;
        client.event_cache().subscribe().unwrap();
        let id = room_id!("!borrowed:example.org");
        let user = client.user_id().unwrap().to_owned();
        let f = EventFactory::new().room(id);
        let now = agent_approval_now_ms().unwrap();
        let mut joined =
            JoinedRoomBuilder::new(id).add_state_event(f.create(&user, RoomVersionId::V11));
        if network {
            joined = joined.set_timeline_prev_batch("earlier");
            server
                .mock_room_messages()
                .ok(RoomMessagesResponseTemplate::default()
                    .events(vec![
                        f.text_msg("Approval required: dangerous command\nrequest")
                            .sender(*BOB)
                            .event_id(event_id!("$borrowed-prompt"))
                            .server_ts(now)
                            .into_raw_timeline(),
                        f.text_msg("boundary")
                            .sender(*BOB)
                            .server_ts(now - 360_000)
                            .into_raw_timeline(),
                    ])
                    .with_delay(Duration::from_millis(100)))
                .mount()
                .await;
        } else {
            joined = joined
                .add_timeline_event(f.text_msg("boundary").sender(*BOB).server_ts(now - 360_000))
                .add_timeline_event(
                    f.text_msg("Approval required: dangerous command\nrequest")
                        .sender(*BOB)
                        .event_id(event_id!("$borrowed-prompt"))
                        .server_ts(now),
                );
        }
        for i in 0..tail {
            joined = joined
                .add_timeline_event(f.text_msg(format!("newer {i}")).sender(*BOB).server_ts(now));
        }
        let room = server.sync_room(&client, joined).await;
        let borrowed = Arc::new(TimelineBuilder::new(&room).build().await.unwrap());
        let (before, _borrowed_updates) = borrowed.subscribe().await;
        let ids = |items: Vec<Arc<SdkTimelineItem>>| {
            items
                .iter()
                .map(|item| item.unique_id().0.clone())
                .collect::<Vec<_>>()
        };
        let before_ids = ids(before.iter().cloned().collect());
        let history = ApprovalHistory::default().room(id.as_str()).unwrap();
        let protection = history.protect(&room).await;
        let state = Arc::new(std::sync::Mutex::new(RoomSnapshot::default()));
        let task = {
            let (room, user, state, history, borrowed) = (
                room.clone(),
                user.clone(),
                state.clone(),
                history.clone(),
                borrowed.clone(),
            );
            tokio::spawn(async move {
                observe_room_lease_with_timeline(
                    &room,
                    &user,
                    &state,
                    &Semaphore::new(1),
                    ObserverBudget {
                        lifetime: Duration::from_secs(5),
                        max_growth: 512,
                    },
                    Some(borrowed),
                    history,
                )
                .await
            })
        };
        until(|| {
            let state = state.lock().unwrap();
            state.checked || state.deferred
        })
        .await;
        {
            let state = state.lock().unwrap();
            assert!(!state.incomplete);
            assert_eq!(state.deferred, tail > 0);
        }
        assert_eq!(message_requests(&server).await, 0);
        assert_eq!(
            ids(borrowed.subscribe().await.0.iter().cloned().collect()),
            before_ids,
            "inbox must not change borrowed lazy pagination"
        );
        drop(protection);
        until(|| {
            state
                .lock()
                .unwrap()
                .items
                .iter()
                .any(|item| item.event_id == "$borrowed-prompt")
        })
        .await;
        assert!(state.lock().unwrap().checked);
        // Retain the registry-like cached Arc even after view close. Release is
        // proven by the view protection lease, never guessed from Arc counts.
        if !network {
            assert_eq!(
                ids(borrowed.subscribe().await.0.iter().cloned().collect()),
                before_ids
            );
        }
        assert_eq!(
            message_requests(&server).await,
            usize::from(network),
            "network discovery only becomes eligible after view release"
        );
        task.abort();
        let _ = task.await;
    }
}

#[tokio::test]
async fn sdk_independent_timelines_share_pagination_status_and_history() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let id = room_id!("!shared-cache:example.org");
    let f = EventFactory::new().room(id);
    let room = server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .set_timeline_prev_batch("earlier")
                .add_timeline_event(f.text_msg("recent").sender(*BOB)),
        )
        .await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default().with_delay(Duration::from_millis(300)))
        .mount()
        .await;
    let view = TimelineBuilder::new(&room).build().await.unwrap();
    let independent = Arc::new(
        TimelineBuilder::new(&room)
            .track_read_marker_and_receipts(TimelineReadReceiptTracking::Disabled)
            .build()
            .await
            .unwrap(),
    );
    let (_, mut status) = view.live_back_pagination_status().await.unwrap();
    let pagination = tokio::spawn(async move { independent.paginate_backwards(30).await });
    timeout(Duration::from_secs(2), async {
        while status.next().await != Some(PaginationStatus::Paginating) {}
    })
    .await
    .expect("a separate timeline still changes the open view's shared pagination status");
    pagination.await.unwrap().unwrap();
    assert_eq!(message_requests(&server).await, 1);
}

#[tokio::test]
async fn view_open_waits_for_detached_sdk_pagination_and_failed_open_releases_protection() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let id = room_id!("!open-during-discovery:example.org");
    let f = EventFactory::new().room(id);
    let now = agent_approval_now_ms().unwrap();
    let mut joined = JoinedRoomBuilder::new(id)
        .set_timeline_prev_batch("earlier")
        .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11));
    for i in 0..3 {
        joined = joined.add_timeline_event(
            f.text_msg(format!("recent {i}"))
                .sender(*BOB)
                .server_ts(now),
        );
    }
    let room = server.sync_room(&client, joined).await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default()
            .events(vec![f
                .text_msg("older boundary")
                .sender(*BOB)
                .server_ts(now - 360_000)
                .into_raw_timeline()])
            .with_delay(Duration::from_millis(700)))
        .mount()
        .await;
    let owner = Arc::new(NativeTimelineOwner::new(&client, Arc::new(|_| {}), 11));
    owner
        .agent_approvals_list_with_discovery(true)
        .await
        .unwrap();
    timeout(Duration::from_secs(3), async {
        while message_requests(&server).await == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
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
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !opening.is_finished(),
        "dropping the caller cannot quiesce SDK-owned pagination"
    );
    let opened = opening.await.unwrap().unwrap();
    let (cache, _subscription) = room.event_cache().await.unwrap();
    assert!(matches!(
        cache.pagination().status().get(),
        PaginationStatus::Idle { .. }
    ));
    let requests = message_requests(&server).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(message_requests(&server).await, requests);
    let gate = owner
        .registry
        .lock()
        .await
        .approval_history
        .room(id.as_str())
        .unwrap();
    assert!(gate.protected());
    owner
        .registry
        .lock()
        .await
        .close_view(NativeTimelineCloseRequest {
            stream_id: opened.stream_id,
        });
    assert!(!gate.protected());
    let error = owner
        .open_at(NativeTimelineOpenRequest {
            room_id: id.to_string(),
            position: NativeTimelineOpenPosition::Focused {
                event_id: "invalid".into(),
            },
        })
        .await;
    assert!(error.is_err());
    assert!(!gate.protected());
}

#[tokio::test]
async fn view_open_succeeds_when_discovery_pagination_exceeds_protect_timeout() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let id = room_id!("!open-after-pagination-timeout:example.org");
    let f = EventFactory::new().room(id);
    let now = agent_approval_now_ms().unwrap();
    let mut joined = JoinedRoomBuilder::new(id)
        .set_timeline_prev_batch("earlier")
        .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11));
    for i in 0..3 {
        joined = joined.add_timeline_event(
            f.text_msg(format!("recent {i}"))
                .sender(*BOB)
                .server_ts(now),
        );
    }
    let room = server.sync_room(&client, joined).await;
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default()
            .events(vec![f
                .text_msg("older boundary")
                .sender(*BOB)
                .server_ts(now - 360_000)
                .into_raw_timeline()])
            .with_delay(Duration::from_secs(30)))
        .mount()
        .await;
    let owner = Arc::new(NativeTimelineOwner::new(&client, Arc::new(|_| {}), 12));
    owner
        .agent_approvals_list_with_discovery(true)
        .await
        .unwrap();
    timeout(Duration::from_secs(3), async {
        while message_requests(&server).await == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
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
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !opening.is_finished(),
        "open must still wait for in-flight discovery pagination"
    );
    let opened = timeout(Duration::from_secs(12), opening)
        .await
        .expect("stuck discovery pagination must not fail the user's room open")
        .expect("open task")
        .expect("open_at");
    assert_eq!(opened.snapshot.room_id, id.as_str());
    let (cache, _subscription) = room.event_cache().await.unwrap();
    assert!(
        matches!(
            cache.pagination().status().get(),
            PaginationStatus::Paginating
        ),
        "the view must open while the detached inbox page is still in flight"
    );
    let gate = owner
        .registry
        .lock()
        .await
        .approval_history
        .room(id.as_str())
        .unwrap();
    assert!(
        gate.protected(),
        "fail-open still holds protection so the inbox defers"
    );
    owner
        .registry
        .lock()
        .await
        .close_view(NativeTimelineCloseRequest {
            stream_id: opened.stream_id,
        });
    assert!(!gate.protected());
}

#[test]
fn intentional_deferral_preserves_real_failure_and_successful_recovery_clears_it() {
    let state = std::sync::Mutex::new(RoomSnapshot::default());
    publish_observed(
        &state,
        std::iter::empty(),
        "@reader:example.org",
        false,
        true,
    );
    assert!(!state.lock().unwrap().incomplete);
    publish_observed(
        &state,
        std::iter::empty(),
        "@reader:example.org",
        false,
        false,
    );
    assert!(state.lock().unwrap().incomplete);
    publish_observed(
        &state,
        std::iter::empty(),
        "@reader:example.org",
        false,
        true,
    );
    assert!(state.lock().unwrap().incomplete);
    publish_observed(
        &state,
        std::iter::empty(),
        "@reader:example.org",
        true,
        false,
    );
    assert!(!state.lock().unwrap().incomplete);
    assert!(state.lock().unwrap().checked);
}

#[tokio::test]
async fn cached_coverage_and_permissions_survive_pause_then_update_from_native_sync() {
    use matrix_sdk::ruma::events::room::power_levels::RoomPowerLevelsEventContent;
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let id = room_id!("!cached-permission:example.org");
    let user = client.user_id().unwrap().to_owned();
    let f = EventFactory::new().room(id);
    let now = agent_approval_now_ms().unwrap();
    let levels = |level| {
        serde_json::from_value::<RoomPowerLevelsEventContent>(
            serde_json::json!({"users": {user.as_str(): 0}, "events": {"m.reaction": level}}),
        )
        .unwrap()
    };
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(id)
                .add_state_event(f.create(&user, RoomVersionId::V11))
                .add_state_event(f.event(levels(0)).sender(&user).state_key(""))
                .add_timeline_event(f.text_msg("covered").sender(*BOB).server_ts(now - 360_000)),
        )
        .await;
    let mut inbox = ApprovalInboxOwner::new(27);
    inbox
        .snapshot(&client, &HashSet::new(), true, HashMap::new())
        .await
        .unwrap();
    let state = inbox.rooms.get(id.as_str()).unwrap().state.clone();
    until(|| {
        let state = state.lock().unwrap();
        state.checked && state.can_send_reaction
    })
    .await;
    let cached = inbox
        .snapshot(&client, &HashSet::new(), false, HashMap::new())
        .await
        .unwrap();
    assert_eq!(cached.coverage, NativeAgentApprovalInboxCoverage::Discovery);
    assert!(!cached.incomplete && !cached.loading);
    assert!(!state.lock().unwrap().observing);
    for _ in 0..5 {
        let cached = inbox
            .snapshot(&client, &HashSet::new(), false, HashMap::new())
            .await
            .unwrap();
        assert_eq!(cached.coverage, NativeAgentApprovalInboxCoverage::Discovery);
        assert!(!cached.incomplete && !cached.loading);
        assert!(state.lock().unwrap().can_send_reaction);
    }
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(id)
                .add_state_event(f.event(levels(100)).sender(&user).state_key("")),
        )
        .await;
    // No polling or timeline observer is needed to refresh permission.
    until(|| !state.lock().unwrap().can_send_reaction).await;
    server
        .sync_room(&client, JoinedRoomBuilder::new(id).set_timeline_limited())
        .await;
    until(|| !state.lock().unwrap().checked).await;
    let latest = inbox
        .snapshot(&client, &HashSet::new(), false, HashMap::new())
        .await
        .unwrap();
    assert_eq!(
        latest.coverage,
        NativeAgentApprovalInboxCoverage::LatestEvent
    );
    assert!(!latest.incomplete && !latest.loading);
}

#[tokio::test]
async fn idle_list_does_not_keep_a_discovery_gap_as_a_rail_alarm() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let id = room_id!("!idle-gap-alarm:example.org");
    let user = client.user_id().unwrap().to_owned();
    let f = EventFactory::new().room(id);
    let now = agent_approval_now_ms().unwrap();
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(id)
                .set_timeline_prev_batch("earlier")
                .add_state_event(f.create(&user, RoomVersionId::V11))
                .add_timeline_event(f.text_msg("recent only").sender(*BOB).server_ts(now)),
        )
        .await;
    server.mock_room_messages().error500().mount().await;
    let mut inbox = ApprovalInboxOwner::new(29);
    inbox
        .snapshot(&client, &HashSet::new(), true, HashMap::new())
        .await
        .unwrap();
    let state = inbox.rooms.get(id.as_str()).unwrap().state.clone();
    until(|| {
        let state = state.lock().unwrap();
        !state.loading && state.incomplete
    })
    .await;
    for _ in 0..2 {
        let idle = inbox
            .snapshot(&client, &HashSet::new(), false, HashMap::new())
            .await
            .unwrap();
        assert!(
            !idle.incomplete,
            "idle list must not keep a discovery gap as a rail alarm"
        );
        assert_eq!(idle.coverage, NativeAgentApprovalInboxCoverage::LatestEvent);
        assert!(!idle.loading);
    }
    state.lock().unwrap().incomplete = true;
    let proven = inbox
        .snapshot(&client, &HashSet::new(), true, HashMap::new())
        .await
        .unwrap();
    assert!(
        proven.incomplete || proven.loading,
        "returning to the page must still retry the gapped room"
    );
}

#[tokio::test]
async fn borrowed_boundary_loss_is_neutral_until_owned_discovery_resumes() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    client.event_cache().subscribe().unwrap();
    let id = room_id!("!deferred-gap:example.org");
    let f = EventFactory::new().room(id);
    let now = agent_approval_now_ms().unwrap();
    let prompt = event_id!("$deferred-gap-prompt");
    let boundary = event_id!("$deferred-gap-boundary");
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(id)
                .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                .add_timeline_event(
                    f.text_msg("boundary")
                        .sender(*BOB)
                        .event_id(boundary)
                        .server_ts(now - 360_000),
                )
                .add_timeline_event(
                    f.text_msg("Approval required: dangerous command\nrequest")
                        .sender(*BOB)
                        .event_id(prompt)
                        .server_ts(now),
                ),
        )
        .await;
    let owner = NativeTimelineOwner::new(&client, Arc::new(|_| {}), 28);
    let opened = owner
        .open_at(NativeTimelineOpenRequest {
            room_id: id.to_string(),
            position: NativeTimelineOpenPosition::LiveBottom,
        })
        .await
        .unwrap();
    owner
        .agent_approvals_list_with_discovery(true)
        .await
        .unwrap();
    let state = owner
        .approval_inbox
        .lock()
        .await
        .rooms
        .get(id.as_str())
        .unwrap()
        .state
        .clone();
    until(|| state.lock().unwrap().checked).await;
    let covered = owner
        .agent_approvals_list_with_discovery(true)
        .await
        .unwrap();
    assert_eq!(
        covered.coverage,
        NativeAgentApprovalInboxCoverage::Discovery
    );
    assert!(!covered.incomplete && !covered.loading);
    assert_eq!(message_requests(&server).await, 0);
    server
        .mock_room_messages()
        .ok(RoomMessagesResponseTemplate::default()
            .events(vec![
                f.text_msg("Approval required: dangerous command\nrequest")
                    .sender(*BOB)
                    .event_id(prompt)
                    .server_ts(now)
                    .into_raw_timeline(),
                f.text_msg("boundary")
                    .sender(*BOB)
                    .event_id(boundary)
                    .server_ts(now - 360_000)
                    .into_raw_timeline(),
            ])
            .with_delay(Duration::from_millis(100)))
        .mount()
        .await;
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(id)
                .set_timeline_limited()
                .set_timeline_prev_batch("recover-after-close")
                .add_timeline_event(f.text_msg("after reset").sender(*BOB).server_ts(now + 1)),
        )
        .await;
    until(|| state.lock().unwrap().deferred).await;
    let deferred = owner
        .agent_approvals_list_with_discovery(true)
        .await
        .unwrap();
    assert_eq!(
        deferred.coverage,
        NativeAgentApprovalInboxCoverage::LatestEvent
    );
    assert!(!deferred.incomplete && !deferred.loading);
    assert!(deferred.items.is_empty());
    assert_eq!(
        message_requests(&server).await,
        0,
        "a reset cannot start inbox pagination in an open view"
    );
    owner
        .registry
        .lock()
        .await
        .close_view(NativeTimelineCloseRequest {
            stream_id: opened.stream_id,
        });
    until(|| {
        let state = state.lock().unwrap();
        state.checked
            && !state.incomplete
            && state
                .items
                .iter()
                .any(|item| item.event_id == prompt.as_str())
    })
    .await;
    let recovered = owner
        .agent_approvals_list_with_discovery(true)
        .await
        .unwrap();
    assert_eq!(
        recovered.coverage,
        NativeAgentApprovalInboxCoverage::Discovery
    );
    assert!(!recovered.incomplete && !recovered.loading);
    assert_eq!(message_requests(&server).await, 1);
}
