//! Timeline view-delta delivery over `tauri::ipc::Channel` (L20).
//!
//! Each mounted timeline view opens one channel for its room through
//! `desktop_timeline_subscribe` before it opens a stream, and passes the
//! returned subscription id to `matrix_timeline_open` / `_jump_latest`, which
//! claim the opened stream for it. Batches of a claimed stream go to that
//! view only; a stream nobody has claimed yet (Core subscribes before the
//! open readback returns) goes to every view of its room, whose open buffer
//! keeps it. A batch is serialized once, and only when some view wants it.
//!
//! Subscriptions bind the session generation that was active when they
//! opened: a batch from an older session is never delivered, and one from a
//! newer session retires them. A channel whose send fails is dropped at
//! once; a renderer reload drops every channel of that webview.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::Mutex;

use synara_core::app::timeline::TimelineViewDeltaBatch;
use tauri::ipc::{Channel, InvokeResponseBody};

/// Streams one view keeps claimed. Each open, jump and last-read restore
/// claims one; past the cap the oldest goes back to room-wide delivery,
/// which costs duplicate sends but never loses a batch.
const MAX_CLAIMED_STREAMS_PER_VIEW: usize = 16;

struct Subscription {
    webview_label: String,
    room_id: String,
    session_generation: u64,
    channel: Channel<InvokeResponseBody>,
    claimed: VecDeque<String>,
}

#[derive(Default)]
struct ChannelsState {
    next_id: u64,
    subscriptions: BTreeMap<u64, Subscription>,
    stream_owners: HashMap<String, u64>,
}

impl ChannelsState {
    fn remove(&mut self, id: u64) -> Option<Subscription> {
        let subscription = self.subscriptions.remove(&id)?;
        for stream_id in &subscription.claimed {
            if self.stream_owners.get(stream_id) == Some(&id) {
                self.stream_owners.remove(stream_id);
            }
        }
        Some(subscription)
    }
}

/// Desktop-owned registry of open timeline view channels.
#[derive(Default)]
pub struct TimelineViewChannels {
    state: Mutex<ChannelsState>,
}

impl TimelineViewChannels {
    fn lock(&self) -> std::sync::MutexGuard<'_, ChannelsState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Register a view's channel; returns its subscription id.
    pub fn subscribe(
        &self,
        webview_label: &str,
        room_id: String,
        session_generation: u64,
        channel: Channel<InvokeResponseBody>,
    ) -> u64 {
        let mut state = self.lock();
        state.next_id = state.next_id.saturating_add(1);
        let id = state.next_id;
        state.subscriptions.insert(
            id,
            Subscription {
                webview_label: webview_label.to_owned(),
                room_id,
                session_generation,
                channel,
                claimed: VecDeque::new(),
            },
        );
        id
    }

    /// Drop a view's channel (unmount). Only the webview that opened it may.
    pub fn unsubscribe(&self, webview_label: &str, id: u64) -> bool {
        let removed = {
            let mut state = self.lock();
            match state.subscriptions.get(&id) {
                Some(subscription) if subscription.webview_label == webview_label => {
                    state.remove(id)
                }
                _ => None,
            }
        };
        removed.is_some()
    }

    /// Drop every channel of a webview whose page is (re)loading: its views
    /// are gone without unmounting.
    pub fn close_webview(&self, webview_label: &str) -> usize {
        let removed: Vec<Subscription> = {
            let mut state = self.lock();
            let ids: Vec<u64> = state
                .subscriptions
                .iter()
                .filter(|(_, subscription)| subscription.webview_label == webview_label)
                .map(|(id, _)| *id)
                .collect();
            ids.into_iter().filter_map(|id| state.remove(id)).collect()
        };
        removed.len()
    }

    /// Route `stream_id`'s batches to subscription `id` only. A missing id
    /// (the view has no channel) leaves the stream on room-wide delivery.
    pub fn claim_stream(&self, id: Option<u64>, stream_id: &str) {
        let Some(id) = id else { return };
        let mut state = self.lock();
        if !state.subscriptions.contains_key(&id) {
            return;
        }
        if let Some(previous) = state.stream_owners.insert(stream_id.to_owned(), id) {
            if previous == id {
                return;
            }
            if let Some(owner) = state.subscriptions.get_mut(&previous) {
                owner.claimed.retain(|claimed| claimed != stream_id);
            }
        }
        let Some(subscription) = state.subscriptions.get_mut(&id) else {
            return;
        };
        subscription.claimed.push_back(stream_id.to_owned());
        let evicted = (subscription.claimed.len() > MAX_CLAIMED_STREAMS_PER_VIEW)
            .then(|| subscription.claimed.pop_front())
            .flatten();
        if let Some(evicted) = evicted {
            if state.stream_owners.get(&evicted) == Some(&id) {
                state.stream_owners.remove(&evicted);
            }
        }
    }

    /// Forget a closed stream's owner.
    pub fn release_stream(&self, stream_id: &str) {
        let mut state = self.lock();
        let Some(id) = state.stream_owners.remove(stream_id) else {
            return;
        };
        if let Some(subscription) = state.subscriptions.get_mut(&id) {
            subscription.claimed.retain(|claimed| claimed != stream_id);
        }
    }

    /// Send one batch to the views that want it; returns how many channels
    /// it reached. Sends happen under the registry lock so batches keep the
    /// order Core emitted them in (the channel's own index keeps that order
    /// in the webview).
    pub fn deliver(&self, batch: &TimelineViewDeltaBatch) -> usize {
        let mut retired = Vec::new();
        let mut state = self.lock();
        let stale: Vec<u64> = state
            .subscriptions
            .iter()
            .filter(|(_, subscription)| subscription.session_generation < batch.session_generation)
            .map(|(id, _)| *id)
            .collect();
        retired.extend(stale.into_iter().filter_map(|id| state.remove(id)));

        let wants = |subscription: &Subscription| {
            subscription.room_id == batch.room_id
                && subscription.session_generation == batch.session_generation
        };
        let targets: Vec<u64> = match state.stream_owners.get(&batch.stream_id) {
            Some(owner) => state
                .subscriptions
                .get(owner)
                .filter(|subscription| wants(subscription))
                .map(|_| vec![*owner])
                .unwrap_or_default(),
            None => state
                .subscriptions
                .iter()
                .filter(|(_, subscription)| wants(subscription))
                .map(|(id, _)| *id)
                .collect(),
        };
        if targets.is_empty() {
            return 0;
        }
        let Ok(json) = serde_json::to_string(batch) else {
            return 0;
        };
        let mut delivered = 0;
        for id in targets {
            let sent = state.subscriptions.get(&id).is_some_and(|subscription| {
                subscription
                    .channel
                    .send(InvokeResponseBody::Json(json.clone()))
                    .is_ok()
            });
            if sent {
                delivered += 1;
            } else {
                // The webview is gone: drop the channel instead of failing
                // on every later batch.
                retired.extend(state.remove(id));
            }
        }
        drop(state);
        drop(retired);
        delivered
    }

    #[cfg(test)]
    fn subscription_count(&self) -> usize {
        self.lock().subscriptions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use synara_core::app::timeline::{TimelineViewDeltaOp, TIMELINE_VIEW_SCHEMA_VERSION};

    const ROOM: &str = "!room:example.org";
    const OTHER_ROOM: &str = "!other:example.org";

    fn recording_channel() -> (Channel<InvokeResponseBody>, Arc<Mutex<Vec<String>>>) {
        let received = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&received);
        let channel = Channel::new(move |body| {
            if let InvokeResponseBody::Json(json) = body {
                sink.lock().expect("lock").push(json);
            }
            Ok(())
        });
        (channel, received)
    }

    fn failing_channel() -> (Channel<InvokeResponseBody>, Arc<AtomicUsize>) {
        let attempts = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&attempts);
        let channel = Channel::new(move |_| {
            counter.fetch_add(1, Ordering::SeqCst);
            Err(tauri::Error::WebviewNotFound)
        });
        (channel, attempts)
    }

    fn batch(
        room_id: &str,
        stream_id: &str,
        generation: u64,
        revision: u64,
    ) -> TimelineViewDeltaBatch {
        TimelineViewDeltaBatch {
            schema_version: TIMELINE_VIEW_SCHEMA_VERSION,
            session_generation: generation,
            stream_id: stream_id.into(),
            room_id: room_id.into(),
            revision,
            ops: vec![TimelineViewDeltaOp::Clear],
            read_state: None,
            pagination: None,
            pinned_event_ids: None,
        }
    }

    fn revisions(received: &Mutex<Vec<String>>) -> Vec<u64> {
        received
            .lock()
            .expect("lock")
            .iter()
            .map(|json| {
                serde_json::from_str::<serde_json::Value>(json).expect("json")["revision"]
                    .as_u64()
                    .expect("revision")
            })
            .collect()
    }

    #[test]
    fn sends_each_batch_in_order_to_the_rooms_views_only() {
        let channels = TimelineViewChannels::default();
        let (channel, received) = recording_channel();
        let (other_channel, other_received) = recording_channel();
        channels.subscribe("main", ROOM.into(), 3, channel);
        channels.subscribe("main", OTHER_ROOM.into(), 3, other_channel);

        for revision in 1..=3 {
            assert_eq!(channels.deliver(&batch(ROOM, "live:1", 3, revision)), 1);
        }

        assert_eq!(revisions(&received), [1, 2, 3]);
        assert!(other_received.lock().expect("lock").is_empty());
        let first: serde_json::Value =
            serde_json::from_str(&received.lock().expect("lock")[0]).expect("json");
        assert_eq!(first["streamId"], "live:1");
        assert_eq!(first["sessionGeneration"], 3);
    }

    #[test]
    fn a_claimed_stream_reaches_its_view_alone() {
        let channels = TimelineViewChannels::default();
        let (room_channel, room_received) = recording_channel();
        let (thread_channel, thread_received) = recording_channel();
        let room_view = channels.subscribe("main", ROOM.into(), 1, room_channel);
        let thread_view = channels.subscribe("main", ROOM.into(), 1, thread_channel);

        // Before the open readback claims it, both views buffer the stream.
        assert_eq!(channels.deliver(&batch(ROOM, "thread:1", 1, 1)), 2);
        channels.claim_stream(Some(thread_view), "thread:1");
        channels.claim_stream(Some(room_view), "live:2");
        assert_eq!(channels.deliver(&batch(ROOM, "thread:1", 1, 2)), 1);
        assert_eq!(channels.deliver(&batch(ROOM, "live:2", 1, 1)), 1);
        assert_eq!(revisions(&room_received), [1, 1]);
        assert_eq!(revisions(&thread_received), [1, 2]);

        // A closed stream goes back to room-wide delivery.
        channels.release_stream("thread:1");
        assert_eq!(channels.deliver(&batch(ROOM, "thread:1", 1, 3)), 2);
        // Without a channel, an open claims nothing.
        channels.claim_stream(None, "live:3");
        assert_eq!(channels.deliver(&batch(ROOM, "live:3", 1, 1)), 2);
    }

    #[test]
    fn an_unsubscribed_or_reloaded_view_receives_nothing_and_frees_its_claims() {
        let channels = TimelineViewChannels::default();
        let (channel, received) = recording_channel();
        let (kept_channel, kept_received) = recording_channel();
        let id = channels.subscribe("main", ROOM.into(), 1, channel);
        channels.claim_stream(Some(id), "live:1");
        let kept = channels.subscribe("main", ROOM.into(), 1, kept_channel);

        // Another webview cannot close it.
        assert!(!channels.unsubscribe("widget-1", id));
        assert!(channels.unsubscribe("main", id));
        assert!(!channels.unsubscribe("main", id));
        assert_eq!(channels.deliver(&batch(ROOM, "live:1", 1, 1)), 1);
        assert!(received.lock().expect("lock").is_empty());
        assert_eq!(revisions(&kept_received), [1]);

        channels.subscribe("other", ROOM.into(), 1, recording_channel().0);
        assert_eq!(channels.close_webview("main"), 1);
        assert_eq!(channels.subscription_count(), 1);
        channels.claim_stream(Some(kept), "live:4");
        assert_eq!(channels.deliver(&batch(ROOM, "live:4", 1, 1)), 1);
        assert_eq!(revisions(&kept_received), [1]);
    }

    #[test]
    fn a_closed_channel_is_dropped_on_its_first_failed_send() {
        let channels = TimelineViewChannels::default();
        let (dead, attempts) = failing_channel();
        let (live, received) = recording_channel();
        let dead_id = channels.subscribe("main", ROOM.into(), 1, dead);
        channels.claim_stream(Some(dead_id), "live:1");
        channels.subscribe("main", ROOM.into(), 1, live);

        assert_eq!(channels.deliver(&batch(ROOM, "live:1", 1, 1)), 0);
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        assert_eq!(channels.subscription_count(), 1);
        // The dead view's claim went with it: the stream reaches the room.
        for revision in 2..=4 {
            assert_eq!(channels.deliver(&batch(ROOM, "live:1", 1, revision)), 1);
        }
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        assert_eq!(revisions(&received), [2, 3, 4]);
    }

    #[test]
    fn a_session_generation_mismatch_is_never_delivered() {
        let channels = TimelineViewChannels::default();
        let (channel, received) = recording_channel();
        channels.subscribe("main", ROOM.into(), 2, channel);

        // A late batch from the previous session is skipped.
        assert_eq!(channels.deliver(&batch(ROOM, "live:1", 1, 9)), 0);
        assert_eq!(channels.subscription_count(), 1);
        assert_eq!(channels.deliver(&batch(ROOM, "live:1", 2, 1)), 1);
        // A newer session retires the subscription: its streams are gone.
        assert_eq!(channels.deliver(&batch(ROOM, "live:1", 3, 1)), 0);
        assert_eq!(channels.subscription_count(), 0);
        assert_eq!(revisions(&received), [1]);
    }

    #[test]
    fn claims_past_the_cap_fall_back_to_room_delivery() {
        let channels = TimelineViewChannels::default();
        let (channel, _) = recording_channel();
        let (other, _) = recording_channel();
        let id = channels.subscribe("main", ROOM.into(), 1, channel);
        channels.subscribe("main", ROOM.into(), 1, other);
        for stream in 0..=MAX_CLAIMED_STREAMS_PER_VIEW {
            channels.claim_stream(Some(id), &format!("focused:{stream}"));
        }
        assert_eq!(channels.deliver(&batch(ROOM, "focused:0", 1, 1)), 2);
        assert_eq!(channels.deliver(&batch(ROOM, "focused:1", 1, 1)), 1);
    }

    #[test]
    fn a_batch_nobody_wants_is_not_serialized() {
        let channels = TimelineViewChannels::default();
        assert_eq!(channels.deliver(&batch(ROOM, "live:1", 1, 1)), 0);
    }
}
