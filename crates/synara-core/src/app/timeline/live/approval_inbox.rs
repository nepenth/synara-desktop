//! Session-scoped, read-only cross-room approval discovery. Live SDK timelines
//! supply edits, redactions, reaction aggregation and later decryption; no UI
//! timeline, read marker, or browser event cache owns the inbox.
use super::*;
use crate::app::agent_approvals::{
    is_eligible_agent_approval_prompt, AGENT_APPROVAL_TERMINAL_REACTIONS, AGENT_APPROVAL_TTL_MS,
};
use tokio::sync::Semaphore;

const MAX_ROOMS: usize = 512;
const MAX_ITEMS: usize = 500;
const HISTORY_MS: u64 = 60 * 60 * 1000;
const BOOTSTRAP_TIMEOUT: Duration = Duration::from_secs(10);
const RETRY_INTERVAL: Duration = Duration::from_secs(30);
const OBSERVER_REOPEN_DELAY: Duration = Duration::from_secs(1);

#[derive(Clone, Copy)]
struct ObserverBudget {
    lifetime: Duration,
    max_growth: usize,
}

const OBSERVER_BUDGET: ObserverBudget = ObserverBudget {
    lifetime: Duration::from_secs(10 * 60),
    max_growth: 512,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NativeAgentApprovalInboxStatus {
    Pending,
    Decided,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeAgentApprovalInboxItem {
    pub room_id: String,
    pub event_id: String,
    pub sender: String,
    pub body: String,
    pub origin_server_ts: u64,
    pub expires_at: u64,
    pub status: NativeAgentApprovalInboxStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeAgentApprovalInboxSnapshot {
    pub session_generation: u64,
    pub items: Vec<NativeAgentApprovalInboxItem>,
    pub loading: bool,
    pub incomplete: bool,
}

#[derive(Default)]
struct RoomSnapshot {
    items: Vec<NativeTimelineItem>,
    loading: bool,
    incomplete: bool,
}

struct RoomObserver {
    state: Arc<std::sync::Mutex<RoomSnapshot>>,
    task: JoinHandle<()>,
    started: tokio::time::Instant,
}

impl Drop for RoomObserver {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub(super) struct ApprovalInboxOwner {
    session_generation: u64,
    rooms: HashMap<String, RoomObserver>,
    bootstrap_slots: Arc<Semaphore>,
}

impl ApprovalInboxOwner {
    pub(super) fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            rooms: HashMap::new(),
            bootstrap_slots: Arc::new(Semaphore::new(4)),
        }
    }

    pub(super) fn snapshot(
        &mut self,
        client: &Client,
        decisions: &ApprovalDecisionRegistry,
    ) -> Result<NativeAgentApprovalInboxSnapshot, &'static str> {
        let own_user = client
            .user_id()
            .ok_or("agent-approval-current-user-missing")?;
        let now = agent_approval_now_ms()?;
        let mut joined = client.joined_rooms();
        joined.sort_by(|left, right| left.room_id().cmp(right.room_id()));
        let truncated_rooms = joined.len() > MAX_ROOMS;
        joined.truncate(MAX_ROOMS);
        let joined_ids: HashSet<_> = joined
            .iter()
            .map(|room| room.room_id().to_string())
            .collect();
        // Aborting departed-room observers also prevents cached requests from
        // another room membership from remaining actionable in this session.
        self.rooms.retain(|id, _| joined_ids.contains(id));
        for room in joined {
            let id = room.room_id().to_string();
            let restart = self.rooms.get(&id).is_some_and(|observer| {
                observer.task.is_finished() && observer.started.elapsed() >= RETRY_INTERVAL
            });
            if restart {
                self.rooms.remove(&id);
            }
            if self.rooms.contains_key(&id) {
                continue;
            }
            let state = Arc::new(std::sync::Mutex::new(RoomSnapshot {
                loading: true,
                ..RoomSnapshot::default()
            }));
            let observer_state = Arc::clone(&state);
            let slots = Arc::clone(&self.bootstrap_slots);
            let own_user = own_user.to_owned();
            let task = tokio::spawn(async move {
                observe_room(room, own_user, observer_state, slots).await;
            });
            self.rooms.insert(
                id,
                RoomObserver {
                    state,
                    task,
                    started: tokio::time::Instant::now(),
                },
            );
        }
        let mut snapshot = NativeAgentApprovalInboxSnapshot {
            session_generation: self.session_generation,
            items: Vec::new(),
            loading: false,
            incomplete: truncated_rooms,
        };
        for (room_id, observer) in &self.rooms {
            let state = observer
                .state
                .lock()
                .map_err(|_| "agent-approval-inbox-state-poisoned")?;
            snapshot.loading |= state.loading;
            snapshot.incomplete |= state.incomplete;
            snapshot.items.extend(state.items.iter().filter_map(|item| {
                project_approval(room_id, item, own_user.as_str(), now, decisions)
            }));
        }
        // Pending first preserves every actionable item before recent history
        // when applying the bounded response limit.
        snapshot.items.sort_by(|left, right| {
            let rank = |item: &NativeAgentApprovalInboxItem| {
                usize::from(item.status != NativeAgentApprovalInboxStatus::Pending)
            };
            rank(left)
                .cmp(&rank(right))
                .then_with(|| right.origin_server_ts.cmp(&left.origin_server_ts))
                .then_with(|| left.event_id.cmp(&right.event_id))
        });
        snapshot.incomplete |= snapshot.items.len() > MAX_ITEMS;
        snapshot.items.truncate(MAX_ITEMS);
        Ok(snapshot)
    }
}

fn project_approval(
    room_id: &str,
    item: &NativeTimelineItem,
    own_user: &str,
    now: u64,
    decisions: &ApprovalDecisionRegistry,
) -> Option<NativeAgentApprovalInboxItem> {
    if !is_eligible_agent_approval_prompt(&item.body, &item.sender, own_user)
        || item.origin_server_ts == 0
        || item.origin_server_ts > now.saturating_add(60_000)
        || now.saturating_sub(item.origin_server_ts) >= HISTORY_MS
        || item.decryption_state.is_some()
    {
        return None;
    }
    let decided = decisions.is_completed(&(room_id.to_owned(), item.event_id.clone()))
        || item.reactions.iter().any(|reaction| {
            reaction.me && AGENT_APPROVAL_TERMINAL_REACTIONS.contains(&reaction.key.as_str())
        });
    let expires_at = item.origin_server_ts.saturating_add(AGENT_APPROVAL_TTL_MS);
    Some(NativeAgentApprovalInboxItem {
        room_id: room_id.to_owned(),
        event_id: item.event_id.clone(),
        sender: item.sender.clone(),
        body: item.body.clone(),
        origin_server_ts: item.origin_server_ts,
        expires_at,
        status: if decided {
            NativeAgentApprovalInboxStatus::Decided
        } else if now >= expires_at {
            NativeAgentApprovalInboxStatus::Expired
        } else {
            NativeAgentApprovalInboxStatus::Pending
        },
    })
}

fn window_covered<'a>(items: impl Iterator<Item = &'a Arc<SdkTimelineItem>>, now: u64) -> bool {
    let cutoff = now.saturating_sub(AGENT_APPROVAL_TTL_MS);
    items.into_iter().any(|item| {
        item.is_timeline_start()
            || item.as_event().is_some_and(|event| {
                let timestamp = u64::from(event.timestamp().get());
                timestamp > 0 && timestamp <= cutoff
            })
    })
}

async fn bootstrap_history(timeline: &Timeline) {
    // SDK .items() contains the full internal cache, but .subscribe() starts
    // with a lazily clipped tail (20 items in SDK 0.18). Only certify the exact
    // subscribed window; lazy pagination must reveal older cached requests too.
    for _ in 0..10 {
        let (items, _updates) = timeline.subscribe().await;
        if window_covered(items.iter(), agent_approval_now_ms().unwrap_or_default()) {
            return;
        }
        match timeline.paginate_backwards(30).await {
            Ok(true) | Err(_) => return,
            Ok(false) => {}
        }
    }
}

fn publish_room(
    state: &std::sync::Mutex<RoomSnapshot>,
    items: impl Iterator<Item = NativeTimelineItem>,
    own_user: &str,
    covered: bool,
) {
    let now = agent_approval_now_ms().unwrap_or_default();
    let mut pending_decryption = false;
    let items = items
        .filter(|item| {
            if item.decryption_state.is_some()
                && now.saturating_sub(item.origin_server_ts) < AGENT_APPROVAL_TTL_MS
            {
                pending_decryption = true;
            }
            now.saturating_sub(item.origin_server_ts) < HISTORY_MS
                && is_eligible_agent_approval_prompt(&item.body, &item.sender, own_user)
        })
        .collect();
    if let Ok(mut state) = state.lock() {
        *state = RoomSnapshot {
            items,
            loading: false,
            incomplete: !covered || pending_decryption,
        };
    }
}

async fn observe_room(
    room: Room,
    own_user: OwnedUserId,
    state: Arc<std::sync::Mutex<RoomSnapshot>>,
    slots: Arc<Semaphore>,
) {
    loop {
        if !observe_room_lease(&room, &own_user, &state, &slots, OBSERVER_BUDGET).await {
            return;
        }
        // Dropping the entire SDK timeline AND update stream releases its room
        // cache subscriptions. SDK 0.18 then auto-shrinks unattended rooms to
        // their last persisted chunk. A short unsubscribed interval lets that
        // task run; rebuilding before dropping would prevent auto-shrink.
        // We never clear/shrink shared caches or disturb an open room's owner.
        tokio::time::sleep(OBSERVER_REOPEN_DELAY).await;
    }
}

async fn observe_room_lease(
    room: &Room,
    own_user: &OwnedUserId,
    state: &std::sync::Mutex<RoomSnapshot>,
    slots: &Semaphore,
    budget: ObserverBudget,
) -> bool {
    let Ok(permit) = slots.acquire().await else {
        return false;
    };
    let built = timeout(BOOTSTRAP_TIMEOUT, async {
        TimelineBuilder::new(room)
            .track_read_marker_and_receipts(TimelineReadReceiptTracking::Disabled)
            .build()
            .await
    })
    .await;
    let timeline = match built {
        Ok(Ok(timeline)) => timeline,
        _ => {
            if let Ok(mut state) = state.lock() {
                state.loading = false;
                state.incomplete = true;
            }
            return false;
        }
    };
    let _ = timeout(BOOTSTRAP_TIMEOUT, bootstrap_history(&timeline)).await;
    let (mut items, mut updates) = timeline.subscribe().await;
    let initial_item_count = items.len();
    let covered = window_covered(items.iter(), agent_approval_now_ms().unwrap_or_default());
    publish_room(
        state,
        items
            .iter()
            .filter_map(|item| project_item(item, Some(own_user))),
        own_user.as_str(),
        covered,
    );
    drop(permit);
    let lease_end = tokio::time::sleep(budget.lifetime);
    tokio::pin!(lease_end);
    let mut retry = tokio::time::interval(RETRY_INTERVAL);
    retry.tick().await;
    if !covered {
        // A bounded first pass can stop at the SDK's skipped cached prefix;
        // continue discovery immediately instead of hiding it for 30 seconds.
        retry.reset_immediately();
    }
    // This cancellable future is polled alongside live diffs, including while
    // waiting for a discovery slot. Dropping the lease drops the future too;
    // it cannot retain an SDK timeline past the observer's lifetime.
    let mut backfill: Option<futures_util::future::BoxFuture<'_, ()>> = None;
    loop {
        let was_covered = window_covered(items.iter(), agent_approval_now_ms().unwrap_or_default());
        tokio::select! {
            update = updates.next() => {
                match update {
                    Some(diffs) => {
                        for diff in diffs { diff.apply(&mut items); }
                    }
                    None => {
                        if let Ok(mut state) = state.lock() { state.incomplete = true; }
                        return false;
                    }
                }
            }
            _ = retry.tick() => {
                if backfill.is_none()
                    && !window_covered(items.iter(), agent_approval_now_ms().unwrap_or_default()) {
                    backfill = Some(Box::pin(async {
                        let Ok(_permit) = slots.acquire().await else { return; };
                        let _ = timeout(BOOTSTRAP_TIMEOUT, bootstrap_history(&timeline)).await;
                    }));
                }
            }
            _ = async {
                if let Some(operation) = backfill.as_mut() { operation.await; }
            }, if backfill.is_some() => {
                backfill = None;
                // Subscribe atomically to a fresh vector after backfill. The
                // old stream may contain a Clear/Reset from a limited sync;
                // never replay its queued diffs against this new snapshot.
                (items, updates) = timeline.subscribe().await;
            }
            _ = &mut lease_end => break,
        }
        // Recompute coverage after EVERY SDK diff. Clear/Reset, and the Remove
        // sequence used when local echoes survive a reset, invalidate the old
        // proof as soon as the boundary event/TimelineStart marker disappears.
        let covered = window_covered(items.iter(), agent_approval_now_ms().unwrap_or_default());
        if was_covered && !covered {
            retry.reset_immediately();
        }
        publish_room(
            state,
            items
                .iter()
                .filter_map(|item| project_item(item, Some(own_user))),
            own_user.as_str(),
            covered,
        );
        if items.len() >= initial_item_count.saturating_add(budget.max_growth) {
            break;
        }
    }
    // Retain known requests while queued for rebootstrap, but never present the
    // paused observer as complete. Decision submission still resolves the exact
    // event through the existing Core authority before any side effect.
    if let Ok(mut state) = state.lock() {
        state.incomplete = true;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    const NOW: u64 = 1_800_000_000_000;
    const ROOM: &str = "!approvals:example.org";
    const USER: &str = "@reader:example.org";
    fn prompt() -> NativeTimelineItem {
        NativeTimelineItem {
            item_id: "prompt".into(),
            event_id: "$prompt".into(),
            sender: "@hermes:example.org".into(),
            event_type: "m.room.message".into(),
            body: "⚠️ Approval required: dangerous command\nrm example".into(),
            origin_server_ts: NOW - 1_000,
            decryption_state: None,
            reactions: Vec::new(),
        }
    }
    #[test]
    fn seed_reactions_are_pending_but_own_decisions_are_terminal() {
        let mut item = prompt();
        item.reactions.push(NativeTimelineReaction {
            key: "✅".into(),
            count: 1,
            me: false,
            senders: Vec::new(),
        });
        let decisions = ApprovalDecisionRegistry::default();
        assert_eq!(
            project_approval(ROOM, &item, USER, NOW, &decisions)
                .unwrap()
                .status,
            NativeAgentApprovalInboxStatus::Pending
        );
        item.reactions[0].me = true;
        assert_eq!(
            project_approval(ROOM, &item, USER, NOW, &decisions)
                .unwrap()
                .status,
            NativeAgentApprovalInboxStatus::Decided
        );
    }
    #[test]
    fn expiry_and_invalid_identity_match_the_decision_owner() {
        let mut item = prompt();
        let decisions = ApprovalDecisionRegistry::default();
        item.origin_server_ts = NOW - AGENT_APPROVAL_TTL_MS;
        assert_eq!(
            project_approval(ROOM, &item, USER, NOW, &decisions)
                .unwrap()
                .status,
            NativeAgentApprovalInboxStatus::Expired
        );
        item.sender = USER.into();
        assert!(project_approval(ROOM, &item, USER, NOW, &decisions).is_none());
        item = prompt();
        item.origin_server_ts = NOW + 60_001;
        assert!(project_approval(ROOM, &item, USER, NOW, &decisions).is_none());
        item.origin_server_ts = 0;
        assert!(project_approval(ROOM, &item, USER, NOW, &decisions).is_none());
    }
    #[test]
    fn decision_readback_wins_before_sync_and_redactions_remove_requests() {
        let mut item = prompt();
        let mut decisions = ApprovalDecisionRegistry::default();
        decisions.remember((ROOM.into(), item.event_id.clone()));
        assert_eq!(
            project_approval(ROOM, &item, USER, NOW, &decisions)
                .unwrap()
                .status,
            NativeAgentApprovalInboxStatus::Decided
        );
        item.body = "Message removed".into();
        assert!(project_approval(ROOM, &item, USER, NOW, &decisions).is_none());
    }
    #[test]
    fn incomplete_coverage_and_decryption_do_not_masquerade_as_an_empty_inbox() {
        let state = std::sync::Mutex::new(RoomSnapshot::default());
        publish_room(&state, std::iter::empty(), USER, false);
        assert!(state.lock().unwrap().incomplete);
        let mut item = prompt();
        item.origin_server_ts = agent_approval_now_ms().unwrap();
        item.decryption_state = Some(NativeDecryptionState::Pending);
        item.body = "Unable to decrypt this message".into();
        publish_room(&state, std::iter::once(item), USER, true);
        let snapshot = state.lock().unwrap();
        assert!(snapshot.incomplete);
        assert!(snapshot.items.is_empty());
    }

    #[tokio::test]
    async fn dropping_session_owner_aborts_live_observers() {
        let mut inbox = ApprovalInboxOwner::new(9);
        let task = tokio::spawn(std::future::pending::<()>());
        let handle = task.abort_handle();
        inbox.rooms.insert(
            ROOM.into(),
            RoomObserver {
                state: Arc::new(std::sync::Mutex::new(RoomSnapshot::default())),
                task,
                started: tokio::time::Instant::now(),
            },
        );
        drop(inbox);
        tokio::task::yield_now().await;
        assert!(handle.is_finished());
    }

    #[tokio::test]
    async fn observer_lease_releases_sdk_cache_retention_without_clearing_shared_history() {
        use matrix_sdk::ruma::{event_id, room_id, RoomVersionId};
        use matrix_sdk::test_utils::mocks::{MatrixMockServer, RoomMessagesResponseTemplate};
        use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, BOB};
        let server = MatrixMockServer::new().await;
        let client = server.client_builder().build().await;
        client.event_cache().subscribe().unwrap();
        let room_id = room_id!("!approval-memory:example.org");
        let f = EventFactory::new().room(room_id);
        let now = agent_approval_now_ms().unwrap();
        let own_user = client.user_id().unwrap().to_owned();
        let mut joined = JoinedRoomBuilder::new(room_id)
            .add_state_event(f.create(&own_user, RoomVersionId::V11))
            .add_timeline_event(
                f.text_msg("before the window")
                    .sender(*BOB)
                    .server_ts(now - 360_000),
            );
        for index in 0..250 {
            joined = joined.add_timeline_event(
                f.text_msg(format!("conversation {index}"))
                    .sender(*BOB)
                    .server_ts(now),
            );
        }
        joined = joined.add_timeline_event(
            f.text_msg("⚠️ Approval required: dangerous command\nCommand: test")
                .sender(*BOB)
                .event_id(event_id!("$retained-prompt"))
                .server_ts(now),
        );
        let room = server.sync_room(&client, joined).await;
        server
            .mock_room_messages()
            .ok(RoomMessagesResponseTemplate::default())
            .mount()
            .await;
        let (cache, _cache_drop) = room.event_cache().await.unwrap();
        let original_count = cache.events().await.unwrap().len();
        assert!(original_count > 200);
        let state = std::sync::Mutex::new(RoomSnapshot::default());
        let slots = Semaphore::new(1);
        assert!(
            observe_room_lease(
                &room,
                &own_user,
                &state,
                &slots,
                ObserverBudget {
                    lifetime: Duration::from_millis(20),
                    max_growth: 512,
                }
            )
            .await
        );
        assert!(
            state.lock().unwrap().incomplete,
            "a released observer is explicitly partial until rebootstrap"
        );
        timeout(Duration::from_secs(2), async {
            while cache.events().await.unwrap().len() >= original_count {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("SDK should auto-shrink after observer subscribers drop");
        // Reopening recovers the pending request from persisted SDK history.
        assert!(
            observe_room_lease(
                &room,
                &own_user,
                &state,
                &slots,
                ObserverBudget {
                    lifetime: Duration::from_millis(20),
                    max_growth: 512,
                }
            )
            .await
        );
        assert!(state
            .lock()
            .unwrap()
            .items
            .iter()
            .any(|item| item.event_id == "$retained-prompt"));
        let requests = server.server().received_requests().await.unwrap();
        assert!(requests
            .iter()
            .all(|request| !request.url.path().contains("/receipt/")
                && !request.url.path().ends_with("/read_markers")
                && !request.url.path().contains("/send/")));
    }

    #[test]
    fn fresh_session_does_not_inherit_decisions() {
        let item = prompt();
        let mut decisions = ApprovalDecisionRegistry::default();
        decisions.remember((ROOM.into(), item.event_id.clone()));
        assert_eq!(
            project_approval(ROOM, &item, USER, NOW, &ApprovalDecisionRegistry::default())
                .unwrap()
                .status,
            NativeAgentApprovalInboxStatus::Pending
        );
        assert!(ApprovalInboxOwner::new(2).rooms.is_empty());
    }
}
