//! Reply previews whose parent is outside a view stream's loaded window.
//!
//! The SDK fills a reply's `in_reply_to` only from items the timeline already
//! holds and leaves it `Unavailable` otherwise, so a reply to an older message
//! shows a placeholder until something asks for the parent.
//! `Timeline::fetch_details_for_event` loads it from the event cache or one
//! `/event` request and replaces the reply item, which reaches the stream as
//! an ordinary `Set` diff. Each stream asks once per reply, a few at a time,
//! with a lifetime bound.

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use eyeball_im::VectorDiff;
use matrix_sdk::ruma::OwnedEventId;
use matrix_sdk_ui::timeline::{Timeline, TimelineDetails, TimelineItem as SdkTimelineItem};
use tokio::task::JoinSet;

/// Replies whose parent one stream fetches over its lifetime.
pub(super) const MAX_REPLY_DETAIL_FETCHES: usize = 128;
/// Parent fetches one stream runs at a time.
const REPLY_DETAIL_FETCH_CONCURRENCY: usize = 4;

/// The reply-parent fetches of one view stream.
///
/// Each fetch runs as its own task. A fetch takes the SDK timeline's fair
/// state write lock; held as a future inside the stream's update task, it
/// would be polled only when the task's `select!` picks it, so once the lock
/// was handed to it, any other branch awaiting a timeline read would wait on
/// a writer nobody polls and the stream would stop for good.
///
/// A fetch is never cancelled part-way, by a deadline or by closing the
/// stream: the SDK marks the reply `Pending` before its request and treats a
/// `Pending` reply as already being fetched, so a cancelled fetch would leave
/// the shared timeline's placeholder in place for good. Each fetch ends with
/// the SDK's own request timeout; closing the stream detaches the running
/// ones and drops the queue.
pub(super) struct ReplyDetailFetches {
    timeline: Arc<Timeline>,
    requested: HashSet<OwnedEventId>,
    queued: VecDeque<OwnedEventId>,
    in_flight: JoinSet<()>,
}

impl ReplyDetailFetches {
    pub(super) fn new(timeline: Arc<Timeline>) -> Self {
        Self {
            timeline,
            requested: HashSet::new(),
            queued: VecDeque::new(),
            in_flight: JoinSet::new(),
        }
    }

    /// Queue the replies among `items` whose parent is unavailable.
    pub(super) fn request_items<'a>(
        &mut self,
        items: impl IntoIterator<Item = &'a Arc<SdkTimelineItem>>,
    ) {
        for item in items {
            let Some(reply_id) = unresolved_reply(item) else {
                continue;
            };
            if self.requested.len() >= MAX_REPLY_DETAIL_FETCHES {
                break;
            }
            if self.requested.insert(reply_id.clone()) {
                self.queued.push_back(reply_id);
            }
        }
        self.start_queued();
    }

    /// Queue the replies a batch of SDK diffs inserted or replaced.
    pub(super) fn request_diffs(&mut self, diffs: &[VectorDiff<Arc<SdkTimelineItem>>]) {
        let mut items: Vec<&Arc<SdkTimelineItem>> = Vec::new();
        for diff in diffs {
            match diff {
                VectorDiff::Append { values } | VectorDiff::Reset { values } => {
                    items.extend(values.iter());
                }
                VectorDiff::PushFront { value }
                | VectorDiff::PushBack { value }
                | VectorDiff::Insert { value, .. }
                | VectorDiff::Set { value, .. } => items.push(value),
                VectorDiff::Clear
                | VectorDiff::PopFront
                | VectorDiff::PopBack
                | VectorDiff::Remove { .. }
                | VectorDiff::Truncate { .. } => {}
            }
        }
        self.request_items(items);
    }

    pub(super) fn is_busy(&self) -> bool {
        !self.in_flight.is_empty()
    }

    /// Wait for one fetch task to end, then start the next queued one. A
    /// task that panicked or was cancelled counts as ended.
    pub(super) async fn next_done(&mut self) -> Option<()> {
        let done = self.in_flight.join_next().await.map(|_| ());
        self.start_queued();
        done
    }

    fn start_queued(&mut self) {
        while self.in_flight.len() < REPLY_DETAIL_FETCH_CONCURRENCY {
            let Some(reply_id) = self.queued.pop_front() else {
                return;
            };
            let timeline = self.timeline.clone();
            self.in_flight.spawn(async move {
                // A failed fetch leaves the SDK's `Error` details: the row
                // keeps its placeholder, which still jumps to the original.
                let _ = timeline.fetch_details_for_event(&reply_id).await;
            });
        }
    }

    #[cfg(test)]
    pub(super) fn in_flight(&self) -> usize {
        self.in_flight.len()
    }
}

impl Drop for ReplyDetailFetches {
    fn drop(&mut self) {
        // See the type's docs: running fetches finish on their own.
        self.in_flight.detach_all();
    }
}

/// The event id of a remote reply whose parent the timeline does not hold
/// and nobody has fetched yet.
fn unresolved_reply(item: &SdkTimelineItem) -> Option<OwnedEventId> {
    let event = item.as_event()?;
    if event.is_local_echo() {
        return None;
    }
    let reply = event.content().as_msglike()?.in_reply_to.as_ref()?;
    matches!(reply.event, TimelineDetails::Unavailable)
        .then(|| event.event_id().map(ToOwned::to_owned))
        .flatten()
}
