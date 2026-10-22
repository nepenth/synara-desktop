//! What one view stream has published: its rows, metadata and revision.
//!
//! The stream's update task is the only writer of rows. Every change is
//! applied and emitted under one lock, and snapshots read under that same
//! lock, so a snapshot's rows always match its revision: the next delta
//! applies to exactly those rows, never twice and never across a gap.
//! Snapshots copy the published rows instead of re-projecting the SDK
//! window, so a refresh costs a clone, not a projection of every row.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

use super::super::{
    TimelinePageState, TimelinePaginationState, TimelineReadState, TimelineRoomActionAuthority,
    TimelineViewCapabilities, TimelineViewDeltaOp, TimelineViewPosition, TimelineViewRow,
    TimelineViewSnapshot, ViewDeltaEmitter, TIMELINE_VIEW_SCHEMA_VERSION,
};

/// Bound on waiting for the update task to publish the SDK diffs already
/// queued for it, so a page's readback includes the rows it loaded.
const SETTLE_TIMEOUT: Duration = Duration::from_secs(1);

/// The SDK provider a stream reads. It decides forward pagination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ViewFocus {
    /// The room's live timeline: it is always at the newest event.
    Live,
    /// An event-focused window, paged forward toward the present.
    Event,
    /// A thread. The SDK cannot page a thread forward.
    Thread,
}

impl ViewFocus {
    pub(super) fn initial_forward(self) -> TimelinePageState {
        match self {
            ViewFocus::Live => TimelinePageState::Exhausted,
            ViewFocus::Event => TimelinePageState::Available,
            ViewFocus::Thread => TimelinePageState::Unavailable,
        }
    }

    pub(super) fn can_paginate_forward(self) -> bool {
        self != ViewFocus::Thread
    }
}

/// One change to publish. Metadata left `None` is unchanged.
#[derive(Default)]
pub(super) struct ViewChange {
    pub(super) ops: Vec<TimelineViewDeltaOp>,
    pub(super) read_state: Option<TimelineReadState>,
    /// The position `read_state` was projected for. A read state projected
    /// for a position the stream has since left is dropped.
    pub(super) read_state_position: Option<TimelineViewPosition>,
    pub(super) backward: Option<TimelinePageState>,
    pub(super) forward: Option<TimelinePageState>,
    /// The SDK cleared or reset a live window, which reopens history this
    /// stream may have reached: an exhausted backward state becomes available.
    pub(super) unlatch_backward: bool,
    pub(super) pinned_event_ids: Option<Vec<String>>,
    pub(super) action_authority: Option<TimelineRoomActionAuthority>,
}

/// Initial published state of a newly opened stream, at revision 0.
pub(super) struct PublishedViewInit {
    pub(super) session_generation: u64,
    pub(super) room_id: String,
    pub(super) focus: ViewFocus,
    pub(super) emitter: ViewDeltaEmitter,
    pub(super) unclaimed_ttl: Option<Duration>,
    pub(super) rows: Vec<TimelineViewRow>,
    pub(super) position: TimelineViewPosition,
    pub(super) pagination: TimelinePaginationState,
    pub(super) read_state: TimelineReadState,
    pub(super) pinned_event_ids: Vec<String>,
    pub(super) action_authority: TimelineRoomActionAuthority,
}

struct State {
    rows: Vec<TimelineViewRow>,
    position: TimelineViewPosition,
    pagination: TimelinePaginationState,
    read_state: TimelineReadState,
    pinned_event_ids: Vec<String>,
    action_authority: TimelineRoomActionAuthority,
    /// When an emitted batch was first left unread by the stream's consumer.
    unclaimed_since: Option<Instant>,
    expired: bool,
}

/// A barrier request: answered once the update task has published every
/// SDK diff that was queued for it.
pub(super) type SettleRequest = oneshot::Sender<()>;

pub(super) struct PublishedView {
    session_generation: u64,
    room_id: String,
    focus: ViewFocus,
    emitter: ViewDeltaEmitter,
    /// See [`PublishedView::claim`]. `None` keeps streams until closed.
    unclaimed_ttl: Option<Duration>,
    settle: mpsc::UnboundedSender<SettleRequest>,
    state: Mutex<State>,
}

impl PublishedView {
    /// The view and the receiver its update task answers settle requests on.
    pub(super) fn new(init: PublishedViewInit) -> (Self, mpsc::UnboundedReceiver<SettleRequest>) {
        let (settle, settle_requests) = mpsc::unbounded_channel();
        let view = Self {
            session_generation: init.session_generation,
            room_id: init.room_id,
            focus: init.focus,
            emitter: init.emitter,
            unclaimed_ttl: init.unclaimed_ttl,
            settle,
            state: Mutex::new(State {
                rows: init.rows,
                position: init.position,
                pagination: init.pagination,
                read_state: init.read_state,
                pinned_event_ids: init.pinned_event_ids,
                action_authority: init.action_authority,
                unclaimed_since: None,
                expired: false,
            }),
        };
        (view, settle_requests)
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(super) fn focus(&self) -> ViewFocus {
        self.focus
    }

    pub(super) fn position(&self) -> TimelineViewPosition {
        self.lock().position.clone()
    }

    pub(super) fn set_position(&self, position: TimelineViewPosition) {
        self.lock().position = position;
    }

    pub(super) fn pagination(&self) -> TimelinePaginationState {
        self.lock().pagination.clone()
    }

    pub(super) fn read_state(&self) -> TimelineReadState {
        self.lock().read_state.clone()
    }

    pub(super) fn pinned_event_ids(&self) -> Vec<String> {
        self.lock().pinned_event_ids.clone()
    }

    pub(super) fn action_authority(&self) -> TimelineRoomActionAuthority {
        self.lock().action_authority
    }

    /// The published rows and metadata, unless the stream is still at
    /// `known_revision`: a presenter that applied every batch up to it has
    /// them already, so the rows are not copied again.
    pub(super) fn snapshot_after(&self, known_revision: u64) -> Option<TimelineViewSnapshot> {
        (self.emitter.revision() != known_revision).then(|| self.snapshot())
    }

    /// The published rows and metadata at one revision.
    pub(super) fn snapshot(&self) -> TimelineViewSnapshot {
        let state = self.lock();
        TimelineViewSnapshot {
            schema_version: TIMELINE_VIEW_SCHEMA_VERSION,
            session_generation: self.session_generation,
            room_id: self.room_id.clone(),
            revision: self.emitter.revision(),
            position: state.position.clone(),
            pagination: state.pagination.clone(),
            read_state: state.read_state.clone(),
            pinned_event_ids: state.pinned_event_ids.clone(),
            rows: state.rows.clone(),
            capabilities: TimelineViewCapabilities {
                mark_read: true,
                mark_unread: true,
                paginate_backward: true,
                paginate_forward: self.focus.can_paginate_forward(),
                can_redact_own: state.action_authority.can_redact_own,
                can_redact_other: state.action_authority.can_redact_other,
            },
        }
    }

    /// Drop `Set` ops that would replace a row with an identical one. The SDK
    /// replaces an item whenever any member's read receipt on it moves, and
    /// rows carry no receipts, so those replacements change nothing. Only
    /// sets before the batch's first structural op are compared: later
    /// indices refer to rows this batch has not published yet.
    ///
    /// Must be called by the update task, the only writer of rows.
    pub(super) fn without_unchanged_sets(
        &self,
        ops: Vec<TimelineViewDeltaOp>,
    ) -> Vec<TimelineViewDeltaOp> {
        let state = self.lock();
        let mut structural = false;
        let mut replaced: HashMap<usize, usize> = HashMap::new();
        let mut kept = Vec::with_capacity(ops.len());
        for op in ops {
            match &op {
                TimelineViewDeltaOp::Set { index, row } if !structural => {
                    let current = match replaced.get(index) {
                        Some(&kept_index) => match &kept[kept_index] {
                            TimelineViewDeltaOp::Set { row, .. } => Some(row),
                            _ => None,
                        },
                        None => state.rows.get(*index),
                    };
                    if current == Some(row) {
                        continue;
                    }
                    replaced.insert(*index, kept.len());
                }
                TimelineViewDeltaOp::Set { .. } => {}
                _ => structural = true,
            }
            kept.push(op);
        }
        kept
    }

    /// Apply and emit one change. Returns whether a batch was emitted.
    pub(super) fn publish(&self, change: ViewChange) -> bool {
        let ViewChange {
            ops,
            read_state,
            read_state_position,
            backward,
            forward,
            unlatch_backward,
            pinned_event_ids,
            action_authority,
        } = change;
        let mut state = self.lock();
        apply_view_ops(&mut state.rows, &ops);
        let read_state = read_state.filter(|_| {
            read_state_position
                .as_ref()
                .is_none_or(|position| *position == state.position)
        });
        if let Some(read_state) = &read_state {
            state.read_state = read_state.clone();
        }
        if let Some(pinned_event_ids) = &pinned_event_ids {
            state.pinned_event_ids = pinned_event_ids.clone();
        }
        if let Some(action_authority) = action_authority {
            state.action_authority = action_authority;
        }
        let mut pagination = state.pagination.clone();
        if let Some(backward) = backward {
            pagination.backward = backward;
        }
        if unlatch_backward && pagination.backward == TimelinePageState::Exhausted {
            pagination.backward = TimelinePageState::Available;
        }
        if let Some(forward) = forward {
            pagination.forward = forward;
        }
        let pagination = (pagination != state.pagination).then(|| {
            state.pagination = pagination.clone();
            pagination
        });
        let emitted = self
            .emitter
            .emit(ops, read_state, pagination, pinned_event_ids);
        if emitted {
            self.note_unclaimed(&mut state);
        }
        emitted
    }

    fn note_unclaimed(&self, state: &mut State) {
        let Some(ttl) = self.unclaimed_ttl else {
            return;
        };
        let now = Instant::now();
        match state.unclaimed_since {
            None => state.unclaimed_since = Some(now),
            Some(since) if now.saturating_duration_since(since) >= ttl => state.expired = true,
            Some(_) => {}
        }
    }

    /// The consumer read this stream (snapshot, page, read state or follow).
    ///
    /// A consumer that stops reading a stream it never closed (a room view
    /// that went away) leaves it running the per-update pipeline for nobody.
    /// With a TTL, a stream whose batches stay unread that long expires: its
    /// update task stops, and the registry closes it on its next open.
    pub(super) fn claim(&self) {
        self.lock().unclaimed_since = None;
    }

    pub(super) fn is_expired(&self) -> bool {
        self.lock().expired
    }

    /// Wait (bounded) until the update task has published every SDK diff
    /// already queued for it. Returns at once when the task has ended.
    pub(super) async fn settled(&self) {
        let (done, wait) = oneshot::channel();
        if self.settle.send(done).is_err() {
            return;
        }
        let _ = timeout(SETTLE_TIMEOUT, wait).await;
    }
}

/// Apply delta ops to a row list the way a presenter does. Out-of-range ops
/// are ignored, as in the SDK-item mirror.
pub(super) fn apply_view_ops(rows: &mut Vec<TimelineViewRow>, ops: &[TimelineViewDeltaOp]) {
    for op in ops {
        match op {
            TimelineViewDeltaOp::Append { rows: values } => rows.extend(values.iter().cloned()),
            TimelineViewDeltaOp::Clear => rows.clear(),
            TimelineViewDeltaOp::PushFront { row } => rows.insert(0, row.clone()),
            TimelineViewDeltaOp::PushBack { row } => rows.push(row.clone()),
            TimelineViewDeltaOp::PopFront => {
                if !rows.is_empty() {
                    rows.remove(0);
                }
            }
            TimelineViewDeltaOp::PopBack => {
                rows.pop();
            }
            TimelineViewDeltaOp::Insert { index, row } => {
                if *index <= rows.len() {
                    rows.insert(*index, row.clone());
                }
            }
            TimelineViewDeltaOp::Set { index, row } => {
                if let Some(slot) = rows.get_mut(*index) {
                    *slot = row.clone();
                }
            }
            TimelineViewDeltaOp::Remove { index } => {
                if *index < rows.len() {
                    rows.remove(*index);
                }
            }
            TimelineViewDeltaOp::Truncate { len } => rows.truncate(*len),
            TimelineViewDeltaOp::Reset { rows: values } => *rows = values.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU64;
    use std::sync::Arc;

    use super::*;
    use crate::app::timeline::{TimelineOtherRow, TimelineViewDeltaBatch};

    fn row(item_id: &str, body: &str) -> TimelineViewRow {
        TimelineViewRow::Other(TimelineOtherRow {
            item_id: item_id.to_owned(),
            event_id: Some(format!("${item_id}")),
            event: None,
            event_type: None,
            forward_transport: None,
            summary: body.to_owned(),
        })
    }

    fn read_state(tail: &str) -> TimelineReadState {
        TimelineReadState {
            visible_tail_event_id: Some(tail.to_owned()),
            receipt_tail_event_id: None,
            own_read_event_id: None,
            unread_anchor_event_id: None,
            is_marked_unread: false,
        }
    }

    fn view(
        focus: ViewFocus,
        rows: Vec<TimelineViewRow>,
        unclaimed_ttl: Option<Duration>,
    ) -> (PublishedView, Arc<Mutex<Vec<TimelineViewDeltaBatch>>>) {
        let batches = Arc::new(Mutex::new(Vec::new()));
        let sink = batches.clone();
        let emitter = ViewDeltaEmitter::new(
            Arc::new(move |batch| sink.lock().unwrap().push(batch)),
            3,
            "live:!room:example.org:1".into(),
            "!room:example.org".into(),
            Arc::new(AtomicU64::new(0)),
        );
        let (view, _settle) = PublishedView::new(PublishedViewInit {
            session_generation: 3,
            room_id: "!room:example.org".into(),
            focus,
            emitter,
            unclaimed_ttl,
            rows,
            position: TimelineViewPosition::LiveBottom,
            pagination: TimelinePaginationState {
                backward: TimelinePageState::Available,
                forward: focus.initial_forward(),
            },
            read_state: read_state("$a"),
            pinned_event_ids: Vec::new(),
            action_authority: TimelineRoomActionAuthority::default(),
        });
        (view, batches)
    }

    #[test]
    fn a_snapshot_taken_between_batches_matches_its_revision_exactly() {
        let (view, batches) = view(ViewFocus::Live, vec![row("a", "one")], None);
        view.publish(ViewChange {
            ops: vec![TimelineViewDeltaOp::PushBack {
                row: row("b", "two"),
            }],
            ..ViewChange::default()
        });
        let snapshot = view.snapshot();
        view.publish(ViewChange {
            ops: vec![
                TimelineViewDeltaOp::Insert {
                    index: 1,
                    row: row("c", "three"),
                },
                TimelineViewDeltaOp::Remove { index: 0 },
            ],
            ..ViewChange::default()
        });

        assert_eq!(snapshot.revision, 1);
        assert_eq!(snapshot.rows, vec![row("a", "one"), row("b", "two")]);
        // A presenter that adopts the snapshot and applies every later batch
        // ends exactly where the stream is: nothing doubled, nothing missed.
        let mut presented = snapshot.rows.clone();
        for batch in batches
            .lock()
            .unwrap()
            .iter()
            .filter(|batch| batch.revision > snapshot.revision)
        {
            apply_view_ops(&mut presented, &batch.ops);
        }
        assert_eq!(presented, view.snapshot().rows);
        assert_eq!(presented, vec![row("c", "three"), row("b", "two")]);
        assert_eq!(view.snapshot().revision, 2);
    }

    #[test]
    fn unchanged_sets_are_dropped_and_changed_ones_kept() {
        let (view, _) = view(
            ViewFocus::Live,
            vec![row("a", "one"), row("b", "two")],
            None,
        );
        let ops = view.without_unchanged_sets(vec![
            TimelineViewDeltaOp::Set {
                index: 0,
                row: row("a", "one"),
            },
            TimelineViewDeltaOp::Set {
                index: 1,
                row: row("b", "edited"),
            },
            // Back to the published row after an earlier set in this batch:
            // it still changes what the presenter shows.
            TimelineViewDeltaOp::Set {
                index: 1,
                row: row("b", "two"),
            },
        ]);
        assert_eq!(
            ops,
            vec![
                TimelineViewDeltaOp::Set {
                    index: 1,
                    row: row("b", "edited"),
                },
                TimelineViewDeltaOp::Set {
                    index: 1,
                    row: row("b", "two"),
                },
            ]
        );

        // After a structural op the indices no longer match published rows.
        let ops = view.without_unchanged_sets(vec![
            TimelineViewDeltaOp::PushFront {
                row: row("z", "zero"),
            },
            TimelineViewDeltaOp::Set {
                index: 1,
                row: row("a", "one"),
            },
        ]);
        assert_eq!(ops.len(), 2);
    }

    #[test]
    fn pagination_and_metadata_publish_only_when_they_change() {
        let (view, batches) = view(ViewFocus::Event, vec![row("a", "one")], None);
        assert!(!view.publish(ViewChange {
            forward: Some(TimelinePageState::Available),
            ..ViewChange::default()
        }));
        assert!(view.publish(ViewChange {
            forward: Some(TimelinePageState::Exhausted),
            ..ViewChange::default()
        }));
        assert_eq!(
            batches.lock().unwrap()[0].pagination,
            Some(TimelinePaginationState {
                backward: TimelinePageState::Available,
                forward: TimelinePageState::Exhausted,
            })
        );
        assert_eq!(
            view.snapshot().pagination.forward,
            TimelinePageState::Exhausted
        );

        // Unlatching only reopens an exhausted backward state.
        assert!(!view.publish(ViewChange {
            unlatch_backward: true,
            ..ViewChange::default()
        }));
        view.publish(ViewChange {
            backward: Some(TimelinePageState::Exhausted),
            ..ViewChange::default()
        });
        view.publish(ViewChange {
            unlatch_backward: true,
            ..ViewChange::default()
        });
        assert_eq!(
            view.snapshot().pagination.backward,
            TimelinePageState::Available
        );
    }

    #[test]
    fn a_read_state_projected_for_a_left_position_is_dropped() {
        let (view, _) = view(ViewFocus::Live, vec![row("a", "one")], None);
        let unread = TimelineViewPosition::Unread {
            anchor_event_id: "$a".into(),
        };
        view.publish(ViewChange {
            read_state: Some(read_state("$stale")),
            read_state_position: Some(unread),
            ..ViewChange::default()
        });
        assert_eq!(view.read_state(), read_state("$a"));
        view.publish(ViewChange {
            read_state: Some(read_state("$b")),
            read_state_position: Some(TimelineViewPosition::LiveBottom),
            ..ViewChange::default()
        });
        assert_eq!(view.read_state(), read_state("$b"));
    }

    #[test]
    fn thread_streams_report_no_forward_pagination() {
        let (thread, _) = view(ViewFocus::Thread, Vec::new(), None);
        let snapshot = thread.snapshot();
        assert_eq!(snapshot.pagination.forward, TimelinePageState::Unavailable);
        assert!(!snapshot.capabilities.paginate_forward);
        let (live, _) = view(ViewFocus::Live, Vec::new(), None);
        assert_eq!(
            live.snapshot().pagination.forward,
            TimelinePageState::Exhausted
        );
        assert!(live.snapshot().capabilities.paginate_forward);
        let (event, _) = view(ViewFocus::Event, Vec::new(), None);
        assert_eq!(
            event.snapshot().pagination.forward,
            TimelinePageState::Available
        );
    }

    #[test]
    fn batches_left_unclaimed_past_the_ttl_expire_the_stream() {
        let push = |view: &PublishedView, id: &str| {
            view.publish(ViewChange {
                ops: vec![TimelineViewDeltaOp::PushBack { row: row(id, id) }],
                ..ViewChange::default()
            });
        };
        let (claimed, _) = view(ViewFocus::Live, Vec::new(), Some(Duration::ZERO));
        push(&claimed, "a");
        claimed.claim();
        push(&claimed, "b");
        assert!(!claimed.is_expired(), "the consumer read the first batch");

        let (abandoned, _) = view(ViewFocus::Live, Vec::new(), Some(Duration::ZERO));
        push(&abandoned, "a");
        assert!(!abandoned.is_expired(), "one unread batch starts the clock");
        push(&abandoned, "b");
        assert!(abandoned.is_expired());

        let (kept, _) = view(ViewFocus::Live, Vec::new(), None);
        push(&kept, "a");
        push(&kept, "b");
        assert!(
            !kept.is_expired(),
            "without a TTL streams stay until closed"
        );
    }
}
