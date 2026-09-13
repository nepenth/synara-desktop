//! Session-scoped, read-only cross-room approval discovery. Live SDK timelines
//! supply edits, redactions, reaction aggregation and later decryption; no UI
//! timeline, read marker, or browser event cache owns the inbox.
use super::approval_history::{ApprovalHistory, RoomHistory};
use super::*;
use crate::app::agent_approvals::{
    classify_agent_approval, is_eligible_agent_approval_prompt, AGENT_APPROVAL_TTL_MS,
};
use tokio::sync::Semaphore;
mod room_monitor;
use room_monitor::RoomMonitor;

const MAX_ROOMS: usize = 512;
const MAX_ITEMS: usize = 500;
const MAX_OBSERVERS: usize = 32;
const PREVIEW_CHARS: usize = 4_000;
const DISCOVERY_LEASE_MS: u64 = 30_000;
const DISCOVERY_BATCH_MS: u64 = 5_000;
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
    pub can_send_reaction: bool,
    pub body_truncated: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NativeAgentApprovalInboxCoverage {
    LatestEvent,
    Discovery,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeAgentApprovalInboxSnapshot {
    pub session_generation: u64,
    pub items: Vec<NativeAgentApprovalInboxItem>,
    pub loading: bool,
    pub incomplete: bool,
    /// Discovery proves only the pending five-minute window. History is retained
    /// if observed; this is not an exhaustive one-hour history search.
    pub coverage_window_ms: u64,
    pub coverage: NativeAgentApprovalInboxCoverage,
}

#[derive(Default)]
struct RoomSnapshot {
    items: Vec<NativeTimelineItem>,
    loading: bool,
    incomplete: bool,
    checked: bool,
    deferred: bool,
    observing: bool,
    can_send_reaction: bool,
}

struct RoomObserver {
    state: Arc<std::sync::Mutex<RoomSnapshot>>,
    task: Option<JoinHandle<()>>,
    started_ms: u64,
}

impl Drop for RoomObserver {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

pub(super) struct ApprovalInboxOwner {
    session_generation: u64,
    rooms: HashMap<String, RoomObserver>,
    bootstrap_slots: Arc<Semaphore>,
    discovery_until: Arc<AtomicU64>,
    history: Arc<ApprovalHistory>,
    room_monitor: Option<RoomMonitor>,
}

impl ApprovalInboxOwner {
    #[cfg(test)]
    pub(super) fn new(session_generation: u64) -> Self {
        Self::with_history(session_generation, Arc::new(ApprovalHistory::default()))
    }

    pub(super) fn with_history(session_generation: u64, history: Arc<ApprovalHistory>) -> Self {
        Self {
            session_generation,
            rooms: HashMap::new(),
            bootstrap_slots: Arc::new(Semaphore::new(4)),
            discovery_until: Arc::new(AtomicU64::new(0)),
            history,
            room_monitor: None,
        }
    }

    pub(super) async fn snapshot(
        &mut self,
        client: &Client,
        decisions: &HashSet<(String, String)>,
        discovery_active: bool,
        reusable: HashMap<String, Weak<Timeline>>,
    ) -> Result<NativeAgentApprovalInboxSnapshot, &'static str> {
        let own_user = client
            .user_id()
            .ok_or("agent-approval-current-user-missing")?;
        let now = agent_approval_now_ms()?;
        if self.room_monitor.is_none() {
            self.room_monitor = Some(RoomMonitor::new(client));
        }
        self.discovery_until.store(
            if discovery_active {
                now.saturating_add(DISCOVERY_LEASE_MS)
            } else {
                0
            },
            Ordering::Relaxed,
        );
        let mut joined = client.joined_rooms();
        // Matrix spaces are navigation containers, not approval conversations.
        joined.retain(|room| !room.is_space());
        joined.sort_by(|a, b| a.room_id().cmp(b.room_id()));
        let truncated_rooms = joined.len() > MAX_ROOMS;
        joined.truncate(MAX_ROOMS);
        let joined_ids: HashSet<_> = joined.iter().map(|r| r.room_id().to_string()).collect();
        self.rooms.retain(|id, _| joined_ids.contains(id));

        // Looking at latest_event is a local cache read: badge polling never
        // creates a live timeline for every joined room. Known pending requests
        // remain candidates even after a newer ordinary message becomes latest.
        let candidates: HashMap<String, u64> = joined
            .iter()
            .filter_map(|room| {
                let id = room.room_id().to_string();
                let known = self
                    .rooms
                    .get(&id)
                    .and_then(|entry| entry.state.lock().ok())
                    .map(|state| pending_until(&state, own_user.as_str(), now, decisions, &id))
                    .unwrap_or_default();
                let until = known.max(latest_candidate_until(room, own_user.as_str(), now));
                (until > now).then_some((id, until))
            })
            .collect();
        let running = self
            .rooms
            .values()
            .filter(|entry| entry.task.is_some())
            .count();
        let is_new_candidate = |id: &str, started_ms: u64| {
            candidates
                .get(id)
                .is_some_and(|until| until.saturating_sub(AGENT_APPROVAL_TTL_MS) > started_ms)
        };
        let waiting_candidates = candidates
            .keys()
            .filter(|id| {
                self.rooms.get(*id).is_none_or(|entry| {
                    entry.task.is_none() && is_new_candidate(id, entry.started_ms)
                })
            })
            .count();
        let mut preempt = running
            .saturating_add(waiting_candidates.min(MAX_OBSERVERS))
            .saturating_sub(MAX_OBSERVERS);
        let eligible_rooms = if discovery_active {
            joined.len()
        } else {
            candidates.len()
        };
        let mut released = Vec::new();
        for (id, entry) in &mut self.rooms {
            let finished = entry.task.as_ref().is_some_and(|task| task.is_finished());
            // Under pressure every room gets another turn, including ordinary
            // rooms whose latest message could hide an earlier prompt. The
            // single index and successful checks survive normal rotation.
            let rotate = eligible_rooms > MAX_OBSERVERS
                && now.saturating_sub(entry.started_ms) >= DISCOVERY_BATCH_MS
                && entry.state.lock().is_ok_and(|state| !state.loading);
            let make_room_for_candidate =
                preempt > 0 && !candidates.contains_key(id) && entry.task.is_some();
            if make_room_for_candidate {
                preempt -= 1;
            }
            if finished
                || rotate
                || make_room_for_candidate
                || (!discovery_active && !candidates.contains_key(id))
            {
                if let Some(task) = entry.task.take() {
                    task.abort();
                    released.push(task);
                }
            }
        }
        // Await cancellation before replacing slots so the retention cap
        // applies to live SDK subscriptions, not merely stored JoinHandles.
        for task in released {
            let _ = task.await;
        }
        // Truly new candidate activity gets the next slot. Otherwise use oldest
        // visit first, not permanent candidate priority, so a busy inbox cannot
        // starve either a 33rd candidate or visible-page broad discovery.
        joined.sort_by_key(|room| {
            let id = room.room_id().as_str();
            let started_ms = self.rooms.get(id).map_or(0, |e| e.started_ms);
            (
                !is_new_candidate(id, started_ms),
                started_ms,
                !candidates.contains_key(id),
            )
        });
        let mut running = self
            .rooms
            .values()
            .filter(|entry| entry.task.is_some())
            .count();
        for room in &joined {
            let id = room.room_id().to_string();
            if !discovery_active && !candidates.contains_key(&id) {
                continue;
            }
            if running >= MAX_OBSERVERS {
                break;
            }
            let entry = self
                .rooms
                .entry(id.clone())
                .or_insert_with(|| RoomObserver {
                    state: Arc::new(std::sync::Mutex::new(RoomSnapshot {
                        loading: true,
                        ..Default::default()
                    })),
                    task: None,
                    started_ms: 0,
                });
            if entry.task.is_some() {
                continue;
            }
            // Failed bootstraps retry at a bounded cadence, without losing the
            // already observed request index while no timeline is retained.
            if entry.started_ms > 0 && now.saturating_sub(entry.started_ms) < DISCOVERY_BATCH_MS {
                continue;
            }
            self.room_monitor
                .as_ref()
                .unwrap()
                .register(room, &entry.state);
            entry.state.lock().unwrap().observing = true;
            let history = self.history.room(&id);
            let state = Arc::clone(&entry.state);
            let slots = Arc::clone(&self.bootstrap_slots);
            let discovery_until = Arc::clone(&self.discovery_until);
            let own_user = own_user.to_owned();
            let room = room.clone();
            let reusable = reusable.get(&id).cloned();
            let candidate_until = candidates.get(&id).copied().unwrap_or_default();
            entry.started_ms = now;
            struct Observing(Arc<std::sync::Mutex<RoomSnapshot>>);
            impl Drop for Observing {
                fn drop(&mut self) {
                    if let Ok(mut state) = self.0.lock() {
                        state.observing = false;
                    }
                }
            }
            // Capture the guard before spawning so even an unpolled, aborted
            // future releases observation ownership.
            let observing = Observing(state.clone());
            entry.task = Some(tokio::spawn(async move {
                let _observing = observing;
                let observer = observe_room(room, own_user, state, slots, reusable, history);
                tokio::pin!(observer);
                let expiry = async {
                    loop {
                        let until = discovery_until.load(Ordering::Relaxed).max(candidate_until);
                        if agent_approval_now_ms().unwrap_or_default() >= until {
                            break;
                        }
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                };
                // Drop queued waits and SDK timeline subscriptions when the
                // renderer vanishes. An SDK-owned in-flight pagination may
                // outlive its waiter; view protection quiesces that SDK status.
                tokio::select! { _ = &mut observer => {}, _ = expiry => {} }
            }));
            running += 1;
        }
        let mut snapshot = NativeAgentApprovalInboxSnapshot {
            session_generation: self.session_generation,
            items: Vec::new(),
            loading: false,
            incomplete: truncated_rooms,
            coverage_window_ms: AGENT_APPROVAL_TTL_MS,
            coverage: NativeAgentApprovalInboxCoverage::LatestEvent,
        };
        let mut checked_all = true;
        let mut deferred = false;
        for room in &joined {
            let id = room.room_id().as_str();
            let Some(entry) = self.rooms.get(id) else {
                checked_all = false;
                snapshot.loading |= discovery_active;
                continue;
            };
            let (items, loading, incomplete, can_send) = {
                let mut state = entry
                    .state
                    .lock()
                    .map_err(|_| "agent-approval-inbox-state-poisoned")?;
                state
                    .items
                    .retain(|item| now.saturating_sub(item.origin_server_ts) < HISTORY_MS);
                checked_all &= state.checked;
                deferred |= state.deferred;
                snapshot.loading |=
                    discovery_active && !state.checked && !state.deferred && !state.incomplete;
                (
                    state.items.clone(),
                    state.loading && !state.deferred,
                    state.incomplete,
                    state.can_send_reaction,
                )
            };
            snapshot.loading |=
                loading && entry.task.as_ref().is_some_and(|task| !task.is_finished());
            snapshot.incomplete |= incomplete;
            snapshot.items.extend(items.iter().filter_map(|item| {
                project_approval(id, item, own_user.as_str(), now, decisions).map(|mut item| {
                    item.can_send_reaction = can_send
                        && !item.body_truncated
                        && item.status == NativeAgentApprovalInboxStatus::Pending;
                    item
                })
            }));
        }
        snapshot.coverage =
            if !deferred && (discovery_active || (!joined.is_empty() && checked_all)) {
                NativeAgentApprovalInboxCoverage::Discovery
            } else {
                NativeAgentApprovalInboxCoverage::LatestEvent
            };
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
        // Honor the shared Core transport limit even for many multibyte or
        // escaped previews. Pending rows are already sorted first.
        let mut bytes = 512;
        let limit = crate::transport::MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
        let mut count = 0;
        for item in &snapshot.items {
            let size = serde_json::to_vec(item)
                .map_err(|_| "agent-approval-inbox-serialization-failed")?
                .len()
                + 1;
            if bytes + size > limit {
                break;
            }
            bytes += size;
            count += 1;
        }
        snapshot.incomplete |= count < snapshot.items.len();
        snapshot.items.truncate(count);
        Ok(snapshot)
    }
}

fn latest_candidate_until(room: &Room, own_user: &str, now: u64) -> u64 {
    use matrix_sdk::latest_events::LatestEventValue;
    let LatestEventValue::Remote(event) = room.latest_event() else {
        return 0;
    };
    let Ok(event) = serde_json::from_str::<serde_json::Value>(event.raw().json().get()) else {
        return 0;
    };
    let Some(body) = event.pointer("/content/body").and_then(|v| v.as_str()) else {
        return 0;
    };
    classify_agent_approval(
        body,
        event["sender"].as_str().unwrap_or_default(),
        own_user,
        event["origin_server_ts"].as_u64().unwrap_or_default(),
        now,
        [],
    )
    .ok()
    .filter(|c| !c.expired)
    .map_or(0, |c| c.expires_at)
}

fn pending_until(
    state: &RoomSnapshot,
    own_user: &str,
    now: u64,
    decisions: &HashSet<(String, String)>,
    room_id: &str,
) -> u64 {
    state
        .items
        .iter()
        .filter_map(|item| project_approval(room_id, item, own_user, now, decisions))
        .filter(|item| item.status == NativeAgentApprovalInboxStatus::Pending)
        .map(|item| item.expires_at)
        .max()
        .unwrap_or_default()
}

fn project_approval(
    room_id: &str,
    item: &NativeTimelineItem,
    own_user: &str,
    now: u64,
    decisions: &HashSet<(String, String)>,
) -> Option<NativeAgentApprovalInboxItem> {
    let classification = classify_agent_approval(
        &item.body,
        &item.sender,
        own_user,
        item.origin_server_ts,
        now,
        item.reactions
            .iter()
            .map(|reaction| (reaction.key.as_str(), reaction.me)),
    )
    .ok()?;
    if now.saturating_sub(item.origin_server_ts) >= HISTORY_MS || item.decryption_state.is_some() {
        return None;
    }
    let decided =
        decisions.contains(&(room_id.to_owned(), item.event_id.clone())) || classification.decided;
    let expires_at = classification.expires_at;
    let body: String = item.body.chars().take(PREVIEW_CHARS).collect();
    let body_truncated = body.len() != item.body.len();
    Some(NativeAgentApprovalInboxItem {
        room_id: room_id.to_owned(),
        event_id: item.event_id.clone(),
        sender: item.sender.clone(),
        body,
        body_truncated,
        can_send_reaction: false,
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

fn publish_observed(
    state: &std::sync::Mutex<RoomSnapshot>,
    items: impl Iterator<Item = NativeTimelineItem>,
    own_user: &str,
    covered: bool,
    deferred: bool,
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
        state.items = items;
        state.loading = false;
        state.incomplete = if deferred {
            state.incomplete || pending_decryption
        } else {
            !covered || pending_decryption
        };
        state.checked = !deferred;
        state.deferred = deferred;
    }
}

#[cfg(test)]
fn publish_room(
    state: &std::sync::Mutex<RoomSnapshot>,
    items: impl Iterator<Item = NativeTimelineItem>,
    own_user: &str,
    covered: bool,
) {
    publish_observed(state, items, own_user, covered, false);
}

async fn observe_room(
    room: Room,
    own_user: OwnedUserId,
    state: Arc<std::sync::Mutex<RoomSnapshot>>,
    slots: Arc<Semaphore>,
    reusable: Option<Weak<Timeline>>,
    history: Arc<RoomHistory>,
) {
    loop {
        if !observe_room_lease_with_timeline(
            &room,
            &own_user,
            &state,
            &slots,
            OBSERVER_BUDGET,
            reusable.as_ref().and_then(Weak::upgrade),
            history.clone(),
        )
        .await
        {
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

async fn observe_room_lease_with_timeline(
    room: &Room,
    own_user: &OwnedUserId,
    state: &std::sync::Mutex<RoomSnapshot>,
    slots: &Semaphore,
    budget: ObserverBudget,
    reusable: Option<Arc<Timeline>>,
    history: Arc<RoomHistory>,
) -> bool {
    let mut history_changes = history.subscribe();
    let Ok(permit) = slots.acquire().await else {
        return false;
    };
    let reusable = reusable.filter(|_| history.protected());
    let mut borrowed = reusable.is_some();
    let built = timeout(BOOTSTRAP_TIMEOUT, async {
        if let Some(timeline) = reusable {
            return Ok(timeline);
        }
        TimelineBuilder::new(room)
            .track_read_marker_and_receipts(TimelineReadReceiptTracking::Disabled)
            .build()
            .await
            .map(Arc::new)
    })
    .await;
    let mut timeline = match built {
        Ok(Ok(timeline)) => timeline,
        _ => {
            if let Ok(mut state) = state.lock() {
                state.loading = false;
                state.incomplete = true;
            }
            return false;
        }
    };
    if !borrowed {
        history
            .run(async {
                let _ = timeout(BOOTSTRAP_TIMEOUT, bootstrap_history(&timeline)).await;
            })
            .await;
    }
    let (mut items, mut updates) = timeline.subscribe().await;
    let initial_item_count = items.len();
    let covered = window_covered(items.iter(), agent_approval_now_ms().unwrap_or_default());
    publish_observed(
        state,
        items
            .iter()
            .filter_map(|item| project_item(item, Some(own_user))),
        own_user.as_str(),
        covered,
        !covered && (borrowed || history.protected()),
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
                    && !borrowed && !history.protected()
                    && !window_covered(items.iter(), agent_approval_now_ms().unwrap_or_default()) {
                    let timeline = Arc::clone(&timeline);
                    let history = Arc::clone(&history);
                    backfill = Some(Box::pin(async move {
                        let Ok(_permit) = slots.acquire().await else { return; };
                        history.run(async { let _ = timeout(BOOTSTRAP_TIMEOUT, bootstrap_history(&timeline)).await; }).await;
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
            _ = history_changes.changed() => {
                backfill = None;
                if !history.protected() {
                    if borrowed {
                        // The registry's view lease, not Arc counts, establishes
                        // release: cached registry Arcs may outlive the UI.
                        let Ok(Ok(owned)) = timeout(BOOTSTRAP_TIMEOUT, TimelineBuilder::new(room)
                            .track_read_marker_and_receipts(TimelineReadReceiptTracking::Disabled).build()).await else {
                            if let Ok(mut state) = state.lock() { state.incomplete = true; }
                            return false;
                        };
                        timeline = Arc::new(owned);
                        borrowed = false;
                        (items, updates) = timeline.subscribe().await;
                    }
                    retry.reset_immediately();
                }
            }
            _ = &mut lease_end => break,
        }
        // Recompute coverage after EVERY SDK diff. Clear/Reset, and the Remove
        // sequence used when local echoes survive a reset, invalidate the old
        // proof as soon as the boundary event/TimelineStart marker disappears.
        let covered = window_covered(items.iter(), agent_approval_now_ms().unwrap_or_default());
        if was_covered && !covered {
            // Losing a boundary requests a new check. Deliberate deferral
            // remains neutral; publish_observed preserves any real failure.
            retry.reset_immediately();
        }
        publish_observed(
            state,
            items
                .iter()
                .filter_map(|item| project_item(item, Some(own_user))),
            own_user.as_str(),
            covered,
            !covered && (borrowed || history.protected()),
        );
        if items.len() >= initial_item_count.saturating_add(budget.max_growth) {
            break;
        }
    }
    // A healthy planned rebuild is not evidence of a gap. Keep the last
    // projection while replacing the lease; the new subscribed window or an
    // actual stream error will determine coverage, not this timer.
    true
}

#[cfg(test)]
async fn observe_room_lease(
    room: &Room,
    own_user: &OwnedUserId,
    state: &std::sync::Mutex<RoomSnapshot>,
    slots: &Semaphore,
    budget: ObserverBudget,
) -> bool {
    observe_room_lease_with_timeline(
        room,
        own_user,
        state,
        slots,
        budget,
        None,
        ApprovalHistory::default().room(room.room_id().as_str()),
    )
    .await
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
            project_approval(
                ROOM,
                &item,
                USER,
                NOW,
                &decisions.completed.iter().cloned().collect()
            )
            .unwrap()
            .status,
            NativeAgentApprovalInboxStatus::Pending
        );
        item.reactions[0].me = true;
        assert_eq!(
            project_approval(
                ROOM,
                &item,
                USER,
                NOW,
                &decisions.completed.iter().cloned().collect()
            )
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
            project_approval(
                ROOM,
                &item,
                USER,
                NOW,
                &decisions.completed.iter().cloned().collect()
            )
            .unwrap()
            .status,
            NativeAgentApprovalInboxStatus::Expired
        );
        item.sender = USER.into();
        assert!(project_approval(
            ROOM,
            &item,
            USER,
            NOW,
            &decisions.completed.iter().cloned().collect()
        )
        .is_none());
        item = prompt();
        item.origin_server_ts = NOW + 60_001;
        assert!(project_approval(
            ROOM,
            &item,
            USER,
            NOW,
            &decisions.completed.iter().cloned().collect()
        )
        .is_none());
        item.origin_server_ts = 0;
        assert!(project_approval(
            ROOM,
            &item,
            USER,
            NOW,
            &decisions.completed.iter().cloned().collect()
        )
        .is_none());
    }
    #[test]
    fn decision_readback_wins_before_sync_and_redactions_remove_requests() {
        let mut item = prompt();
        let mut decisions = ApprovalDecisionRegistry::default();
        decisions.remember((ROOM.into(), item.event_id.clone()));
        assert_eq!(
            project_approval(
                ROOM,
                &item,
                USER,
                NOW,
                &decisions.completed.iter().cloned().collect()
            )
            .unwrap()
            .status,
            NativeAgentApprovalInboxStatus::Decided
        );
        item.body = "Message removed".into();
        assert!(project_approval(
            ROOM,
            &item,
            USER,
            NOW,
            &decisions.completed.iter().cloned().collect()
        )
        .is_none());
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
    async fn badge_is_cheap_and_visible_page_discovery_is_bounded_and_self_expiring() {
        use matrix_sdk::ruma::{OwnedRoomId, RoomVersionId};
        use matrix_sdk::test_utils::mocks::MatrixMockServer;
        use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, BOB};
        let server = MatrixMockServer::new().await;
        let client = server.client_builder().build().await;
        client.event_cache().subscribe().unwrap();
        let now = agent_approval_now_ms().unwrap();
        for index in 0..70 {
            let room_id: OwnedRoomId = format!("!cheap-{index:03}:example.org").parse().unwrap();
            let f = EventFactory::new().room(&room_id);
            let create = f.create(client.user_id().unwrap(), RoomVersionId::V11);
            let create = if index == 1 {
                create.with_space_type()
            } else {
                create
            };
            server
                .sync_room(
                    &client,
                    JoinedRoomBuilder::new(&room_id)
                        .add_state_event(create)
                        .add_timeline_event(
                            f.text_msg(if index < 2 {
                                "Approval required: dangerous command\nold or space"
                            } else {
                                "normal activity"
                            })
                            .sender(*BOB)
                            .server_ts(if index == 0 {
                                now - AGENT_APPROVAL_TTL_MS
                            } else {
                                now
                            }),
                        ),
                )
                .await;
        }
        let mut inbox = ApprovalInboxOwner::new(4);
        let before = server.server().received_requests().await.unwrap().len();
        let badge = inbox
            .snapshot(&client, &HashSet::new(), false, HashMap::new())
            .await
            .unwrap();
        assert!(badge.items.is_empty());
        assert!(!badge.incomplete);
        assert_eq!(
            badge.coverage,
            NativeAgentApprovalInboxCoverage::LatestEvent
        );
        assert!(!badge.loading);
        assert!(
            inbox.rooms.is_empty(),
            "idle polling must build zero ordinary-room timelines"
        );
        assert_eq!(
            server.server().received_requests().await.unwrap().len(),
            before
        );
        let page = inbox
            .snapshot(&client, &HashSet::new(), true, HashMap::new())
            .await
            .unwrap();
        assert!(!page.incomplete);
        assert!(page.loading);
        assert_eq!(page.coverage, NativeAgentApprovalInboxCoverage::Discovery);
        assert_eq!(
            inbox.rooms.values().filter(|e| e.task.is_some()).count(),
            MAX_OBSERVERS
        );
        assert!(
            !inbox.rooms.contains_key("!cheap-001:example.org"),
            "spaces never get observers"
        );
        // A new candidate must preempt an ordinary observer even if those
        // rooms are blocked waiting for history and have not proved coverage.
        let candidate_id: OwnedRoomId = "!cheap-069:example.org".parse().unwrap();
        let f = EventFactory::new().room(&candidate_id);
        server
            .sync_room(
                &client,
                JoinedRoomBuilder::new(&candidate_id).add_timeline_event(
                    f.text_msg("Approval required: dangerous command\npriority request")
                        .sender(*BOB)
                        .server_ts(now),
                ),
            )
            .await;
        inbox
            .snapshot(&client, &HashSet::new(), true, HashMap::new())
            .await
            .unwrap();
        assert!(inbox
            .rooms
            .get(candidate_id.as_str())
            .unwrap()
            .task
            .is_some());
        assert!(inbox.rooms.values().filter(|e| e.task.is_some()).count() <= MAX_OBSERVERS);
        // Simulate expiry of the renewed wall-clock lease. No subsequent list
        // request is made: queued and running tasks must cancel themselves.
        inbox.discovery_until.store(0, Ordering::Relaxed);
        timeout(Duration::from_secs(2), async {
            while inbox
                .rooms
                .iter()
                .filter(|(id, _)| id.as_str() != candidate_id.as_str())
                .any(|(_, entry)| entry.task.as_ref().is_some_and(|task| !task.is_finished()))
            {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("renderer disappearance must release every broad observer");
    }

    #[tokio::test]
    async fn candidate_observers_have_a_retention_cap_and_report_partial() {
        use matrix_sdk::ruma::{OwnedRoomId, RoomVersionId};
        use matrix_sdk::test_utils::mocks::MatrixMockServer;
        use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder, BOB};
        let server = MatrixMockServer::new().await;
        let client = server.client_builder().build().await;
        client.event_cache().subscribe().unwrap();
        let now = agent_approval_now_ms().unwrap();
        for index in 0..40 {
            let room_id: OwnedRoomId = format!("!candidate-{index:03}:example.org")
                .parse()
                .unwrap();
            let f = EventFactory::new().room(&room_id);
            server
                .sync_room(
                    &client,
                    JoinedRoomBuilder::new(&room_id)
                        .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                        .add_timeline_event(
                            f.text_msg("coverage boundary")
                                .sender(*BOB)
                                .server_ts(now - 360_000),
                        )
                        .add_timeline_event(
                            f.text_msg("Approval required: dangerous command\nnew request")
                                .event_id(
                                    &format!("$candidate-{index:03}")
                                        .parse::<matrix_sdk::ruma::OwnedEventId>()
                                        .unwrap(),
                                )
                                .sender(*BOB)
                                .server_ts(now),
                        ),
                )
                .await;
        }
        let mut inbox = ApprovalInboxOwner::new(5);
        let snapshot = inbox
            .snapshot(&client, &HashSet::new(), false, HashMap::new())
            .await
            .unwrap();
        assert!(!snapshot.incomplete);
        assert_eq!(
            inbox.rooms.values().filter(|e| e.task.is_some()).count(),
            MAX_OBSERVERS
        );
        // More than 32 candidates must all get turns while retaining no more
        // than 32 SDK observers. Their shared index remains visible between turns.
        timeout(Duration::from_secs(12), async {
            loop {
                let snapshot = inbox
                    .snapshot(&client, &HashSet::new(), false, HashMap::new())
                    .await
                    .unwrap();
                assert!(
                    inbox
                        .rooms
                        .values()
                        .filter(|entry| entry.task.is_some())
                        .count()
                        <= MAX_OBSERVERS
                );
                let ids: HashSet<_> = snapshot
                    .items
                    .iter()
                    .map(|item| item.event_id.as_str())
                    .collect();
                if (0..40).all(|i| ids.contains(format!("$candidate-{i:03}").as_str())) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("every candidate must be discovered well before its five-minute expiry");
        // Ordinary latest activity can hide a pending prompt. Visible-page
        // discovery must visit it even while all 40 known candidates are live.
        let hidden_id: OwnedRoomId = "!hidden-prompt:example.org".parse().unwrap();
        let f = EventFactory::new().room(&hidden_id);
        server
            .sync_room(
                &client,
                JoinedRoomBuilder::new(&hidden_id)
                    .add_state_event(f.create(client.user_id().unwrap(), RoomVersionId::V11))
                    .add_timeline_event(
                        f.text_msg("coverage boundary")
                            .sender(*BOB)
                            .server_ts(now - 360_000),
                    )
                    .add_timeline_event(
                        f.text_msg("Approval required: dangerous command\nhidden request")
                            .sender(*BOB)
                            .event_id(matrix_sdk::ruma::event_id!("$hidden-prompt"))
                            .server_ts(now),
                    )
                    .add_timeline_event(
                        f.text_msg("newer ordinary conversation")
                            .sender(*BOB)
                            .server_ts(now + 1),
                    ),
            )
            .await;
        timeout(Duration::from_secs(12), async {
            loop {
                let snapshot = inbox
                    .snapshot(&client, &HashSet::new(), true, HashMap::new())
                    .await
                    .unwrap();
                assert!(!snapshot.incomplete);
                assert!(
                    inbox
                        .rooms
                        .values()
                        .filter(|entry| entry.task.is_some())
                        .count()
                        <= MAX_OBSERVERS
                );
                if snapshot
                    .items
                    .iter()
                    .any(|item| item.event_id == "$hidden-prompt")
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("known candidates must not starve visible-page discovery of other rooms");
    }

    #[test]
    fn preview_is_unicode_bounded_and_truncation_never_grants_actions() {
        let mut item = prompt();
        item.body.push_str(&"é".repeat(PREVIEW_CHARS));
        let preview = project_approval(ROOM, &item, USER, NOW, &HashSet::new()).unwrap();
        assert_eq!(preview.body.chars().count(), PREVIEW_CHARS);
        assert!(preview.body_truncated);
        assert!(!preview.can_send_reaction);
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
                task: Some(task),
                started_ms: 0,
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
            !state.lock().unwrap().incomplete,
            "a healthy lease rotation must not manufacture partial coverage"
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
            project_approval(ROOM, &item, USER, NOW, &HashSet::new())
                .unwrap()
                .status,
            NativeAgentApprovalInboxStatus::Pending
        );
        assert!(ApprovalInboxOwner::new(2).rooms.is_empty());
    }
}

#[cfg(test)]
mod ownership_tests;
