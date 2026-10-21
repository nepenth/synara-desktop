//! D0.3 live Matrix SDK timeline ownership and privacy-safe projection.
//!
//! SDK timeline objects stay inside the Rust session. The webview receives a
//! product snapshot containing only stable identifiers, sender IDs,
//! event types, timestamps, and safe display text.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::Instant;

use eyeball_im::VectorDiff;
use futures_util::{stream, StreamExt};
use matrix_sdk::{
    event_cache::PaginationStatus,
    room::{calls::CallError, edit::EditedContent, Receipts},
    ruma::{
        api::client::receipt::create_receipt::v3::ReceiptType,
        api::client::room::get_event_by_timestamp,
        events::{
            poll::unstable_start::UnstablePollStartEventContent,
            reaction::ReactionEventContent,
            receipt::{ReceiptThread, ReceiptType as EventReceiptType},
            relation::Annotation,
            room::message::{
                MessageFormat, MessageType, Relation, RoomMessageEventContent,
                RoomMessageEventContentWithoutRelation,
            },
            sticker::StickerEventContent,
            AnyMessageLikeEventContent, AnySyncMessageLikeEvent, AnySyncTimelineEvent, Mentions,
            StateEventType,
        },
        EventId, MilliSecondsSinceUnixEpoch, OwnedEventId, OwnedRoomId, OwnedUserId, RoomId, UInt,
        UserId,
    },
    Client, EncryptionState, Room,
};
use matrix_sdk_crypto::types::events::UtdCause;
use matrix_sdk_ui::timeline::{
    EncryptedMessage, MsgLikeKind, ThreadListPaginationState, ThreadListService, Timeline,
    TimelineBuilder, TimelineDetails, TimelineEventFocusThreadMode, TimelineEventItemId,
    TimelineFocus, TimelineItem as SdkTimelineItem, TimelineItemContent as SdkTimelineItemContent,
    TimelineReadReceiptTracking,
};
use serde::{Deserialize, Serialize};
use tokio::time::{timeout, Duration};
use tokio::{sync::Mutex as AsyncMutex, task::JoinHandle};

use crate::app::account_data::{
    append_agent_approval_history_item_live, prune_agent_approval_history_items,
    snapshot_agent_approval_history, NativeAgentApprovalHistorySnapshot,
    SynaraAgentApprovalHistoryDecision, SynaraAgentApprovalHistoryItem,
};
use crate::app::agent_approvals::{
    agent_approval_history_summary, plan_agent_approval, AgentApprovalDecisionStatus,
    AGENT_APPROVAL_TTL_MS,
};
use crate::app::send::{
    abort_queued_send, apply_poll_start_relations, edit_message_content,
    enqueue_event_via_room_queue, is_wedged_send_diagnostic, message_content, normalize_poll,
    parse_edit_event_id, parse_reply_event_id, parse_send_room_id, parse_thread_root_event_id,
    parse_transaction_id, poll_response_content, poll_start_content, queued_send_is_wedged,
    queued_send_outcome, redact_via_room_queue, reenable_queued_send, send_event_via_room_queue,
    unwedge_queued_send, wait_for_queued_send, MatrixPollRespondResult, MatrixSendPollResult,
    MatrixSendTextResult, QueuedSendSession, SendQueue,
};
use crate::app::threads::{
    rebuild_thread_index, NativeThreadListSnapshot, ThreadIndex, ThreadListItemProjection,
};
use crate::app::utd_recovery::{UtdRecoveryCoordinator, UtdRecoveryKind, MAX_EVENT_IDS_PER_BATCH};
use crate::dto::{LocalEchoState, RoomEncryptionStatus, TimelineEncryptedUnavailableItem};

#[cfg(test)]
use super::{view::TimelineViewDeltaBatch, TIMELINE_VIEW_SCHEMA_VERSION};

use super::{
    format_forwarded_media_body, format_forwarded_plain_body, project_timeline_diffs_with_media,
    project_timeline_item_with_media,
    reactions::{
        enrich_native_items, enrich_native_reactions, enrich_view_delta_ops, enrich_view_rows,
        reaction_event_id_from_send_state, KnownAnnotationIds,
    },
    reply_draft_readback, should_attach_formatted_body, snapshot_pinned_events,
    ComposerDraftRegistry, NativeAgentApprovalDecisionRequest, NativeAgentApprovalDecisionResult,
    NativeComposerReplyDraft, NativeComposerReplyDraftReadback, NativeDecryptionState,
    NativeReactionMutation, NativeReactionMutationResult, NativeTimelineActionKind,
    NativeTimelineActionReadback, NativeTimelineCloseRequest, NativeTimelineDirection,
    NativeTimelineEventReadback, NativeTimelineEventRoute, NativeTimelineFollowLiveRequest,
    NativeTimelineItem, NativeTimelineJumpLatestRequest, NativeTimelineOpenPosition,
    NativeTimelineOpenReadback, NativeTimelineOpenRequest, NativeTimelineReaction,
    NativeTimelineReactionSender, NativeTimelineReadAction, NativeTimelineReadIntent,
    NativeTimelineReadStateReadback, NativeTimelineReadStateRequest, NativeTimelineSnapshot,
    NativeTimelineTimestampToEventReadback, NativeTimelineViewPaginationRequest,
    NativeTimelineViewportHint, NativeUtdPhase, NativeUtdStatus, PinnedEventsSnapshot,
    TimelineMediaRegistry, TimelineMediaSource, TimelinePageState, TimelinePaginationState,
    TimelineReadState, TimelineRoomActionAuthority, TimelineViewPosition, TimelineViewSnapshot,
    TimelineViewUpdateEmit, UtdIndex, UtdPhase, UtdReasonCode, ViewDeltaEmitter,
    NATIVE_TIMELINE_ACTION_SCHEMA_VERSION, NATIVE_TIMELINE_OPEN_SCHEMA_VERSION,
    NATIVE_TIMELINE_VIEWPORT_RESTORE_TTL_MS,
};

#[cfg(test)]
mod agent_approval_history_overlay_tests;
mod approval_history;
#[cfg(test)]
mod approval_history_tests;
#[cfg(test)]
mod eviction_tests;
#[cfg(test)]
mod focused_route_tests;
#[cfg(test)]
mod registry_lock_tests;
#[cfg(test)]
mod room_read_state_tests;
#[cfg(test)]
mod stream_view_tests;
#[cfg(test)]
mod thread_list_tests;
#[cfg(test)]
mod thread_open_tests;
#[cfg(test)]
mod thread_receipt_tests;
use approval_history::{ApprovalHistory, HistoryProtection};
mod approval_inbox;
mod published;
mod recency;
use approval_inbox::ApprovalInboxOwner;
pub use approval_inbox::{
    NativeAgentApprovalInboxCoverage, NativeAgentApprovalInboxItem,
    NativeAgentApprovalInboxSnapshot, NativeAgentApprovalInboxStatus,
};
use published::{PublishedView, PublishedViewInit, SettleRequest, ViewChange, ViewFocus};
use recency::{RecencyMap, UseStamp};

const PAGINATION_BATCH_SIZE: u16 = 30;
const REDACTED_PLACEHOLDER: &str = "Message removed";
const UTD_PLACEHOLDER: &str = "Unable to decrypt this message";
const UNSUPPORTED_PLACEHOLDER: &str = "Unsupported event";
const MAX_FOCUSED_EVENT_READBACKS: usize = 256;
/// Event-focused and thread timelines kept for reuse. Each one holds an SDK
/// timeline with its own items, so the cache is far smaller than the
/// decision-dedup window above.
const MAX_FOCUSED_TIMELINES: usize = 32;
/// Live room timelines kept after their last use. Rooms with an open view
/// stream are never evicted, so this bounds only idle rooms.
const MAX_LIVE_TIMELINES: usize = 16;
/// Open view streams. A renderer reload can orphan streams it never closes;
/// past this cap the least recently used stream is closed.
const MAX_VIEW_STREAMS: usize = 32;
/// How long a view stream's batches may stay unread before it expires (see
/// [`NativeTimelineOwner::with_unclaimed_stream_ttl`]). A consumer showing
/// the stream reads it within seconds of each batch.
pub const UNCLAIMED_VIEW_STREAM_TTL: Duration = Duration::from_secs(10 * 60);
const FOCUSED_CONTEXT_EVENT_COUNT: u16 = 25;
/// Bound on building an event-focused timeline. matrix-sdk 0.19 holds its
/// event-cache-wide state lock while it fetches `/context`, which stalls sync
/// processing and every other room's first timeline build until it ends, so
/// a slow request must not run for the full product retry budget.
const FOCUSED_BUILD_TIMEOUT: Duration = Duration::from_secs(10);
/// Live and permalink Event timelines hide in-thread replies now that
/// `NativeTimelineOpenPosition::Thread` owns the threaded stream.
const HIDE_THREADED_EVENTS: bool = true;

/// Follow one queued text send after `send_text` has returned, recording the
/// SDK outcome so a repeat request with the same caller txn answers Sent with
/// the event id. Recoverable failures and wedges keep following: the row's
/// Retry or the recovery owner can still send it. Ends when the request is
/// sent or discarded, or when the room queue closes with the client.
fn spawn_queued_text_follower(
    sends: Arc<tokio::sync::Mutex<SendQueue>>,
    mut session: QueuedSendSession,
) {
    tokio::spawn(async move {
        let txn = session.transaction_id.clone();
        loop {
            let result = wait_for_queued_send(&mut session).await;
            let mut sends = sends.lock().await;
            match result {
                Ok(ack) => {
                    let _ = sends.mark_sent_with_event(&txn, ack.event_id);
                    break;
                }
                Err(error) if error.cancelled => {
                    let _ = sends.cancel(&txn);
                    break;
                }
                Err(error) if error.wedged => {
                    let _ = sends.mark_wedged(&txn, error.diagnostic_id);
                }
                Err(error) if error.still_queued => {}
                Err(_) => break,
            }
        }
    });
}

fn permalink_event_thread_mode() -> TimelineEventFocusThreadMode {
    TimelineEventFocusThreadMode::Automatic {
        hide_threaded_events: HIDE_THREADED_EVENTS,
    }
}

fn optional_draft_thread_root(
    thread_root_event_id: Option<&str>,
) -> Result<Option<&str>, &'static str> {
    match thread_root_event_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        None => Ok(None),
        Some(thread_root) => {
            parse_action_event_id(thread_root, "v-timeline-reply-draft-invalid-event-id")?;
            Ok(Some(thread_root))
        }
    }
}

const AGENT_APPROVAL_SIDE_EFFECT_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_TIMELINE_ACTION_REASON_CHARS: usize = 512;

fn normalize_timeline_action_reason(
    reason: Option<&str>,
    too_long_diagnostic: &'static str,
) -> Result<Option<String>, &'static str> {
    let reason = reason.map(str::trim).filter(|value| !value.is_empty());
    if reason.is_some_and(|value| value.chars().count() > MAX_TIMELINE_ACTION_REASON_CHARS) {
        return Err(too_long_diagnostic);
    }
    Ok(reason.map(str::to_owned))
}

fn validate_poll_vote_selection(
    answer_ids: Vec<String>,
    available_answer_ids: &HashSet<String>,
    max_selections: usize,
    is_closed: bool,
) -> Result<Vec<String>, &'static str> {
    if is_closed {
        return Err("v-timeline-poll-vote-closed");
    }
    if max_selections == 0 {
        return Err("v-timeline-poll-vote-selection-bound-invalid");
    }
    // Empty `answer_ids` is the MSC3381 retract/clear (`answers: []`). Desktop
    // and iOS "Clear vote" presenters already submit that list.
    if answer_ids.len() > max_selections {
        return Err("v-timeline-poll-vote-too-many-answers");
    }
    let mut unique = HashSet::with_capacity(answer_ids.len());
    for answer_id in &answer_ids {
        if answer_id.is_empty()
            || answer_id.trim() != answer_id
            || !available_answer_ids.contains(answer_id)
        {
            return Err("v-timeline-poll-vote-invalid-answer");
        }
        if !unique.insert(answer_id.as_str()) {
            return Err("v-timeline-poll-vote-duplicate-answer");
        }
    }
    Ok(answer_ids)
}

fn position_allows_mark_read(position: &TimelineViewPosition) -> bool {
    matches!(
        position,
        TimelineViewPosition::LiveBottom | TimelineViewPosition::Thread { .. }
    )
}

async fn thread_unread_count(
    client: &Client,
    room_id: &RoomId,
    root_event_id: &str,
) -> Option<u32> {
    let thread_id = OwnedEventId::try_from(root_event_id).ok()?;
    let (cache, _drop) = client
        .event_cache()
        .thread(room_id, &thread_id)
        .await
        .ok()?;
    let unread = cache.num_unread_messages().await.ok()?;
    Some(unread.min(u64::from(u32::MAX)) as u32)
}

fn exact_read_receipts(event_id: OwnedEventId) -> Receipts {
    Receipts::new()
        .fully_read_marker(Some(event_id.clone()))
        .private_read_receipt(Some(event_id))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LiveTimelineReadMark {
    acknowledged_event_id: Option<OwnedEventId>,
    unread_flag_cleared: bool,
}

/// Closed readback for an explicit room mark-read that does not require an open view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomReadStateReadback {
    pub receipt_sent: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acknowledged_event_id: Option<String>,
    pub unread_flag_cleared: bool,
}

fn room_readback_from_mark(
    mark: &LiveTimelineReadMark,
) -> Result<NativeRoomReadStateReadback, &'static str> {
    if mark.acknowledged_event_id.is_none() && !mark.unread_flag_cleared {
        return Err("v-rooms-room-read-state-mark-read-failed");
    }
    Ok(NativeRoomReadStateReadback {
        receipt_sent: mark.acknowledged_event_id.is_some(),
        acknowledged_event_id: mark.acknowledged_event_id.as_ref().map(ToString::to_string),
        unread_flag_cleared: mark.unread_flag_cleared,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LiveReadTargetPlan {
    Send(OwnedEventId),
    ClearUnreadFlag,
    NoOp,
}

fn plan_live_read_target(
    latest_event_id: Option<OwnedEventId>,
    intent: NativeTimelineReadIntent,
    observed_live_tail_event_id: Option<&str>,
) -> Result<LiveReadTargetPlan, &'static str> {
    match intent {
        NativeTimelineReadIntent::AutomaticVisibility => {
            let observed_event_id = parse_action_event_id(
                observed_live_tail_event_id
                    .filter(|event_id| !event_id.trim().is_empty())
                    .ok_or("v-timeline-read-observed-tail-required")?,
                "v-timeline-read-observed-tail-invalid",
            )?;
            match latest_event_id {
                Some(latest_event_id) if latest_event_id == observed_event_id => {
                    Ok(LiveReadTargetPlan::Send(latest_event_id))
                }
                _ => Ok(LiveReadTargetPlan::NoOp),
            }
        }
        NativeTimelineReadIntent::ExplicitUser => {
            if observed_live_tail_event_id.is_some() {
                return Err("v-timeline-read-observed-tail-unexpected");
            }
            Ok(match latest_event_id {
                Some(latest_event_id) => LiveReadTargetPlan::Send(latest_event_id),
                None => LiveReadTargetPlan::ClearUnreadFlag,
            })
        }
    }
}

/// Pages explicit mark-read may fetch to find a receipt-capable remote event,
/// and the total time it may spend doing so. Callers run this after releasing
/// the timeline registry lock, so other timeline commands are not stalled.
const EXPLICIT_READ_TARGET_PAGES: usize = 3;
const EXPLICIT_READ_TARGET_BUDGET: Duration = Duration::from_secs(8);

/// Latest remote event in the room, including in-thread replies.
///
/// The live timeline hides threaded events (`HIDE_THREADED_EVENTS`), so
/// `Timeline::latest_event_id` skips thread replies. Room unread counts and
/// the homeserver both count them, so a room whose newest activity is in a
/// thread would never clear. The room event cache keeps every event in one
/// linked chunk (threading support is not enabled), so read it first and fall
/// back to the timeline's own resolver.
async fn latest_room_event_id(timeline: &Timeline) -> Option<OwnedEventId> {
    if !timeline.is_threaded() {
        if let Ok((cache, _drop_handles)) = timeline.room().event_cache().await {
            if let Ok(Some(event_id)) = cache
                .rfind_map_event_in_memory_by(|event| event.event_id().map(ToOwned::to_owned))
                .await
            {
                return Some(event_id);
            }
        }
    }
    timeline.latest_event_id().await
}

/// Resolve the event an explicit mark-read must acknowledge.
///
/// A freshly opened timeline can be empty while the room has remote events:
/// the event cache was cold and the open-time page failed, timed out, or held
/// only hidden events. Treating that as "no events" would clear only the
/// local unread flag and never move `m.fully_read`, so other clients of the
/// same user keep the room unread. Page backwards until a remote event
/// appears. `Ok(None)` means the start of the room was reached with no
/// receipt-capable event. A page failure, the time budget, or running out of
/// pages is an error rather than a silent flag clear.
async fn resolve_explicit_read_target(
    timeline: &Timeline,
) -> Result<Option<OwnedEventId>, &'static str> {
    if let Some(event_id) = latest_room_event_id(timeline).await {
        return Ok(Some(event_id));
    }
    if timeline.is_threaded() {
        return Ok(None);
    }
    timeout(EXPLICIT_READ_TARGET_BUDGET, async {
        for _ in 0..EXPLICIT_READ_TARGET_PAGES {
            let hit_start = timeline
                .paginate_backwards(PAGINATION_BATCH_SIZE)
                .await
                .map_err(|_| "v-timeline-read-target-unresolved")?;
            if let Some(event_id) = latest_room_event_id(timeline).await {
                return Ok(Some(event_id));
            }
            if hit_start {
                return Ok(None);
            }
        }
        Err("v-timeline-read-target-unresolved")
    })
    .await
    .map_err(|_| "v-timeline-read-target-unresolved")?
}

/// Advance the private receipt and fully-read marker through the SDK owner.
///
/// Automatic visibility requests are compare-and-target operations: the exact
/// event painted by the client must still be the SDK-authoritative live tail.
/// A newer arrival therefore produces a no-op, and the write can never jump to
/// an event the client did not observe. Explicit user actions intentionally
/// resolve the current tail at execution time.
async fn mark_live_timeline_read(
    timeline: &Timeline,
    intent: NativeTimelineReadIntent,
    observed_live_tail_event_id: Option<&str>,
) -> Result<LiveTimelineReadMark, &'static str> {
    // Use the same SDK-owned latest-event resolver as Timeline::mark_as_read;
    // hand-walking visible items diverges for local echoes, focus, and threads.
    let latest_event_id = match intent {
        NativeTimelineReadIntent::AutomaticVisibility => timeline.latest_event_id().await,
        NativeTimelineReadIntent::ExplicitUser => resolve_explicit_read_target(timeline).await?,
    };
    match plan_live_read_target(latest_event_id, intent, observed_live_tail_event_id)? {
        LiveReadTargetPlan::NoOp => Ok(LiveTimelineReadMark {
            acknowledged_event_id: None,
            unread_flag_cleared: false,
        }),
        LiveReadTargetPlan::ClearUnreadFlag => {
            // Explicit Mark Read must still clear a manually marked-unread room
            // when the room has no receipt-capable remote event. A thread stream
            // must not clear the room unread flag.
            if timeline.is_threaded() {
                return Ok(LiveTimelineReadMark {
                    acknowledged_event_id: None,
                    unread_flag_cleared: false,
                });
            }
            timeline
                .room()
                .set_unread_flag(false)
                .await
                .map_err(|_| "v-timeline-clear-empty-unread-failed")?;
            Ok(LiveTimelineReadMark {
                acknowledged_event_id: None,
                unread_flag_cleared: true,
            })
        }
        LiveReadTargetPlan::Send(event_id) => {
            if timeline.is_threaded() {
                // 0.19 FullyRead receipts are always Unthreaded. Thread streams
                // send a private receipt so Timeline infers ReceiptThread::Thread
                // from TimelineFocus::Thread.
                timeline
                    .send_single_receipt(ReceiptType::ReadPrivate, event_id.clone())
                    .await
                    .map_err(|_| "v-timeline-send-thread-receipt-failed")?;
            } else if intent == NativeTimelineReadIntent::ExplicitUser {
                // An explicit Mark as Read always writes. `Timeline`'s receipt
                // deduplication compares against main-timeline positions and can
                // drop the write when the target is an in-thread reply or the
                // local receipt view is stale, which would report success while
                // other clients keep the room unread. `Room::send_multiple_receipts`
                // posts the markers and then clears the unread flag.
                timeline
                    .room()
                    .send_multiple_receipts(exact_read_receipts(event_id.clone()))
                    .await
                    .map_err(|_| "v-timeline-send-read-markers-failed")?;
            } else {
                timeline
                    // Pinned matrix-sdk-ui 0.19 invariant: `Timeline::send_multiple_receipts`
                    // clears the SDK room's unread flag after a submitted marker update and
                    // also when receipt deduplication removes every unchanged marker. Keep
                    // this evidence in the adjacent regression test when upgrading the SDK.
                    .send_multiple_receipts(exact_read_receipts(event_id.clone()))
                    .await
                    .map_err(|_| "v-timeline-send-read-markers-failed")?;
            }
            Ok(LiveTimelineReadMark {
                acknowledged_event_id: Some(event_id),
                unread_flag_cleared: false,
            })
        }
    }
}

/// Apply a stream's read or unread action to its timeline.
async fn apply_stream_read_action(
    timeline: &Timeline,
    request: &NativeTimelineReadStateRequest,
) -> Result<(Option<bool>, Option<OwnedEventId>), &'static str> {
    match request.action {
        NativeTimelineReadAction::MarkRead => {
            let mark = mark_live_timeline_read(
                timeline,
                request.intent,
                request.observed_live_tail_event_id.as_deref(),
            )
            .await?;
            Ok((
                Some(mark.acknowledged_event_id.is_some()),
                mark.acknowledged_event_id,
            ))
        }
        NativeTimelineReadAction::MarkUnread => {
            if request.intent != NativeTimelineReadIntent::ExplicitUser
                || request.observed_live_tail_event_id.is_some()
            {
                return Err("v-timeline-read-mark-unread-requires-explicit-intent");
            }
            timeline
                .room()
                .set_unread_flag(true)
                .await
                .map_err(|_| "v-timeline-view-mark-unread-failed")?;
            Ok((None, None))
        }
    }
}

fn remember_agent_approval_decision(
    decisions: &mut VecDeque<(String, String)>,
    decision_key: (String, String),
) {
    if let Some(existing_index) = decisions.iter().position(|entry| entry == &decision_key) {
        decisions.remove(existing_index);
    }
    if decisions.len() >= MAX_FOCUSED_EVENT_READBACKS {
        decisions.pop_front();
    }
    decisions.push_back(decision_key);
}

fn agent_approval_now_ms() -> Result<u64, &'static str> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "agent-approval-clock-invalid")?
        .as_millis()
        .try_into()
        .map_err(|_| "agent-approval-clock-invalid")
}

#[derive(Default)]
struct ApprovalDecisionRegistry {
    completed: VecDeque<(String, String)>,
    /// Weak per-event locks serialize duplicate actions while allowing
    /// unrelated rooms/events to progress independently. Expired entries are
    /// discarded whenever a new action resolves its lock.
    in_flight: HashMap<(String, String), Weak<AsyncMutex<()>>>,
}

impl ApprovalDecisionRegistry {
    fn is_completed(&self, key: &(String, String)) -> bool {
        self.completed.contains(key)
    }

    fn lock_for(&mut self, key: &(String, String)) -> Arc<AsyncMutex<()>> {
        self.in_flight.retain(|_, lock| lock.strong_count() > 0);
        if let Some(lock) = self.in_flight.get(key).and_then(Weak::upgrade) {
            return lock;
        }
        let lock = Arc::new(AsyncMutex::new(()));
        self.in_flight.insert(key.clone(), Arc::downgrade(&lock));
        lock
    }

    fn remember(&mut self, key: (String, String)) {
        remember_agent_approval_decision(&mut self.completed, key);
    }
}

/// A redaction or reaction that is still in the persisted send queue
/// (recoverable error or slow server) will be sent in order and already shows
/// as a local echo, so it is accepted rather than reported as a failure the
/// user would retry. Wedged, cancelled or rejected requests fail.
fn accept_still_queued(
    result: Result<crate::app::send::QueuedSendAck, crate::app::send::QueuedSendError>,
    failed: &'static str,
) -> Result<(), &'static str> {
    match result {
        Ok(_) => Ok(()),
        Err(error) if error.still_queued => Ok(()),
        Err(_) => Err(failed),
    }
}

fn approval_reaction_readback(
    existing: &[NativeTimelineReaction],
    reaction_key: &str,
    own_user_id: &str,
    reaction_event_id: String,
) -> NativeTimelineReaction {
    let mut readback = existing
        .iter()
        .find(|reaction| reaction.key == reaction_key)
        .cloned()
        .unwrap_or_else(|| NativeTimelineReaction {
            key: reaction_key.to_owned(),
            count: 0,
            me: false,
            senders: Vec::new(),
        });
    if !readback.me {
        readback.count = readback.count.saturating_add(1);
    }
    readback.me = true;
    readback
        .senders
        .retain(|sender| sender.user_id != own_user_id);
    readback.senders.push(NativeTimelineReactionSender {
        user_id: own_user_id.to_owned(),
        reaction_event_id: Some(reaction_event_id),
    });
    readback
}

struct LiveTimelineEntry {
    timeline: Arc<Timeline>,
    is_encrypted: bool,
    last_used: UseStamp,
}

struct ViewStreamEntry {
    _approval_history: HistoryProtection,
    room_id: String,
    timeline: Arc<Timeline>,
    /// Rows, position, pagination and revision, shared with the update task.
    view: Arc<PublishedView>,
    last_access: UseStamp,
}

impl ViewStreamEntry {
    /// The stream a consumer command acts on, marked as used and read.
    /// An expired stream no longer receives updates and reads as closed.
    fn claim(&self) -> Result<&Self, &'static str> {
        if self.view.is_expired() {
            return Err("v-timeline-view-not-open");
        }
        self.last_access.touch();
        self.view.claim();
        Ok(self)
    }
}

/// Media registries of the open view streams, by stream id.
///
/// Media handles resolve from here, without the timeline registry lock, so a
/// slow open or page in one room never stalls image loads anywhere else.
#[derive(Default)]
struct StreamMediaIndex {
    streams: Mutex<HashMap<String, Arc<AsyncMutex<TimelineMediaRegistry>>>>,
}

impl StreamMediaIndex {
    fn streams(
        &self,
    ) -> std::sync::MutexGuard<'_, HashMap<String, Arc<AsyncMutex<TimelineMediaRegistry>>>> {
        self.streams
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn insert(&self, stream_id: String, media: Arc<AsyncMutex<TimelineMediaRegistry>>) {
        self.streams().insert(stream_id, media);
    }

    fn remove(&self, stream_id: &str) {
        self.streams().remove(stream_id);
    }

    async fn resolve(
        &self,
        session_generation: u64,
        handle_id: &str,
    ) -> Option<TimelineMediaSource> {
        let streams = self.streams().values().cloned().collect::<Vec<_>>();
        for media in streams {
            let registry = media.lock().await;
            if registry.session_generation() != session_generation {
                continue;
            }
            if let Some(source) = registry.resolve(handle_id) {
                return Some(source.clone());
            }
        }
        None
    }
}

pub struct NativeTimelineRegistry {
    approval_history: Arc<ApprovalHistory>,
    media_index: Arc<StreamMediaIndex>,
    session_generation: u64,
    entries: HashMap<String, LiveTimelineEntry>,
    focused_entries: RecencyMap<(String, String), Arc<Timeline>>,
    /// Thread timelines keyed by `(room_id, root_event_id)`. Must not share
    /// `focused_entries`: a permalink of the root and a thread view of the
    /// root are different SDK streams.
    thread_entries: RecencyMap<(String, String), Arc<Timeline>>,
    view_streams: HashMap<String, ViewStreamEntry>,
    view_update_tasks: HashMap<String, JoinHandle<()>>,
    next_view_stream_id: u64,
    /// See [`NativeTimelineOwner::with_unclaimed_stream_ttl`].
    unclaimed_stream_ttl: Option<Duration>,
    utd_index: UtdIndex,
    utd_recovery: UtdRecoveryCoordinator,
}

const AGENT_APPROVAL_HISTORY_PENDING_TTL: Duration = Duration::from_secs(30);

fn lock_approval_history_pending(
    pending: &Mutex<Option<(Instant, Vec<SynaraAgentApprovalHistoryItem>)>>,
) -> std::sync::MutexGuard<'_, Option<(Instant, Vec<SynaraAgentApprovalHistoryItem>)>> {
    pending
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn lock_approval_history_unconfirmed(
    pending: &Mutex<HashMap<(String, String), SynaraAgentApprovalHistoryItem>>,
) -> std::sync::MutexGuard<'_, HashMap<(String, String), SynaraAgentApprovalHistoryItem>> {
    pending
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn approval_history_item_key(item: &SynaraAgentApprovalHistoryItem) -> (String, String) {
    (item.room_id.clone(), item.event_id.clone())
}

fn native_agent_approval_history_item(
    room_id: String,
    event_id: String,
    item: &NativeTimelineItem,
    decision: SynaraAgentApprovalHistoryDecision,
    decided_at: f64,
) -> SynaraAgentApprovalHistoryItem {
    SynaraAgentApprovalHistoryItem {
        room_id,
        event_id,
        sender: item.sender.clone(),
        decision,
        decided_at,
        origin_server_ts: item.origin_server_ts as f64,
        expires_at: item.origin_server_ts.saturating_add(AGENT_APPROVAL_TTL_MS) as f64,
        summary: agent_approval_history_summary(&item.body),
    }
}

fn fresh_overlay_items(
    pending: &Option<(Instant, Vec<SynaraAgentApprovalHistoryItem>)>,
    now: Instant,
) -> Option<&[SynaraAgentApprovalHistoryItem]> {
    pending.as_ref().and_then(|(stored_at, items)| {
        (now.saturating_duration_since(*stored_at) < AGENT_APPROVAL_HISTORY_PENDING_TTL)
            .then_some(items.as_slice())
    })
}

/// Union a local write overlay with the SDK cache. Overlay-only snapshots hide
/// other-device `/sync` items for the full TTL; merging keeps local decisions
/// visible without masking a later remote RMW that has already reached the store.
fn merge_approval_history_overlay(
    overlay: Option<&[SynaraAgentApprovalHistoryItem]>,
    cached: Vec<SynaraAgentApprovalHistoryItem>,
    now_ms: f64,
) -> Vec<SynaraAgentApprovalHistoryItem> {
    let Some(overlay) = overlay.filter(|items| !items.is_empty()) else {
        return cached;
    };
    prune_agent_approval_history_items(overlay.iter().cloned().chain(cached).collect(), now_ms)
}

fn push_overlay_history_item(
    pending: Option<(Instant, Vec<SynaraAgentApprovalHistoryItem>)>,
    item: SynaraAgentApprovalHistoryItem,
    now: Instant,
) -> (Instant, Vec<SynaraAgentApprovalHistoryItem>) {
    let decided_at = item.decided_at;
    let mut items = fresh_overlay_items(&pending, now)
        .map(Vec::from)
        .unwrap_or_default();
    items.push(item);
    (now, prune_agent_approval_history_items(items, decided_at))
}

/// Shared handle so Core and the desktop session own one live registry.
pub struct NativeTimelineOwner {
    client: Client,
    emit: TimelineViewUpdateEmit,
    session_generation: u64,
    /// Held only to look up, install or publish entries. SDK builds, network
    /// pages and sends run between lock scopes (see [`Slots`]).
    registry: tokio::sync::Mutex<NativeTimelineRegistry>,
    approval_history: Arc<ApprovalHistory>,
    media_index: Arc<StreamMediaIndex>,
    /// One live-timeline build per room at a time; a concurrent open of the
    /// same room waits here and then reuses the entry instead of building a
    /// second SDK timeline.
    live_builds: Mutex<HashMap<String, Weak<AsyncMutex<()>>>>,
    /// Serializes duplicate decisions per exact event without monopolizing the
    /// global timeline registry or blocking unrelated approval prompts.
    approval_decisions: Arc<std::sync::Mutex<ApprovalDecisionRegistry>>,
    approval_inbox: tokio::sync::Mutex<ApprovalInboxOwner>,
    drafts: tokio::sync::Mutex<ComposerDraftRegistry>,
    thread_lists: tokio::sync::Mutex<HashMap<String, ThreadListService>>,
    thread_index: tokio::sync::Mutex<ThreadIndex>,
    /// Shared with the per-send followers that record the SDK outcome.
    sends: Arc<tokio::sync::Mutex<SendQueue>>,
    /// Held across caller-txn lookup, enqueue and record in `send_text`, so a
    /// repeated caller txn cannot race into a second event and Core enqueues
    /// in the order the shell asked.
    send_enqueue: tokio::sync::Mutex<()>,
    approval_history_mutation: tokio::sync::Mutex<()>,
    approval_history_pending: Mutex<Option<(Instant, Vec<SynaraAgentApprovalHistoryItem>)>>,
    /// Local decisions whose reaction landed but whose account-data RMW did
    /// not. `AlreadyDecided` retries drain this map so a failed history write
    /// is not stranded by completed-decision memory.
    approval_history_unconfirmed: Mutex<HashMap<(String, String), SynaraAgentApprovalHistoryItem>>,
    pin_media: AsyncMutex<TimelineMediaRegistry>,
}

impl NativeTimelineOwner {
    pub fn new(client: &Client, emit: TimelineViewUpdateEmit, session_generation: u64) -> Self {
        let _ = client.event_cache().subscribe();
        let registry = NativeTimelineRegistry::new(session_generation);
        let approval_history = registry.approval_history.clone();
        let media_index = registry.media_index.clone();
        Self {
            client: client.clone(),
            emit,
            session_generation,
            registry: tokio::sync::Mutex::new(registry),
            approval_history: approval_history.clone(),
            media_index,
            live_builds: Mutex::new(HashMap::new()),
            approval_decisions: Arc::new(
                std::sync::Mutex::new(ApprovalDecisionRegistry::default()),
            ),
            approval_inbox: tokio::sync::Mutex::new(ApprovalInboxOwner::with_history(
                session_generation,
                approval_history,
            )),
            drafts: tokio::sync::Mutex::new(ComposerDraftRegistry::new()),
            thread_lists: tokio::sync::Mutex::new(HashMap::new()),
            thread_index: tokio::sync::Mutex::new(ThreadIndex::new(session_generation)),
            sends: Arc::new(tokio::sync::Mutex::new(SendQueue::new(session_generation))),
            send_enqueue: tokio::sync::Mutex::new(()),
            approval_history_mutation: tokio::sync::Mutex::new(()),
            approval_history_pending: Mutex::new(None),
            approval_history_unconfirmed: Mutex::new(HashMap::new()),
            pin_media: AsyncMutex::new(TimelineMediaRegistry::new(session_generation, "pinned")),
        }
    }

    /// Expire view streams whose batches stay unread for `ttl`.
    ///
    /// For shells that may stop reading a stream without closing it. An
    /// expired stream's update task stops, so a room nobody shows no longer
    /// runs the per-update pipeline, and the registry closes it on its next
    /// open; a command on it fails as not open, which the shell's
    /// view-not-open recovery answers by reopening. Desktop does not use it:
    /// it closes streams on unmount, and a hidden window's throttled poll
    /// must not lose its stream.
    pub fn with_unclaimed_stream_ttl(mut self, ttl: Duration) -> Self {
        self.registry.get_mut().unclaimed_stream_ttl = Some(ttl);
        self
    }

    pub async fn agent_approvals_list(
        &self,
    ) -> Result<NativeAgentApprovalInboxSnapshot, &'static str> {
        self.agent_approvals_list_with_discovery(false).await
    }

    /// Native clients renew discovery only while their approvals page is visible.
    pub async fn agent_approvals_list_with_discovery(
        &self,
        discovery_active: bool,
    ) -> Result<NativeAgentApprovalInboxSnapshot, &'static str> {
        let decisions = {
            let registry = self
                .approval_decisions
                .lock()
                .map_err(|_| "agent-approval-decision-state-poisoned")?;
            registry.completed.iter().cloned().collect()
        };
        // Only borrow live-bottom timelines. Focused history windows do not
        // certify current cross-room coverage, and registry ownership is unchanged.
        let reusable = {
            let registry = self.registry.lock().await;
            let mut timelines = HashMap::new();
            for view in registry.view_streams.values() {
                if matches!(view.view.position(), TimelineViewPosition::LiveBottom) {
                    timelines.insert(view.room_id.clone(), Arc::downgrade(&view.timeline));
                }
            }
            timelines
        };
        self.approval_inbox
            .lock()
            .await
            .snapshot(&self.client, &decisions, discovery_active, reusable)
            .await
    }

    pub async fn agent_approval_history_snapshot(
        &self,
    ) -> Result<NativeAgentApprovalHistorySnapshot, &'static str> {
        let overlay = {
            let pending = lock_approval_history_pending(&self.approval_history_pending);
            fresh_overlay_items(&pending, Instant::now()).map(Vec::from)
        };
        let unconfirmed = lock_approval_history_unconfirmed(&self.approval_history_unconfirmed)
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let now_ms = agent_approval_now_ms()
            .map(|ms| ms as f64)
            .unwrap_or(f64::NAN);
        let local = merge_approval_history_overlay(overlay.as_deref(), unconfirmed, now_ms);
        match snapshot_agent_approval_history(&self.client).await {
            Ok(cached) => Ok(NativeAgentApprovalHistorySnapshot {
                items: merge_approval_history_overlay(Some(&local), cached.items, now_ms),
            }),
            Err(err) => {
                if local.is_empty() {
                    Err(err)
                } else {
                    Ok(NativeAgentApprovalHistorySnapshot { items: local })
                }
            }
        }
    }

    async fn record_agent_approval_history(
        &self,
        item: SynaraAgentApprovalHistoryItem,
    ) -> Result<(), &'static str> {
        let _lock = self.approval_history_mutation.lock().await;
        {
            let mut pending = lock_approval_history_pending(&self.approval_history_pending);
            *pending = Some(push_overlay_history_item(
                pending.take(),
                item.clone(),
                Instant::now(),
            ));
        }
        match append_agent_approval_history_item_live(&self.client, item.clone()).await {
            Ok(snapshot) => {
                lock_approval_history_unconfirmed(&self.approval_history_unconfirmed)
                    .remove(&approval_history_item_key(&item));
                *lock_approval_history_pending(&self.approval_history_pending) =
                    Some((Instant::now(), snapshot.items));
                Ok(())
            }
            Err(err) => {
                lock_approval_history_unconfirmed(&self.approval_history_unconfirmed)
                    .insert(approval_history_item_key(&item), item);
                Err(err)
            }
        }
    }

    /// Drain a failed history write for this event. Must not run while holding
    /// `approval_decisions` (std mutex) or the per-event decision guard.
    async fn retry_unconfirmed_approval_history(&self, room_id: &str, event_id: &str) {
        let item = lock_approval_history_unconfirmed(&self.approval_history_unconfirmed)
            .remove(&(room_id.to_owned(), event_id.to_owned()));
        let Some(item) = item else {
            return;
        };
        if let Err(diagnostic) = self.record_agent_approval_history(item).await {
            eprintln!("agent-approval-history-write-failed: {diagnostic}");
        }
    }

    pub async fn lock(&self) -> tokio::sync::MutexGuard<'_, NativeTimelineRegistry> {
        self.registry.lock().await
    }

    /// Download bytes for an opaque timeline media handle. Not `Core.command`.
    ///
    /// The handle never includes an `mxc://` URI. Fail-closed codes stay
    /// static. The product cap is 32 MiB.
    pub async fn media_bytes(&self, handle_id: &str) -> Result<Vec<u8>, &'static str> {
        let source = self
            .resolve_media(handle_id)
            .await
            .ok_or("p4-s33-media-unknown-handle")?;
        let request = matrix_sdk::media::MediaRequestParameters {
            source: source.source,
            format: matrix_sdk::media::MediaFormat::File,
        };
        super::super::media::fetch_media_cached(&self.client, &request, 32 * 1024 * 1024)
            .await
            .map_err(|error| match error {
                super::super::media::BoundedMediaError::TooLarge => "p4-s33-media-too-large",
                _ => "p4-s33-media-failed",
            })
    }

    /// Resolve an opaque media handle of any open view stream. Never waits
    /// for the timeline registry.
    pub async fn resolve_media(&self, handle_id: &str) -> Option<TimelineMediaSource> {
        self.media_index
            .resolve(self.session_generation, handle_id)
            .await
    }

    pub async fn event_readback(
        &self,
        room_id: &str,
        event_id: &str,
    ) -> Result<NativeTimelineEventReadback, &'static str> {
        event_readback_in(&mut Slots::Shared(self), &self.client, room_id, event_id).await
    }

    /// Resolve the closest event at or after `timestamp_ms` (SDK 0.18 Forward /
    /// `since`, matching Jump to Time `'f'`). Does not paginate the open view.
    pub async fn timestamp_to_event(
        &self,
        room_id: &str,
        timestamp_ms: u64,
    ) -> Result<NativeTimelineTimestampToEventReadback, &'static str> {
        let room_id = parse_room_id(room_id)?;
        self.client
            .get_room(room_id.as_ref())
            .ok_or("d0.3-timeline-room-not-found")?;
        let millis = UInt::try_from(timestamp_ms)
            .map_err(|_| "p2-timeline-timestamp-to-event-invalid-timestamp")?;
        let request = get_event_by_timestamp::v1::Request::since(
            room_id.clone(),
            MilliSecondsSinceUnixEpoch(millis),
        );
        let response = self
            .client
            .send(request)
            .await
            .map_err(|_| "p2-timeline-timestamp-to-event-failed")?;
        Ok(NativeTimelineTimestampToEventReadback {
            room_id: room_id.to_string(),
            event_id: response.event_id.to_string(),
            origin_server_ts: response.origin_server_ts.get().into(),
        })
    }

    /// Page a view stream. The registry lock covers only the stream lookup
    /// and the final snapshot source; the SDK page runs without it.
    pub async fn paginate(
        &self,
        request: NativeTimelineViewPaginationRequest,
    ) -> Result<TimelineViewSnapshot, &'static str> {
        paginate_view(&mut Slots::Shared(self), request).await
    }

    /// Mark a view stream read or unread.
    ///
    /// The registry lock is held only to validate the stream and to read the
    /// snapshot source. Receipt writes and any explicit-read pagination run without
    /// it, so a slow homeserver does not stall every other timeline command.
    pub async fn set_read_state(
        &self,
        request: NativeTimelineReadStateRequest,
    ) -> Result<NativeTimelineReadStateReadback, &'static str> {
        let (timeline, view) = self.registry.lock().await.read_state_target(&request)?;
        let (receipt_sent, acknowledged_event_id) =
            apply_stream_read_action(&timeline, &request).await?;
        publish_current_read_state(&timeline, &view, self.client.user_id()).await;
        let snapshot = self
            .registry
            .lock()
            .await
            .published_view(&request.stream_id)?
            .snapshot();
        Ok(NativeTimelineReadStateReadback {
            action: request.action,
            receipt_sent,
            acknowledged_event_id: acknowledged_event_id.map(|event_id| event_id.to_string()),
            snapshot,
        })
    }

    /// Promote unread placement on an existing live provider without reopening.
    /// Fails closed when the observed tail is no longer the SDK live tail.
    ///
    /// The registry lock covers only the stream lookup; reading the SDK tail
    /// and the new read state run without it.
    pub async fn follow_live_tail(
        &self,
        request: NativeTimelineFollowLiveRequest,
    ) -> Result<TimelineViewSnapshot, &'static str> {
        follow_live_tail_in(&mut Slots::Shared(self), &self.client, request).await
    }

    /// Mark a room read or unread without requiring an already-open view stream.
    /// Context-menu Mark as Read uses this while the room is not mounted.
    ///
    /// The registry lock covers only looking up or installing the live
    /// timeline. Building it, target resolution (which may paginate) and the
    /// receipt write run without the lock.
    pub async fn set_room_read_state(
        &self,
        room_id: &str,
        action: NativeTimelineReadAction,
    ) -> Result<NativeRoomReadStateReadback, &'static str> {
        let room_id = parse_room_id(room_id)?;
        let room = self
            .client
            .get_room(&room_id)
            .ok_or("v-rooms-room-read-state-room-not-found")?;
        match action {
            NativeTimelineReadAction::MarkRead => {
                let timeline = open_room_read_timeline_in(
                    &mut Slots::Shared(self),
                    &self.client,
                    room_id.as_str(),
                )
                .await?;
                let mark = mark_live_timeline_read(
                    &timeline,
                    NativeTimelineReadIntent::ExplicitUser,
                    None,
                )
                .await
                .map_err(|_| "v-rooms-room-read-state-mark-read-failed")?;
                room_readback_from_mark(&mark)
            }
            NativeTimelineReadAction::MarkUnread => {
                room.set_unread_flag(true)
                    .await
                    .map_err(|_| "v-rooms-room-read-state-mark-unread-failed")?;
                Ok(NativeRoomReadStateReadback {
                    receipt_sent: false,
                    acknowledged_event_id: None,
                    unread_flag_cleared: false,
                })
            }
        }
    }

    pub async fn toggle_reaction(
        &self,
        room_id: &str,
        event_id: &str,
        key: &str,
    ) -> Result<NativeReactionMutationResult, &'static str> {
        toggle_reaction_in(
            &mut Slots::Shared(self),
            &self.client,
            room_id,
            event_id,
            key,
        )
        .await
    }

    /// Waits up to the queued-send bound for the server's acknowledgement,
    /// without holding the registry lock.
    pub async fn ensure_reaction(
        &self,
        room_id: &str,
        event_id: &str,
        key: &str,
    ) -> Result<NativeReactionMutationResult, &'static str> {
        ensure_reaction_in(
            &mut Slots::Shared(self),
            &self.client,
            room_id,
            event_id,
            key,
        )
        .await
    }

    pub async fn decide_agent_approval(
        &self,
        request: NativeAgentApprovalDecisionRequest,
    ) -> Result<NativeAgentApprovalDecisionResult, &'static str> {
        let room_id = parse_room_id(&request.room_id)?.to_string();
        let event_id = parse_event_id(&request.event_id)?;
        let decision_key = (room_id.clone(), event_id.to_string());

        let decision_lock = {
            let mut decisions = self
                .approval_decisions
                .lock()
                .map_err(|_| "agent-approval-decision-state-poisoned")?;
            if decisions.is_completed(&decision_key) {
                None
            } else {
                Some(decisions.lock_for(&decision_key))
            }
        };
        let Some(decision_lock) = decision_lock else {
            // Completed-memory returns must drop the std mutex before this
            // await so a history backfill cannot deadlock other decisions.
            self.retry_unconfirmed_approval_history(&room_id, event_id.as_str())
                .await;
            return Ok(NativeAgentApprovalDecisionResult {
                room_id,
                event_id: event_id.to_string(),
                status: AgentApprovalDecisionStatus::AlreadyDecided,
                reaction: None,
            });
        };
        // Cancellation before a Matrix side effect releases this guard. Once
        // send begins, the guard moves into a detached task below.
        let decision_guard = timeout(
            AGENT_APPROVAL_SIDE_EFFECT_TIMEOUT,
            Arc::clone(&decision_lock).lock_owned(),
        )
        .await
        .map_err(|_| "agent-approval-decision-in-flight-timeout")?;
        if self
            .approval_decisions
            .lock()
            .map_err(|_| "agent-approval-decision-state-poisoned")?
            .is_completed(&decision_key)
        {
            drop(decision_guard);
            self.retry_unconfirmed_approval_history(&room_id, event_id.as_str())
                .await;
            return Ok(NativeAgentApprovalDecisionResult {
                room_id,
                event_id: event_id.to_string(),
                status: AgentApprovalDecisionStatus::AlreadyDecided,
                reaction: None,
            });
        }

        let room = self
            .client
            .get_room(parse_room_id(&room_id)?.as_ref())
            .ok_or("v-crypto.6-event-room-not-found")?;
        let current_user_id = self
            .client
            .user_id()
            .ok_or("agent-approval-current-user-missing")?
            .to_string();
        // A focused encrypted event can arrive first as an undecrypted item
        // and become projectable on a later SDK update. Re-evaluate the exact
        // event for a short bounded window; never fall back to a room scan or
        // another event with similar text. The timeout owns builder, initial
        // subscribe, and update wait—not just the final stream read.
        let (item, plan) = timeout(Duration::from_secs(3), async {
            let timeline = TimelineBuilder::new(&room)
                .with_focus(TimelineFocus::Event {
                    target: event_id.clone(),
                    num_context_events: 0,
                    thread_mode: permalink_event_thread_mode(),
                })
                .build()
                .await
                .map_err(|_| "v-crypto.6-event-open-failed")?;
            let (mut items, mut updates) = timeline.subscribe().await;
            loop {
                let item = items
                    .iter()
                    .filter_map(|item| project_item(item, self.client.user_id()))
                    .find(|item| item.event_id == event_id.as_str());
                if let Some(item) = item {
                    match plan_agent_approval(
                        &request.action_id,
                        &item.body,
                        &item.sender,
                        &current_user_id,
                        item.origin_server_ts,
                        agent_approval_now_ms()?,
                        item.reactions
                            .iter()
                            .map(|reaction| (reaction.key.as_str(), reaction.me)),
                    ) {
                        Ok(plan) => break Ok((item, plan)),
                        Err(error) if item.decryption_state.is_none() => break Err(error),
                        Err(_) => {}
                    }
                }
                match updates.next().await {
                    Some(diffs) => {
                        for diff in diffs {
                            diff.apply(&mut items);
                        }
                    }
                    None => break Err("v-crypto.6-event-not-found"),
                }
            }
        })
        .await
        .map_err(|_| "v-crypto.6-event-open-timeout")??;
        if plan.status == AgentApprovalDecisionStatus::AlreadyDecided {
            self.approval_decisions
                .lock()
                .map_err(|_| "agent-approval-decision-state-poisoned")?
                .remember(decision_key);
            drop(decision_guard);
            if let Some(decision) = SynaraAgentApprovalHistoryDecision::matching_own_reaction(
                &request.action_id,
                item.reactions
                    .iter()
                    .map(|reaction| (reaction.key.as_str(), reaction.me)),
            ) {
                if let Ok(decided_at) = agent_approval_now_ms() {
                    let history_item = native_agent_approval_history_item(
                        room_id.clone(),
                        event_id.to_string(),
                        &item,
                        decision,
                        decided_at as f64,
                    );
                    if let Err(diagnostic) = self.record_agent_approval_history(history_item).await
                    {
                        eprintln!("agent-approval-history-write-failed: {diagnostic}");
                    }
                }
            }
            self.retry_unconfirmed_approval_history(&room_id, event_id.as_str())
                .await;
            return Ok(NativeAgentApprovalDecisionResult {
                room_id,
                event_id: event_id.to_string(),
                status: plan.status,
                reaction: None,
            });
        }

        let reaction_key = plan
            .reaction
            .ok_or("agent-approval-plan-inconsistent")?
            .to_owned();
        // Capture all fallible local state before the network side effect. Once
        // Matrix accepts the reaction, completed memory is recorded
        // synchronously before any further cancellation point.
        let own_user_id = current_user_id;
        let send_event_id = event_id.clone();
        let send_reaction_key = reaction_key.clone();
        let completed_decisions = Arc::clone(&self.approval_decisions);
        // Tokio tasks continue when their JoinHandle is dropped. Moving the
        // owned per-event guard into this task makes the Matrix side effect
        // cancellation-safe: if iOS ends the callback after send begins, a
        // duplicate waits until the original send either records completion
        // or fails, instead of racing a second reaction.
        let send_task = tokio::spawn(async move {
            let _decision_guard = decision_guard;
            let sent = room
                .send(ReactionEventContent::from(Annotation::new(
                    send_event_id,
                    send_reaction_key,
                )))
                .await
                .map_err(|_| "v-send.2-reaction-ensure-failed")?;
            completed_decisions
                .lock()
                .map_err(|_| "agent-approval-decision-state-poisoned")?
                .remember(decision_key);
            Ok::<String, &'static str>(sent.response.event_id.to_string())
        });
        let sent_event_id = timeout(AGENT_APPROVAL_SIDE_EFFECT_TIMEOUT, send_task)
            .await
            .map_err(|_| "v-send.2-reaction-ensure-timeout")?
            .map_err(|_| "v-send.2-reaction-ensure-failed")??;
        let readback =
            approval_reaction_readback(&item.reactions, &reaction_key, &own_user_id, sent_event_id);
        if let Some(decision) =
            SynaraAgentApprovalHistoryDecision::from_action_id(&request.action_id)
        {
            if let Ok(decided_at) = agent_approval_now_ms() {
                let history_item = native_agent_approval_history_item(
                    room_id.clone(),
                    event_id.to_string(),
                    &item,
                    decision,
                    decided_at as f64,
                );
                if let Err(diagnostic) = self.record_agent_approval_history(history_item).await {
                    eprintln!("agent-approval-history-write-failed: {diagnostic}");
                }
            }
        }
        Ok(NativeAgentApprovalDecisionResult {
            room_id: room_id.clone(),
            event_id: event_id.to_string(),
            status: plan.status,
            reaction: Some(NativeReactionMutationResult {
                room_id,
                target_event_id: event_id.to_string(),
                key: reaction_key,
                mutation: NativeReactionMutation::Added,
                readback: Some(readback),
            }),
        })
    }

    pub async fn redact_reaction(
        &self,
        room_id: &str,
        target_event_id: &str,
        reaction_event_id: &str,
        key: &str,
    ) -> Result<NativeReactionMutationResult, &'static str> {
        redact_reaction_in(
            &mut Slots::Shared(self),
            &self.client,
            room_id,
            target_event_id,
            reaction_event_id,
            key,
        )
        .await
    }

    pub async fn open_at(
        &self,
        request: NativeTimelineOpenRequest,
    ) -> Result<NativeTimelineOpenReadback, &'static str> {
        // A routed event may need a homeserver fetch. Resolve it before taking
        // the registry lock so a slow `/event` does not stall other rooms.
        let focus_route = match &request.position {
            NativeTimelineOpenPosition::Focused { event_id } => {
                self.event_route(&request.room_id, event_id).await.ok()
            }
            _ => None,
        };
        // Builds, the sparse-history probe and `/context` run between short
        // registry lock scopes, so opening one room never stalls another
        // room, a snapshot or a media load.
        open_view(
            &mut Slots::Shared(self),
            &self.approval_history,
            self.emit.clone(),
            &self.client,
            request,
            focus_route,
        )
        .await
    }

    /// The build gate for one room's live timeline.
    fn live_build_gate(&self, room_id: &str) -> Arc<AsyncMutex<()>> {
        let mut builds = self
            .live_builds
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        builds.retain(|_, gate| gate.strong_count() > 0);
        if let Some(gate) = builds.get(room_id).and_then(Weak::upgrade) {
            return gate;
        }
        let gate = Arc::new(AsyncMutex::new(()));
        builds.insert(room_id.to_owned(), Arc::downgrade(&gate));
        gate
    }

    /// Where a routed event lives: the room timeline, a thread, or nowhere
    /// this account can see. Reads the event cache or fetches the single
    /// event; it never builds an event-focused timeline, whose SDK cache is
    /// memoized per event and would pin whatever context size opened it first.
    pub async fn event_route(
        &self,
        room_id: &str,
        event_id: &str,
    ) -> Result<NativeTimelineEventRoute, &'static str> {
        let room_id = parse_room_id(room_id)?;
        let event_id = parse_event_id(event_id)?;
        let room = self
            .client
            .get_room(&room_id)
            .ok_or("d0.3-timeline-room-not-found")?;
        Ok(resolve_event_route(&room, &event_id).await)
    }

    #[cfg(test)]
    pub async fn debug_stream_is_threaded(&self, stream_id: &str) -> Option<bool> {
        self.registry
            .lock()
            .await
            .view_streams
            .get(stream_id)
            .inspect(|stream| stream.last_access.touch())
            .map(|stream| stream.timeline.is_threaded())
    }

    #[cfg(test)]
    pub async fn debug_live_is_threaded(&self, room_id: &str) -> Option<bool> {
        self.registry
            .lock()
            .await
            .entries
            .get(room_id)
            .inspect(|entry| entry.last_used.touch())
            .map(|entry| entry.timeline.is_threaded())
    }

    /// Open the room's live bottom, then close the stream it replaces. A
    /// failed open keeps the old stream, so the caller is never left
    /// without one.
    pub async fn jump_latest(
        &self,
        request: NativeTimelineJumpLatestRequest,
    ) -> Result<NativeTimelineOpenReadback, &'static str> {
        let room_id = self.registry.lock().await.jump_latest_room(&request)?;
        let opened = self
            .open_at(NativeTimelineOpenRequest {
                room_id,
                position: NativeTimelineOpenPosition::LiveBottom,
            })
            .await?;
        self.registry
            .lock()
            .await
            .close_view(NativeTimelineCloseRequest {
                stream_id: request.stream_id,
            });
        Ok(opened)
    }

    /// The published rows and metadata of one stream. The registry lock
    /// covers only the lookup; the snapshot copies published rows.
    pub async fn snapshot(&self, stream_id: &str) -> Result<TimelineViewSnapshot, &'static str> {
        let view = self.registry.lock().await.published_view(stream_id)?;
        Ok(view.snapshot())
    }

    /// Request SDK-owned decryption on this exact open view. Completion only
    /// acknowledges the request; decrypted rows arrive through its live stream.
    pub async fn retry_decryption(&self, stream_id: &str) -> Result<bool, &'static str> {
        let timeline = {
            let registry = self.registry.lock().await;
            registry
                .view_streams
                .get(stream_id)
                .ok_or("v-timeline-view-not-open")?
                .claim()?
                .timeline
                .clone()
        };
        let session_ids: std::collections::BTreeSet<String> = timeline
            .items()
            .await
            .iter()
            .filter_map(|item| {
                let event = item.as_event()?;
                match event.content().as_unable_to_decrypt()? {
                    EncryptedMessage::MegolmV1AesSha2 { session_id, .. } => {
                        Some(session_id.clone())
                    }
                    _ => None,
                }
            })
            .collect();
        timeline.retry_decryption(session_ids).await;
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)] // Host boundary mirrors the typed Matrix send contract.
    pub async fn send_text(
        &self,
        room_id: String,
        body: String,
        msg_type: Option<String>,
        formatted_body: Option<String>,
        mention_user_ids: Option<Vec<String>>,
        mention_room: Option<bool>,
        reply_to: Option<String>,
        thread_root: Option<String>,
        txn_id: Option<String>,
    ) -> Result<MatrixSendTextResult, &'static str> {
        let parsed_room = parse_send_room_id(&room_id)?;
        let reply_to = parse_reply_event_id(reply_to)?;
        let thread_root = parse_thread_root_event_id(thread_root)?;
        let client_txn_id = parse_transaction_id(txn_id)?.map(|txn_id| txn_id.to_string());
        let content = message_content(
            body.clone(),
            msg_type,
            formatted_body,
            mention_user_ids,
            mention_room.unwrap_or(false),
            reply_to,
            thread_root,
        )?;
        let room = self
            .client
            .get_room(&parsed_room)
            .ok_or("d0.4-send-room-not-found")?;
        let room_id = parsed_room.to_string();

        let _enqueue = self.send_enqueue.lock().await;
        if let Some(client_txn_id) = client_txn_id.as_deref() {
            if let Some(existing) = self.repeated_text_send(&room_id, client_txn_id).await? {
                return Ok(existing);
            }
        }
        // Return once the SDK's persisted send queue holds the request: from
        // here it survives a relaunch and the SDK retries it. The timeline
        // row shows Sending / Failed with Retry and Discard (by this txn).
        let session = enqueue_event_via_room_queue(&room, content.into())
            .await
            .map_err(|error| error.diagnostic_id)?;
        let local_txn_id = session.transaction_id.clone();
        {
            let mut sends = self.sends.lock().await;
            sends
                .enqueue_text_with_txn(room_id.clone(), body, local_txn_id.clone())
                .map_err(|error| error.diagnostic_id())?;
            if let Some(client_txn_id) = client_txn_id {
                let _ = sends.set_client_txn_id(&local_txn_id, client_txn_id);
            }
        }
        spawn_queued_text_follower(Arc::clone(&self.sends), session);
        Ok(MatrixSendTextResult {
            room_id,
            event_id: String::new(),
            local_txn_id,
            status: crate::dto::SendStatus::Queued,
        })
    }

    /// The outcome of a send this session already enqueued for the caller's
    /// transaction id (or for an SDK transaction id passed back), so a retry
    /// or a second flush never creates a second event. `None` when unknown.
    async fn repeated_text_send(
        &self,
        room_id: &str,
        client_txn_id: &str,
    ) -> Result<Option<MatrixSendTextResult>, &'static str> {
        let sends = self.sends.lock().await;
        let existing = sends
            .find_by_client_txn(room_id, client_txn_id)
            .or_else(|| {
                sends
                    .get(client_txn_id)
                    .filter(|item| item.room_id == room_id)
            });
        let Some(existing) = existing else {
            return Ok(None);
        };
        let (event_id, status) = match existing.state {
            LocalEchoState::Sent => (
                existing.event_id.clone().unwrap_or_default(),
                crate::dto::SendStatus::Sent,
            ),
            LocalEchoState::Cancelled => return Err("d0.4-send-queue-cancelled"),
            // Still the SDK's request: its row offers Retry and Discard.
            LocalEchoState::Sending | LocalEchoState::Failed | LocalEchoState::Wedged => {
                (String::new(), crate::dto::SendStatus::Queued)
            }
        };
        Ok(Some(MatrixSendTextResult {
            room_id: room_id.to_owned(),
            event_id,
            local_txn_id: existing.local_txn_id.clone(),
            status,
        }))
    }

    /// Enqueue and wait for the homeserver (edits, whose callers replace the
    /// row with the acknowledged event). Composer sends use `send_text`,
    /// which returns once the request is queued.
    async fn send_text_via_queue(
        &self,
        room: matrix_sdk::Room,
        room_id: String,
        body: String,
        content: RoomMessageEventContent,
    ) -> Result<MatrixSendTextResult, &'static str> {
        let mut session = enqueue_event_via_room_queue(&room, content.into())
            .await
            .map_err(|error| error.diagnostic_id)?;
        let local_txn_id = session.transaction_id.clone();
        {
            let mut sends = self.sends.lock().await;
            sends
                .enqueue_text_with_txn(room_id.clone(), body, local_txn_id.clone())
                .map_err(|error| error.diagnostic_id())?;
        }
        let send_result = wait_for_queued_send(&mut session).await;
        {
            let mut sends = self.sends.lock().await;
            match &send_result {
                Ok(_) => {
                    let _ = sends.mark_sent(&local_txn_id);
                }
                Err(error) if error.wedged => {
                    let _ = sends.mark_wedged(&local_txn_id, error.diagnostic_id);
                }
                Err(error) if error.cancelled => {
                    let _ = sends.cancel(&local_txn_id);
                }
                // Still persisted in the SDK queue: it stays `Sending` and the
                // timeline row shows the SDK send state (with Discard).
                Err(error) if error.still_queued => {}
                Err(error) => {
                    let _ = sends.mark_failed(&local_txn_id, error.diagnostic_id);
                }
            }
        }
        if matches!(&send_result, Err(error) if error.still_queued) {
            // Reporting this as a failure made the composer keep the text, and
            // a second press duplicated the message once the SDK retry landed.
            return Ok(MatrixSendTextResult {
                room_id,
                event_id: String::new(),
                local_txn_id,
                status: crate::dto::SendStatus::Queued,
            });
        }
        let ack = send_result.map_err(|error| error.diagnostic_id)?;
        Ok(MatrixSendTextResult {
            room_id,
            event_id: ack.event_id,
            local_txn_id,
            status: crate::dto::SendStatus::Sent,
        })
    }

    pub async fn unwedge_send(
        &self,
        room_id: &str,
        local_txn_id: &str,
    ) -> Result<(), &'static str> {
        let parsed_room = parse_send_room_id(room_id)?;
        let room = self
            .client
            .get_room(&parsed_room)
            .ok_or("d0.4-send-room-not-found")?;
        unwedge_queued_send(&room, local_txn_id)
            .await
            .map_err(|error| error.diagnostic_id)?;
        let mut sends = self.sends.lock().await;
        let _ = sends.retry(local_txn_id);
        Ok(())
    }

    pub async fn abort_send(
        &self,
        room_id: &str,
        local_txn_id: &str,
    ) -> Result<bool, &'static str> {
        let parsed_room = parse_send_room_id(room_id)?;
        let room = self
            .client
            .get_room(&parsed_room)
            .ok_or("d0.4-send-room-not-found")?;
        let aborted = abort_queued_send(&room, local_txn_id)
            .await
            .map_err(|error| error.diagnostic_id)?;
        // `false` means the SDK had already sent the request. The product
        // queue must not record a cancel for a message that went out.
        if aborted {
            let mut sends = self.sends.lock().await;
            let _ = sends.cancel(local_txn_id);
        }
        Ok(aborted)
    }

    /// Retry one local echo without allocating a new transaction id.
    ///
    /// Wedged requests (`SendingFailed` with `is_recoverable: false`) go through
    /// [`unwedge_queued_send`]. A recoverable failure that is still queued uses
    /// the same room-queue re-enable path as [`wait_for_queued_send`].
    pub async fn retry_send(&self, room_id: &str, local_txn_id: &str) -> Result<(), &'static str> {
        if parse_transaction_id(Some(local_txn_id.to_owned()))?.is_none() {
            return Err("d0.4-send-invalid-transaction-id");
        }
        let parsed_room = parse_send_room_id(room_id)?;
        let room = self
            .client
            .get_room(&parsed_room)
            .ok_or("d0.4-send-room-not-found")?;
        if queued_send_is_wedged(&room, local_txn_id)
            .await
            .map_err(|error| error.diagnostic_id)?
        {
            unwedge_queued_send(&room, local_txn_id)
                .await
                .map_err(|error| error.diagnostic_id)?;
        } else {
            reenable_queued_send(&room, local_txn_id)
                .await
                .map_err(|error| error.diagnostic_id)?;
        }
        let mut sends = self.sends.lock().await;
        let _ = sends.retry(local_txn_id);
        Ok(())
    }

    pub async fn outbound_text_for_room(
        &self,
        room_id: &str,
    ) -> Vec<crate::app::send::OutboundTextMessage> {
        self.sends
            .lock()
            .await
            .list_for_room(room_id)
            .into_iter()
            .cloned()
            .collect()
    }

    pub async fn send_poll(
        &self,
        room_id: String,
        question: String,
        answers: Vec<String>,
        max_selections: u32,
        thread_root: Option<String>,
        reply_to: Option<String>,
    ) -> Result<MatrixSendPollResult, &'static str> {
        let parsed_room = parse_send_room_id(&room_id)?;
        let thread_root = parse_thread_root_event_id(thread_root)?;
        let reply_to = parse_reply_event_id(reply_to)?;
        let normalized =
            normalize_poll(&question, &answers, max_selections).map_err(|e| e.diagnostic_id())?;
        let mut content = poll_start_content(&normalized).map_err(|e| e.diagnostic_id())?;
        apply_poll_start_relations(&mut content, reply_to, thread_root);
        let room = self
            .client
            .get_room(&parsed_room)
            .ok_or("v-send.3-poll-room-not-found")?;
        let outcome = queued_send_outcome(
            send_event_via_room_queue(&room, UnstablePollStartEventContent::New(content).into())
                .await,
        )
        .map_err(|error| error.diagnostic_id)?;
        Ok(MatrixSendPollResult {
            room_id: parsed_room.to_string(),
            event_id: outcome.event_id(),
            status: outcome.send_status(),
        })
    }

    pub async fn poll_respond(
        &self,
        room_id: String,
        poll_event_id: String,
        answer_ids: Vec<String>,
    ) -> Result<MatrixPollRespondResult, &'static str> {
        let parsed_room = parse_send_room_id(&room_id)?;
        let content =
            poll_response_content(&poll_event_id, &answer_ids).map_err(|e| e.diagnostic_id())?;
        let room = self
            .client
            .get_room(&parsed_room)
            .ok_or("v-send.3-poll-room-not-found")?;
        let outcome = queued_send_outcome(send_event_via_room_queue(&room, content.into()).await)
            .map_err(|error| error.diagnostic_id)?;
        Ok(MatrixPollRespondResult {
            room_id: parsed_room.to_string(),
            poll_event_id,
            event_id: outcome.event_id(),
            status: outcome.send_status(),
        })
    }

    #[allow(clippy::too_many_arguments)] // Host boundary mirrors the typed Matrix edit contract.
    pub async fn edit_message(
        &self,
        room_id: String,
        event_id: String,
        body: String,
        msg_type: Option<String>,
        formatted_body: Option<String>,
        mention_user_ids: Option<Vec<String>>,
        mention_room: Option<bool>,
        txn_id: Option<String>,
    ) -> Result<MatrixSendTextResult, &'static str> {
        let parsed_room = parse_send_room_id(&room_id)?;
        let parsed_event = parse_edit_event_id(&event_id)?;
        let _txn_id = parse_transaction_id(txn_id)?;
        let content = edit_message_content(
            body.clone(),
            msg_type,
            formatted_body,
            mention_user_ids,
            mention_room.unwrap_or(false),
            parsed_event,
        )?;
        let room = self
            .client
            .get_room(&parsed_room)
            .ok_or("v-send.r-edit-room-not-found")?;
        self.send_text_via_queue(room, parsed_room.to_string(), body, content)
            .await
            .map_err(|diagnostic| {
                if is_wedged_send_diagnostic(diagnostic) {
                    diagnostic
                } else if diagnostic.starts_with("d0.4-") || diagnostic.starts_with("p6.1-") {
                    "v-send.r-edit-sdk-failed"
                } else {
                    diagnostic
                }
            })
    }

    pub async fn edit_text(
        &self,
        room_id: &str,
        event_id: &str,
        body: &str,
        formatted_body: Option<&str>,
    ) -> Result<NativeTimelineActionReadback, &'static str> {
        let room_id = parse_action_room_id(room_id)?;
        let event_id = parse_action_event_id(event_id, "v-timeline-edit-invalid-event-id")?;
        let body = body.trim();
        if body.is_empty() {
            return Err("v-timeline-edit-empty-body");
        }
        let formatted_body = normalize_edit_formatted_body(body, formatted_body)?;
        let room = self
            .client
            .get_room(&room_id)
            .ok_or("v-timeline-edit-room-not-found")?;
        let new_content = match formatted_body {
            Some(html) => RoomMessageEventContentWithoutRelation::text_html(body.to_owned(), html),
            None => RoomMessageEventContentWithoutRelation::text_plain(body.to_owned()),
        };
        let edit_content = room
            .make_edit_event(&event_id, EditedContent::RoomMessage(new_content))
            .await
            .map_err(|_| "v-timeline-edit-prepare-failed")?;
        let outcome = queued_send_outcome(send_event_via_room_queue(&room, edit_content).await)
            .map_err(|_| "v-timeline-edit-send-failed")?;
        Ok(NativeTimelineActionReadback {
            schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
            action: NativeTimelineActionKind::EditText,
            room_id: room_id.to_string(),
            event_id: event_id.to_string(),
            status: outcome.status("sent").into(),
        })
    }

    pub async fn redact_event(
        &self,
        room_id: &str,
        event_id: &str,
        reason: Option<&str>,
    ) -> Result<NativeTimelineActionReadback, &'static str> {
        let room_id = parse_action_room_id(room_id)?;
        let event_id = parse_action_event_id(event_id, "v-timeline-redact-invalid-event-id")?;
        let reason = normalize_timeline_action_reason(reason, "v-timeline-redact-reason-too-long")?;
        let room = self
            .client
            .get_room(&room_id)
            .ok_or("v-timeline-redact-room-not-found")?;
        let own_user_id = self
            .client
            .user_id()
            .ok_or("v-timeline-redact-permission-denied")?;
        let timelines = {
            let registry = self.registry.lock().await;
            let mut timelines = Vec::new();
            if let Some(entry) = registry
                .entries
                .get(room_id.as_str())
                .inspect(|entry| entry.last_used.touch())
            {
                timelines.push(entry.timeline.clone());
            }
            timelines.extend(
                registry
                    .view_streams
                    .values()
                    .filter(|entry| entry.room_id == room_id.as_str())
                    .map(|entry| entry.timeline.clone()),
            );
            timelines.extend(
                registry
                    .focused_entries
                    .iter()
                    .filter(|((entry_room_id, _), _)| entry_room_id == room_id.as_str())
                    .map(|(_, timeline)| timeline.clone()),
            );
            timelines
        };
        let mut event_is_own = None;
        for timeline in timelines {
            if let Some(item) = timeline.item_by_event_id(&event_id).await {
                event_is_own = Some(item.sender() == own_user_id);
                break;
            }
        }
        let event_is_own = event_is_own.ok_or("v-timeline-redact-event-not-visible")?;
        let authority = room_action_authority(&room, Some(own_user_id)).await;
        let authorized = if event_is_own {
            authority.can_redact_own
        } else {
            authority.can_redact_other
        };
        if !authorized {
            return Err("v-timeline-redact-permission-denied");
        }
        accept_still_queued(
            redact_via_room_queue(&room, &event_id, reason.as_deref()).await,
            "v-timeline-redact-failed",
        )?;
        Ok(NativeTimelineActionReadback {
            schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
            action: NativeTimelineActionKind::Redact,
            room_id: room_id.to_string(),
            event_id: event_id.to_string(),
            status: "redacted".into(),
        })
    }

    pub async fn report(
        &self,
        room_id: &str,
        event_id: &str,
        reason: Option<&str>,
    ) -> Result<NativeTimelineActionReadback, &'static str> {
        let room_id = parse_action_room_id(room_id)?;
        let event_id = parse_action_event_id(event_id, "v-timeline-report-invalid-event-id")?;
        let reason = normalize_timeline_action_reason(reason, "v-timeline-report-reason-too-long")?;
        let room = self
            .client
            .get_room(&room_id)
            .ok_or("v-timeline-report-room-not-found")?;
        room.report_content(event_id.clone(), reason)
            .await
            .map_err(|_| "v-timeline-report-failed")?;
        Ok(NativeTimelineActionReadback {
            schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
            action: NativeTimelineActionKind::Report,
            room_id: room_id.to_string(),
            event_id: event_id.to_string(),
            status: "reported".into(),
        })
    }

    pub async fn pin_event(
        &self,
        room_id: &str,
        event_id: &str,
    ) -> Result<NativeTimelineActionReadback, &'static str> {
        self.set_pinned(room_id, event_id, true).await
    }

    pub async fn unpin_event(
        &self,
        room_id: &str,
        event_id: &str,
    ) -> Result<NativeTimelineActionReadback, &'static str> {
        self.set_pinned(room_id, event_id, false).await
    }

    pub async fn pinned_events_snapshot(
        &self,
        room_id: &str,
    ) -> Result<PinnedEventsSnapshot, &'static str> {
        let room_id = parse_action_room_id(room_id)?;
        let room = self
            .client
            .get_room(&room_id)
            .ok_or("v-timeline-pinned-room-not-found")?;
        let mut media = self.pin_media.lock().await;
        snapshot_pinned_events(&room, &mut media).await
    }

    async fn set_pinned(
        &self,
        room_id: &str,
        event_id: &str,
        pin: bool,
    ) -> Result<NativeTimelineActionReadback, &'static str> {
        let room_id = parse_action_room_id(room_id)?;
        let event_id = parse_action_event_id(
            event_id,
            if pin {
                "v-timeline-pin-invalid-event-id"
            } else {
                "v-timeline-unpin-invalid-event-id"
            },
        )?;
        let room = self.client.get_room(&room_id).ok_or(if pin {
            "v-timeline-pin-room-not-found"
        } else {
            "v-timeline-unpin-room-not-found"
        })?;
        if !user_can_pin_events(&room, self.client.user_id()).await {
            return Err(if pin {
                "v-timeline-pin-permission-denied"
            } else {
                "v-timeline-unpin-permission-denied"
            });
        }
        let changed = if pin {
            room.pin_event(&event_id)
                .await
                .map_err(|_| "v-timeline-pin-failed")?
        } else {
            room.unpin_event(&event_id)
                .await
                .map_err(|_| "v-timeline-unpin-failed")?
        };
        Ok(NativeTimelineActionReadback {
            schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
            action: if pin {
                NativeTimelineActionKind::Pin
            } else {
                NativeTimelineActionKind::Unpin
            },
            room_id: room_id.to_string(),
            event_id: event_id.to_string(),
            status: if changed {
                if pin {
                    "pinned"
                } else {
                    "unpinned"
                }
            } else if pin {
                "already_pinned"
            } else {
                "already_unpinned"
            }
            .to_owned(),
        })
    }

    pub async fn poll_vote(
        &self,
        room_id: &str,
        event_id: &str,
        answer_ids: Vec<String>,
    ) -> Result<NativeTimelineActionReadback, &'static str> {
        let room_id = parse_action_room_id(room_id)?;
        let event_id = parse_action_event_id(event_id, "v-timeline-poll-vote-invalid-event-id")?;
        let room = self
            .client
            .get_room(&room_id)
            .ok_or("v-timeline-poll-vote-room-not-found")?;
        // Validate against the same SDK timeline object that projected the
        // visible poll/capability. Never trust presenter-supplied option IDs or
        // selection bounds, and never send a response after an observed close.
        let timelines = {
            let registry = self.registry.lock().await;
            let mut timelines = Vec::new();
            if let Some(entry) = registry
                .entries
                .get(room_id.as_str())
                .inspect(|entry| entry.last_used.touch())
            {
                timelines.push(entry.timeline.clone());
            }
            timelines.extend(
                registry
                    .view_streams
                    .values()
                    .filter(|entry| entry.room_id == room_id.as_str())
                    .map(|entry| entry.timeline.clone()),
            );
            timelines.extend(
                registry
                    .focused_entries
                    .iter()
                    .filter(|((entry_room_id, _), _)| entry_room_id == room_id.as_str())
                    .map(|(_, timeline)| timeline.clone()),
            );
            timelines
        };
        let mut poll_definitions = Vec::new();
        for timeline in timelines {
            let Some(item) = timeline.item_by_event_id(&event_id).await else {
                continue;
            };
            if let SdkTimelineItemContent::MsgLike(content) = item.content() {
                if let MsgLikeKind::Poll(poll) = &content.kind {
                    let results = poll.results();
                    poll_definitions.push((
                        results
                            .answers
                            .into_iter()
                            .map(|answer| answer.id)
                            .collect::<HashSet<_>>(),
                        results.max_selections,
                        results.end_time.is_some(),
                    ));
                }
            }
        }
        let (available_answer_ids, max_selections, _) = poll_definitions
            .first()
            .ok_or("v-timeline-poll-vote-poll-not-visible")?;
        if poll_definitions.iter().any(|(_, _, closed)| *closed) {
            return Err("v-timeline-poll-vote-closed");
        }
        if poll_definitions
            .iter()
            .any(|(answers, max, _)| answers != available_answer_ids || max != max_selections)
        {
            return Err("v-timeline-poll-vote-state-conflict");
        }
        let answer_ids = validate_poll_vote_selection(
            answer_ids,
            available_answer_ids,
            usize::try_from(*max_selections)
                .map_err(|_| "v-timeline-poll-vote-selection-bound-invalid")?,
            false,
        )?;
        let content = poll_response_content(event_id.as_str(), &answer_ids)
            .map_err(|_| "v-timeline-poll-vote-invalid-answer")?;
        let outcome = queued_send_outcome(send_event_via_room_queue(&room, content.into()).await)
            .map_err(|_| "v-timeline-poll-vote-send-failed")?;
        Ok(NativeTimelineActionReadback {
            schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
            action: NativeTimelineActionKind::PollVote,
            room_id: room_id.to_string(),
            event_id: event_id.to_string(),
            status: outcome.status("voted").into(),
        })
    }

    pub async fn decline_call(
        &self,
        room_id: &str,
        event_id: &str,
    ) -> Result<NativeTimelineActionReadback, &'static str> {
        let room_id = parse_action_room_id(room_id)?;
        let event_id = parse_action_event_id(event_id, "v-timeline-call-decline-invalid-event-id")?;
        let room = self
            .client
            .get_room(&room_id)
            .ok_or("v-timeline-call-decline-room-not-found")?;
        let content =
            room.make_decline_call_event(&event_id)
                .await
                .map_err(|error| match error {
                    CallError::DeclineOwnCall => "v-timeline-call-decline-own-call",
                    CallError::BadEventType => "v-timeline-call-decline-bad-event-type",
                    _ => "v-timeline-call-decline-prepare-failed",
                })?;
        room.send(content)
            .await
            .map_err(|_| "v-timeline-call-decline-send-failed")?;
        Ok(NativeTimelineActionReadback {
            schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
            action: NativeTimelineActionKind::CallDecline,
            room_id: room_id.to_string(),
            event_id: event_id.to_string(),
            status: "declined".into(),
        })
    }

    pub async fn forward_text(
        &self,
        source_room_id: &str,
        event_id: &str,
        target_room_id: &str,
        as_quote: bool,
        confirmed_encryption_downgrade: bool,
    ) -> Result<NativeTimelineActionReadback, &'static str> {
        let source_room_id = parse_action_room_id(source_room_id)?;
        let target_room_id = parse_action_room_id(target_room_id)?;
        let event_id = parse_action_event_id(event_id, "v-timeline-forward-invalid-event-id")?;
        let source_room = self
            .client
            .get_room(&source_room_id)
            .ok_or("v-timeline-forward-source-room-not-found")?;
        let target_room = self
            .client
            .get_room(&target_room_id)
            .ok_or("v-timeline-forward-target-room-not-found")?;
        validate_forward_encryption(&source_room, &target_room, confirmed_encryption_downgrade)
            .await?;
        let (sender_label, body) = load_forwardable_text(&source_room, &event_id).await?;
        let forwarded_body = format_forwarded_plain_body(&sender_label, &body, as_quote);
        let mut content = RoomMessageEventContent::text_plain(forwarded_body);
        content.mentions = Some(Mentions::new());
        let outcome =
            queued_send_outcome(send_event_via_room_queue(&target_room, content.into()).await)
                .map_err(|_| "v-timeline-forward-send-failed")?;
        Ok(NativeTimelineActionReadback {
            schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
            action: NativeTimelineActionKind::ForwardText,
            room_id: target_room_id.to_string(),
            event_id: outcome.event_id(),
            status: outcome.status("sent").into(),
        })
    }

    pub async fn forward_media(
        &self,
        source_room_id: &str,
        event_id: &str,
        target_room_id: &str,
        confirmed_encryption_downgrade: bool,
    ) -> Result<NativeTimelineActionReadback, &'static str> {
        let source_room_id = parse_action_room_id(source_room_id)?;
        let target_room_id = parse_action_room_id(target_room_id)?;
        let event_id =
            parse_action_event_id(event_id, "v-timeline-forward-media-invalid-event-id")?;
        let source_room = self
            .client
            .get_room(&source_room_id)
            .ok_or("v-timeline-forward-media-source-room-not-found")?;
        let target_room = self
            .client
            .get_room(&target_room_id)
            .ok_or("v-timeline-forward-media-target-room-not-found")?;
        validate_forward_encryption(&source_room, &target_room, confirmed_encryption_downgrade)
            .await?;
        let content = load_forwardable_media(&source_room, &event_id).await?;
        let outcome = queued_send_outcome(send_event_via_room_queue(&target_room, content).await)
            .map_err(|_| "v-timeline-forward-media-send-failed")?;
        Ok(NativeTimelineActionReadback {
            schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
            action: NativeTimelineActionKind::ForwardMedia,
            room_id: target_room_id.to_string(),
            event_id: outcome.event_id(),
            status: outcome.status("sent").into(),
        })
    }

    pub async fn set_reply_draft(
        &self,
        room_id: &str,
        event_id: &str,
        start_thread: bool,
    ) -> Result<NativeComposerReplyDraftReadback, &'static str> {
        let room_id = parse_action_room_id(room_id)?;
        let event_id = parse_action_event_id(event_id, "v-timeline-reply-draft-invalid-event-id")?;
        let room = self
            .client
            .get_room(&room_id)
            .ok_or("v-timeline-reply-draft-room-not-found")?;
        let draft = load_reply_draft_preview(&room, &event_id, start_thread).await?;
        let room_id_string = room_id.to_string();
        let draft = self.drafts.lock().await.set(room_id_string.clone(), draft);
        Ok(reply_draft_readback(
            room_id_string,
            crate::dto::ComposerDraftStatus::Set,
            Some(draft),
        ))
    }

    pub async fn clear_reply_draft(
        &self,
        room_id: &str,
        expected_draft_revision: u64,
        thread_root_event_id: Option<&str>,
    ) -> Result<NativeComposerReplyDraftReadback, &'static str> {
        let room_id = parse_action_room_id(room_id)?;
        let thread_root = optional_draft_thread_root(thread_root_event_id)?;
        let room_id_string = room_id.to_string();
        let superseding_draft = self.drafts.lock().await.compare_and_clear(
            &room_id_string,
            thread_root,
            expected_draft_revision,
        );
        Ok(match superseding_draft {
            Some(draft) => reply_draft_readback(
                room_id_string,
                crate::dto::ComposerDraftStatus::Set,
                Some(draft),
            ),
            None => reply_draft_readback(
                room_id_string,
                crate::dto::ComposerDraftStatus::Cleared,
                None,
            ),
        })
    }

    pub async fn get_reply_draft(
        &self,
        room_id: &str,
        thread_root_event_id: Option<&str>,
    ) -> Result<NativeComposerReplyDraftReadback, &'static str> {
        let room_id = parse_action_room_id(room_id)?;
        let thread_root = optional_draft_thread_root(thread_root_event_id)?;
        let room_id_string = room_id.to_string();
        let draft = self
            .drafts
            .lock()
            .await
            .get(&room_id_string, thread_root)
            .cloned();
        Ok(reply_draft_readback(
            room_id_string,
            if draft.is_some() {
                crate::dto::ComposerDraftStatus::Set
            } else {
                crate::dto::ComposerDraftStatus::Empty
            },
            draft,
        ))
    }

    pub async fn thread_list(
        &self,
        room_id: &str,
        action: &str,
    ) -> Result<NativeThreadListSnapshot, &'static str> {
        let room_id = parse_action_room_id(room_id).map_err(|_| "v-thread-list-invalid-room-id")?;
        let room_id_string = room_id.to_string();
        match action {
            "close" => {
                self.thread_lists.lock().await.remove(&room_id_string);
                self.thread_index.lock().await.clear_room(&room_id_string);
                Ok(NativeThreadListSnapshot::empty(room_id_string))
            }
            "open" | "paginate" => {
                use matrix_sdk_ui::timeline::RoomExt;
                let room = self
                    .client
                    .get_room(&room_id)
                    .ok_or("v-thread-list-room-not-found")?;
                let mut lists = self.thread_lists.lock().await;
                if !lists.contains_key(&room_id_string) {
                    lists.insert(room_id_string.clone(), room.thread_list_service());
                }
                let service = lists
                    .get(&room_id_string)
                    .expect("thread list service present");
                if action == "paginate" || service.items().is_empty() {
                    service
                        .paginate()
                        .await
                        .map_err(|_| "v-thread-list-paginate-failed")?;
                }
                let end_reached = matches!(
                    service.pagination_state(),
                    ThreadListPaginationState::Idle { end_reached: true }
                );
                let projections = service.items().into_iter().map(|item| {
                    let latest = item.latest_event.as_ref();
                    ThreadListItemProjection {
                        root_event_id: item.root_event.event_id.to_string(),
                        reply_count: item.num_replies,
                        latest_event_id: latest.map(|event| event.event_id.to_string()),
                        latest_origin_server_ts: latest
                            .map(|event| u64::from(event.timestamp.get()))
                            .or(Some(u64::from(item.root_event.timestamp.get()))),
                        participated: item.root_event.is_own
                            || latest.map(|event| event.is_own).unwrap_or(false),
                    }
                });
                let mut index = self.thread_index.lock().await;
                let (mut threads, truncated) =
                    rebuild_thread_index(&mut index, &room_id_string, projections);
                drop(index);
                drop(lists);
                for summary in &mut threads {
                    summary.unread_count =
                        thread_unread_count(&self.client, &room_id, &summary.root_event_id).await;
                }
                Ok(NativeThreadListSnapshot {
                    schema_version: crate::app::threads::NATIVE_THREAD_LIST_SCHEMA_VERSION,
                    room_id: room_id_string,
                    threads,
                    end_reached,
                    truncated,
                })
            }
            _ => Err("v-thread-list-invalid-action"),
        }
    }
}

impl NativeTimelineRegistry {
    pub fn new(session_generation: u64) -> Self {
        Self {
            approval_history: Arc::new(ApprovalHistory::default()),
            media_index: Arc::new(StreamMediaIndex::default()),
            session_generation,
            entries: HashMap::new(),
            focused_entries: RecencyMap::new(MAX_FOCUSED_TIMELINES),
            thread_entries: RecencyMap::new(MAX_FOCUSED_TIMELINES),
            view_streams: HashMap::new(),
            view_update_tasks: HashMap::new(),
            next_view_stream_id: 0,
            unclaimed_stream_ttl: None,
            utd_index: UtdIndex::new(session_generation),
            utd_recovery: UtdRecoveryCoordinator::new(session_generation),
        }
    }

    /// Drop the least recently used idle live timelines above
    /// [`MAX_LIVE_TIMELINES`]. A room with an open view stream, and the room
    /// that was just opened, are never evicted.
    fn evict_idle_live_entries(&mut self, keep_room_id: &str) {
        while self.entries.len() > MAX_LIVE_TIMELINES {
            let candidate = self
                .entries
                .iter()
                .filter(|(room_id, _)| {
                    room_id.as_str() != keep_room_id
                        && !self
                            .view_streams
                            .values()
                            .any(|stream| &stream.room_id == *room_id)
                })
                .min_by_key(|(_, entry)| entry.last_used.get())
                .map(|(room_id, _)| room_id.clone());
            let Some(room_id) = candidate else {
                break;
            };
            self.entries.remove(&room_id);
        }
    }

    /// Close the least recently used view streams so a new one fits under
    /// [`MAX_VIEW_STREAMS`]. Streams a reloaded renderer never closed are
    /// the ones that stop being touched.
    fn evict_least_recent_view_streams(&mut self) {
        let expired = self
            .view_streams
            .iter()
            .filter(|(_, stream)| stream.view.is_expired())
            .map(|(stream_id, _)| stream_id.clone())
            .collect::<Vec<_>>();
        for stream_id in expired {
            self.close_view(NativeTimelineCloseRequest { stream_id });
        }
        while self.view_streams.len() >= MAX_VIEW_STREAMS {
            let Some(stream_id) = self
                .view_streams
                .iter()
                .min_by_key(|(_, stream)| stream.last_access.get())
                .map(|(stream_id, _)| stream_id.clone())
            else {
                break;
            };
            self.close_view(NativeTimelineCloseRequest { stream_id });
        }
    }

    pub async fn open(
        &mut self,
        client: &Client,
        room_id: &str,
    ) -> Result<NativeTimelineSnapshot, &'static str> {
        let room_id = parse_room_id(room_id)?.to_string();
        Slots::Owned(self).live(client, &room_id).await?;
        self.snapshot(client, &room_id).await
    }

    /// The room's cached live timeline, marked as used.
    fn cached_live(&self, room_id: &str) -> Option<Arc<Timeline>> {
        self.entries
            .get(room_id)
            .inspect(|entry| entry.last_used.touch())
            .map(|entry| entry.timeline.clone())
    }

    /// Install a live timeline built without the registry lock. If another
    /// open installed one first, keep that one so every stream of the room
    /// shares one provider. Returns the installed timeline and whether it is
    /// the one passed in.
    fn adopt_live(&mut self, room_id: &str, entry: LiveTimelineEntry) -> (Arc<Timeline>, bool) {
        if let Some(existing) = self.cached_live(room_id) {
            return (existing, false);
        }
        let timeline = entry.timeline.clone();
        self.entries.insert(room_id.to_owned(), entry);
        self.evict_idle_live_entries(room_id);
        (timeline, true)
    }

    fn demote_live(&self, room_id: &str) {
        if let Some(entry) = self.entries.get(room_id) {
            entry.last_used.demote();
        }
    }

    /// Install an event-focused timeline, keeping one a concurrent open
    /// installed first.
    fn adopt_focused(&mut self, key: (String, String), timeline: Arc<Timeline>) -> Arc<Timeline> {
        if let Some(existing) = self.focused_entries.get(&key) {
            return existing.clone();
        }
        self.focused_entries.insert(key, timeline.clone());
        timeline
    }

    /// Install a thread timeline, keeping one a concurrent open installed
    /// first.
    fn adopt_thread(&mut self, key: (String, String), timeline: Arc<Timeline>) -> Arc<Timeline> {
        if let Some(existing) = self.thread_entries.get(&key) {
            return existing.clone();
        }
        self.thread_entries.insert(key, timeline.clone());
        timeline
    }

    fn next_stream_number(&mut self) -> u64 {
        self.next_view_stream_id = self.next_view_stream_id.saturating_add(1);
        self.next_view_stream_id
    }

    /// Open the requested view from the native timeline owner without
    /// collapsing an event-link focus into the live-bottom route.
    pub async fn open_at(
        &mut self,
        emit: TimelineViewUpdateEmit,
        client: &Client,
        request: NativeTimelineOpenRequest,
        resolved_focus_route: Option<NativeTimelineEventRoute>,
    ) -> Result<NativeTimelineOpenReadback, &'static str> {
        let approval_history = self.approval_history.clone();
        open_view(
            &mut Slots::Owned(self),
            &approval_history,
            emit,
            client,
            request,
            resolved_focus_route,
        )
        .await
    }

    /// Publish an opened view. Deliberately synchronous: once the stream is
    /// inserted there is no cancellation point before the readback returns.
    fn publish_view(&mut self, view: PreparedView) -> NativeTimelineOpenReadback {
        let PreparedView {
            approval_history,
            stream_id,
            room_id,
            timeline,
            position,
            focus_route,
            own_user_id,
            view,
            settle_requests,
            media,
            items,
            annotations,
            hydrate_sender_profiles,
            updates,
            snapshot,
        } = view;
        self.evict_least_recent_view_streams();
        self.media_index.insert(stream_id.clone(), media.clone());
        self.view_streams.insert(
            stream_id.clone(),
            ViewStreamEntry {
                _approval_history: approval_history,
                room_id,
                timeline: timeline.clone(),
                view: view.clone(),
                last_access: UseStamp::now(),
            },
        );
        self.view_update_tasks.insert(
            stream_id.clone(),
            spawn_view_update_owner(
                ViewUpdateOwnerInput {
                    own_user_id,
                    view,
                    settle_requests,
                    media,
                    items,
                    annotations,
                    timeline,
                    hydrate_sender_profiles,
                },
                updates,
            ),
        );
        NativeTimelineOpenReadback {
            schema_version: NATIVE_TIMELINE_OPEN_SCHEMA_VERSION,
            stream_id,
            position,
            snapshot,
            focus_route,
        }
    }

    pub async fn snapshot(
        &mut self,
        client: &Client,
        room_id: &str,
    ) -> Result<NativeTimelineSnapshot, &'static str> {
        let room_id = parse_room_id(room_id)?.to_string();
        let (timeline, is_encrypted) = self
            .entries
            .get(&room_id)
            .inspect(|entry| entry.last_used.touch())
            .map(|entry| (entry.timeline.clone(), entry.is_encrypted))
            .ok_or("d0.3-timeline-not-open")?;
        let hit_start = live_hit_start(&timeline).await;
        let mut snapshot = snapshot_from_timeline(
            self.session_generation,
            room_id.clone(),
            &timeline,
            is_encrypted,
            hit_start,
            client.user_id(),
        )
        .await?;
        self.reconcile_utd(&mut snapshot, UtdRecoveryKind::RetryDecrypt);
        Ok(snapshot)
    }

    pub async fn paginate(
        &mut self,
        _client: &Client,
        request: NativeTimelineViewPaginationRequest,
    ) -> Result<TimelineViewSnapshot, &'static str> {
        paginate_view(&mut Slots::Owned(self), request).await
    }

    /// The timeline and published view of the stream a command targets,
    /// marked as read by its consumer.
    fn stream_target(
        &self,
        stream_id: &str,
    ) -> Result<(Arc<Timeline>, Arc<PublishedView>), &'static str> {
        let stream = self
            .view_streams
            .get(stream_id)
            .ok_or("v-timeline-view-not-open")?
            .claim()?;
        Ok((stream.timeline.clone(), stream.view.clone()))
    }

    /// Validate a stream read-state request and return the stream's timeline.
    pub fn read_state_timeline(
        &self,
        request: &NativeTimelineReadStateRequest,
    ) -> Result<Arc<Timeline>, &'static str> {
        Ok(self.read_state_target(request)?.0)
    }

    fn read_state_target(
        &self,
        request: &NativeTimelineReadStateRequest,
    ) -> Result<(Arc<Timeline>, Arc<PublishedView>), &'static str> {
        let (timeline, view) = self.stream_target(&request.stream_id)?;
        if request.action == NativeTimelineReadAction::MarkRead
            && !position_allows_mark_read(&view.position())
        {
            return Err("v-timeline-read-requires-live-view");
        }
        Ok((timeline, view))
    }

    /// Promote unread placement on an existing live provider without reopening.
    ///
    /// Compare-and-transition, mirroring automatic read receipts: the platform
    /// passes the exact tail event it painted at the visual bottom, and the
    /// stream position flips only when that event is still the
    /// SDK-authoritative live tail. Focused providers cannot be promoted: their
    /// loaded tail is not the room live edge and they do not follow sync.
    /// A newer arrival or focused provider fails closed with
    /// `v-timeline-follow-live-tail-not-loaded` so the client paginates
    /// forward or jumps to latest instead of claiming a live edge it cannot
    /// see. Streams already at the live bottom succeed idempotently without
    /// consulting the tail.
    pub async fn follow_live_tail(
        &mut self,
        client: &Client,
        request: NativeTimelineFollowLiveRequest,
    ) -> Result<TimelineViewSnapshot, &'static str> {
        follow_live_tail_in(&mut Slots::Owned(self), client, request).await
    }

    /// Open (or reuse) the live timeline a room-level Mark as Read acts on.
    pub async fn open_room_read_timeline(
        &mut self,
        client: &Client,
        room_id_string: &str,
    ) -> Result<Arc<Timeline>, &'static str> {
        open_room_read_timeline_in(&mut Slots::Owned(self), client, room_id_string).await
    }

    /// The published view a snapshot reads, marked as read by its consumer.
    fn published_view(&self, stream_id: &str) -> Result<Arc<PublishedView>, &'static str> {
        Ok(self.stream_target(stream_id)?.1)
    }

    pub fn view_snapshot_for_stream(
        &self,
        stream_id: &str,
    ) -> Result<TimelineViewSnapshot, &'static str> {
        Ok(self.published_view(stream_id)?.snapshot())
    }

    pub fn close_view(&mut self, request: NativeTimelineCloseRequest) -> bool {
        let removed = self.view_streams.remove(&request.stream_id).is_some();
        self.media_index.remove(&request.stream_id);
        if let Some(task) = self.view_update_tasks.remove(&request.stream_id) {
            task.abort();
        }
        removed
    }

    /// The room of the stream Jump to latest replaces.
    fn jump_latest_room(
        &self,
        request: &NativeTimelineJumpLatestRequest,
    ) -> Result<String, &'static str> {
        Ok(self
            .view_streams
            .get(&request.stream_id)
            .ok_or("v-timeline-view-not-open")?
            .claim()?
            .room_id
            .clone())
    }

    /// Rebind an opened stream to the live bottom and return a fresh stream
    /// ownership packet. The previous stream is closed only once the new one
    /// is open, so a failed open leaves the caller its old stream; on success
    /// the caller must adopt the new stream id from readback.
    pub async fn jump_latest(
        &mut self,
        emit: TimelineViewUpdateEmit,
        client: &Client,
        request: NativeTimelineJumpLatestRequest,
    ) -> Result<NativeTimelineOpenReadback, &'static str> {
        let room_id = self.jump_latest_room(&request)?;
        let opened = self
            .open_at(
                emit,
                client,
                NativeTimelineOpenRequest {
                    room_id,
                    position: NativeTimelineOpenPosition::LiveBottom,
                },
                None,
            )
            .await?;
        self.close_view(NativeTimelineCloseRequest {
            stream_id: request.stream_id,
        });
        Ok(opened)
    }

    pub async fn resolve_media(&self, handle_id: &str) -> Option<TimelineMediaSource> {
        self.media_index
            .resolve(self.session_generation, handle_id)
            .await
    }

    pub async fn paginate_legacy(
        &mut self,
        client: &Client,
        room_id: &str,
        direction: NativeTimelineDirection,
    ) -> Result<NativeTimelineSnapshot, &'static str> {
        let room_id = parse_room_id(room_id)?.to_string();
        let (timeline, is_encrypted) = self
            .entries
            .get(&room_id)
            .inspect(|entry| entry.last_used.touch())
            .map(|entry| (entry.timeline.clone(), entry.is_encrypted))
            .ok_or("d0.3-timeline-not-open")?;
        let hit_start = match direction {
            NativeTimelineDirection::Backwards => timeline
                .paginate_backwards(PAGINATION_BATCH_SIZE)
                .await
                .map_err(|_| "d0.3-timeline-paginate-backwards-failed")?,
            NativeTimelineDirection::Forwards => {
                timeline
                    .paginate_forwards(PAGINATION_BATCH_SIZE)
                    .await
                    .map_err(|_| "d0.3-timeline-paginate-forwards-failed")?;
                live_hit_start(&timeline).await
            }
        };
        let mut snapshot = snapshot_from_timeline(
            self.session_generation,
            room_id.clone(),
            &timeline,
            is_encrypted,
            hit_start,
            client.user_id(),
        )
        .await?;
        self.reconcile_utd(&mut snapshot, UtdRecoveryKind::EncryptedHistoryRecovery);
        Ok(snapshot)
    }

    pub async fn event_readback(
        &mut self,
        client: &Client,
        room_id: &str,
        event_id: &str,
    ) -> Result<NativeTimelineEventReadback, &'static str> {
        event_readback_in(&mut Slots::Owned(self), client, room_id, event_id).await
    }

    /// The only self-reaction toggle owner. `matrix-sdk-ui` owns the decision
    /// to add or redact the current user's annotation and its local echo.
    pub async fn toggle_reaction(
        &mut self,
        client: &Client,
        room_id: &str,
        target_event_id: &str,
        key: &str,
    ) -> Result<NativeReactionMutationResult, &'static str> {
        toggle_reaction_in(
            &mut Slots::Owned(self),
            client,
            room_id,
            target_event_id,
            key,
        )
        .await
    }

    /// Every timeline of a room that may render a reaction target: the live
    /// provider first, then view streams, threads and focused windows.
    fn reaction_candidates(&self, room_id: &str) -> Result<Vec<Arc<Timeline>>, &'static str> {
        let live = self
            .cached_live(room_id)
            .ok_or("v-send.2-reaction-timeline-not-open")?;
        Ok(std::iter::once(live)
            .chain(
                self.view_streams
                    .values()
                    .filter(|entry| entry.room_id == room_id)
                    .map(|entry| entry.timeline.clone()),
            )
            .chain(
                self.thread_entries
                    .iter()
                    .chain(self.focused_entries.iter())
                    .filter(|((entry_room_id, _), _)| entry_room_id == room_id)
                    .map(|(_, timeline)| timeline.clone()),
            )
            .collect())
    }

    /// See [`ensure_reaction_in`].
    pub async fn ensure_reaction(
        &mut self,
        client: &Client,
        room_id: &str,
        target_event_id: &str,
        key: &str,
    ) -> Result<NativeReactionMutationResult, &'static str> {
        ensure_reaction_in(
            &mut Slots::Owned(self),
            client,
            room_id,
            target_event_id,
            key,
        )
        .await
    }

    /// See [`redact_reaction_in`].
    pub async fn redact_reaction(
        &mut self,
        client: &Client,
        room_id: &str,
        target_event_id: &str,
        reaction_event_id: &str,
        key: &str,
    ) -> Result<NativeReactionMutationResult, &'static str> {
        redact_reaction_in(
            &mut Slots::Owned(self),
            client,
            room_id,
            target_event_id,
            reaction_event_id,
            key,
        )
        .await
    }

    /// Legacy UTD bookkeeping for the `NativeTimelineSnapshot` readbacks.
    ///
    /// Best effort: the index and the recovery coordinator are bounded and
    /// session-wide, so a full index must never fail a room open, a page or
    /// a reaction. Whatever cannot be recorded is skipped; the counts in
    /// `snapshot.utd` always come from the snapshot itself.
    fn reconcile_utd(&mut self, snapshot: &mut NativeTimelineSnapshot, kind: UtdRecoveryKind) {
        let room_id = snapshot.room_id.clone();
        let previous_active: Vec<String> = self
            .utd_index
            .list_active_for_room(&room_id)
            .iter()
            .map(|entry| entry.event_id.clone())
            .collect();

        for item in snapshot
            .items
            .iter()
            .filter(|item| item.decryption_state.is_some())
        {
            let reason = match item.decryption_state {
                Some(NativeDecryptionState::Unavailable) => UtdReasonCode::Other,
                _ => UtdReasonCode::MissingKeys,
            };
            if self
                .utd_index
                .get(&room_id, &item.event_id)
                .map(|entry| entry.reason)
                != Some(reason)
                && self
                    .utd_index
                    .mark_unavailable(
                        TimelineEncryptedUnavailableItem {
                            item_id: item.item_id.clone(),
                            event_id: item.event_id.clone(),
                            room_id: room_id.clone(),
                            reason: Some(reason.as_str().to_owned()),
                        },
                        reason,
                    )
                    .is_err()
            {
                continue;
            }
            if item.decryption_state == Some(NativeDecryptionState::Pending)
                && self
                    .utd_index
                    .get(&room_id, &item.event_id)
                    .map(|e| e.phase)
                    == Some(UtdPhase::UnableToDecrypt)
            {
                let _ = self.utd_index.begin_retry(&room_id, &item.event_id);
            }
        }

        let current_event_ids: std::collections::HashSet<&str> = snapshot
            .items
            .iter()
            .filter(|item| item.decryption_state.is_some())
            .map(|item| item.event_id.as_str())
            .collect();
        let newly_recovered = previous_active
            .iter()
            .filter(|event_id| !current_event_ids.contains(event_id.as_str()))
            .count() as u32;
        for event_id in previous_active
            .iter()
            .filter(|event_id| !current_event_ids.contains(event_id.as_str()))
        {
            let _ = self.utd_index.mark_decrypted(&room_id, event_id);
        }
        self.utd_index.gc_decrypted();

        let pending_count = snapshot
            .items
            .iter()
            .filter(|item| item.decryption_state == Some(NativeDecryptionState::Pending))
            .count() as u32;
        let unavailable_count = snapshot
            .items
            .iter()
            .filter(|item| item.decryption_state == Some(NativeDecryptionState::Unavailable))
            .count() as u32;
        let utd_count = pending_count.saturating_add(unavailable_count);
        let needs_recovery_session = self
            .utd_recovery
            .get(&room_id)
            .map(|session| !session.phase.is_active())
            .unwrap_or(true);
        if utd_count > 0 && needs_recovery_session {
            let pending_ids = snapshot
                .items
                .iter()
                .filter(|item| item.decryption_state.is_some())
                .take(MAX_EVENT_IDS_PER_BATCH)
                .map(|item| item.event_id.clone())
                .collect();
            if let Ok(op_id) = self.utd_recovery.begin(room_id.clone(), kind, pending_ids) {
                let _ = self.utd_recovery.mark_in_flight(&room_id, op_id);
            }
        }
        if let Some(session) = self.utd_recovery.get(&room_id).cloned() {
            if session.phase.is_active() {
                let recovered = session.recovered_count.saturating_add(newly_recovered);
                let _ = if utd_count == 0 {
                    self.utd_recovery
                        .succeed(&room_id, session.op_id, recovered, 0)
                } else {
                    self.utd_recovery.report_progress(
                        &room_id,
                        session.op_id,
                        newly_recovered,
                        utd_count,
                    )
                };
            }
        }
        let recovery = self.utd_recovery.get(&room_id);
        snapshot.utd = NativeUtdStatus {
            phase: if pending_count > 0 {
                NativeUtdPhase::Recovering
            } else if unavailable_count > 0 && recovery.map(|s| s.recovered_count).unwrap_or(0) > 0
            {
                NativeUtdPhase::Partial
            } else if unavailable_count > 0 {
                NativeUtdPhase::Unavailable
            } else {
                NativeUtdPhase::Idle
            },
            pending_count,
            unavailable_count,
            recovered_count: recovery.map(|s| s.recovered_count).unwrap_or(0),
        };
    }
}

impl Drop for NativeTimelineRegistry {
    fn drop(&mut self) {
        for (_, task) in self.view_update_tasks.drain() {
            task.abort();
        }
    }
}

/// Where a timeline operation finds and installs cached SDK timelines.
///
/// `Shared` is the owner's registry. Its lock is held for one lookup,
/// install or publish at a time and never across an SDK build, a network
/// page or a send, so one slow room cannot stall every other timeline
/// command and media load. `Owned` is a registry its caller holds
/// exclusively (unit tests and the single-owner live proofs).
enum Slots<'a> {
    Owned(&'a mut NativeTimelineRegistry),
    Shared(&'a NativeTimelineOwner),
}

impl Slots<'_> {
    /// Run `f` on the registry, under the owner's lock when shared.
    async fn with<R>(&mut self, f: impl FnOnce(&mut NativeTimelineRegistry) -> R) -> R {
        match self {
            Slots::Owned(registry) => f(registry),
            Slots::Shared(owner) => f(&mut *owner.registry.lock().await),
        }
    }

    fn session_generation(&self) -> u64 {
        match self {
            Slots::Owned(registry) => registry.session_generation,
            Slots::Shared(owner) => owner.session_generation,
        }
    }

    /// The room's live timeline. A miss builds it, with its sparse-history
    /// probe, outside the registry lock and one build per room at a time.
    /// Returns whether this call built the installed timeline.
    async fn live(
        &mut self,
        client: &Client,
        room_id: &str,
    ) -> Result<(Arc<Timeline>, bool), &'static str> {
        if let Some(timeline) = self.with(|registry| registry.cached_live(room_id)).await {
            return Ok((timeline, false));
        }
        let gate = match self {
            Slots::Shared(owner) => Some(owner.live_build_gate(room_id)),
            Slots::Owned(_) => None,
        };
        let _building = match &gate {
            Some(gate) => Some(gate.lock().await),
            None => None,
        };
        if let Some(timeline) = self.with(|registry| registry.cached_live(room_id)).await {
            return Ok((timeline, false));
        }
        let entry = build_live_entry(client, room_id).await?;
        Ok(self
            .with(|registry| registry.adopt_live(room_id, entry))
            .await)
    }

    async fn cached_focused(&mut self, room_id: &str, event_id: &EventId) -> Option<Arc<Timeline>> {
        let key = (room_id.to_owned(), event_id.to_string());
        self.with(|registry| registry.focused_entries.get(&key).cloned())
            .await
    }

    /// Build an event-focused timeline (`/context`) outside the registry lock
    /// and install it. `None` when the SDK cannot build it in time.
    async fn build_focused(
        &mut self,
        room: &Room,
        room_id: &str,
        event_id: &OwnedEventId,
        num_context_events: u16,
    ) -> Option<Arc<Timeline>> {
        let build = TimelineBuilder::new(room)
            .with_focus(TimelineFocus::Event {
                target: event_id.clone(),
                num_context_events,
                thread_mode: permalink_event_thread_mode(),
            })
            .build();
        let timeline = timeout(FOCUSED_BUILD_TIMEOUT, build).await.ok()?.ok()?;
        let key = (room_id.to_owned(), event_id.to_string());
        Some(
            self.with(|registry| registry.adopt_focused(key, Arc::new(timeline)))
                .await,
        )
    }

    async fn focused(
        &mut self,
        room: &Room,
        room_id: &str,
        event_id: &OwnedEventId,
        num_context_events: u16,
    ) -> Option<Arc<Timeline>> {
        match self.cached_focused(room_id, event_id).await {
            Some(timeline) => Some(timeline),
            None => {
                self.build_focused(room, room_id, event_id, num_context_events)
                    .await
            }
        }
    }

    async fn forget_focused(&mut self, room_id: &str, event_id: &EventId) {
        let key = (room_id.to_owned(), event_id.to_string());
        self.with(|registry| {
            registry.focused_entries.remove(&key);
        })
        .await;
    }

    /// The thread timeline for `root_event_id`, built outside the registry
    /// lock on a miss. `None` when the SDK cannot build it.
    async fn thread(
        &mut self,
        room: &Room,
        room_id: &str,
        root_event_id: &OwnedEventId,
    ) -> Option<Arc<Timeline>> {
        let key = (room_id.to_owned(), root_event_id.to_string());
        if let Some(timeline) = self
            .with(|registry| registry.thread_entries.get(&key).cloned())
            .await
        {
            return Some(timeline);
        }
        let timeline = TimelineBuilder::new(room)
            .with_focus(TimelineFocus::Thread {
                root_event_id: root_event_id.clone(),
            })
            .track_read_marker_and_receipts(TimelineReadReceiptTracking::AllEvents)
            .build()
            .await
            .ok()?;
        debug_assert!(
            timeline.is_threaded(),
            "TimelineFocus::Thread must report is_threaded"
        );
        Some(
            self.with(|registry| registry.adopt_thread(key, Arc::new(timeline)))
                .await,
        )
    }
}

/// Build a room's live timeline. Network-bound when the restored cache is
/// sparse, so it never runs under the shared registry lock.
async fn build_live_entry(
    client: &Client,
    room_id: &str,
) -> Result<LiveTimelineEntry, &'static str> {
    let room_id = parse_room_id(room_id)?;
    let room = client
        .get_room(&room_id)
        .ok_or("d0.3-timeline-room-not-found")?;
    // Timeline open is a persisted read path. `latest_encryption_state`
    // deliberately performs a homeserver request when the state has not
    // been marked synchronized, which prevents cached messages from
    // opening after a cold offline restart. The SDK room store is the
    // authoritative readback here; send owners independently refresh
    // encryption state before performing network writes.
    let is_encrypted = room.encryption_state().is_encrypted();
    let timeline = TimelineBuilder::new(&room)
        .with_focus(TimelineFocus::Live {
            hide_threaded_events: HIDE_THREADED_EVENTS,
        })
        .track_read_marker_and_receipts(TimelineReadReceiptTracking::AllEvents)
        .build()
        .await
        .map_err(|_| "d0.3-timeline-open-failed")?;
    // A sparse restored cache must not require a scrollable viewport
    // to fetch history. One bounded SDK page serves both native clients;
    // a network failure retains the cached row and explicit pagination.
    // Whether it reached the room start is read back from the SDK's live
    // pagination status when a view stream opens.
    let sparse = timeline
        .items()
        .await
        .iter()
        .filter(|item| item.as_event().is_some())
        .take(2)
        .count()
        <= 1;
    if sparse {
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            timeline.paginate_backwards(PAGINATION_BATCH_SIZE),
        )
        .await;
    }
    Ok(LiveTimelineEntry {
        timeline: Arc::new(timeline),
        is_encrypted,
        last_used: UseStamp::now(),
    })
}

type TimelineUpdates = std::pin::Pin<
    Box<dyn futures_util::Stream<Item = Vec<VectorDiff<Arc<SdkTimelineItem>>>> + Send>,
>;

/// An opened view whose initial rows are ready, waiting to be published.
struct PreparedView {
    approval_history: HistoryProtection,
    stream_id: String,
    room_id: String,
    timeline: Arc<Timeline>,
    position: TimelineViewPosition,
    focus_route: Option<NativeTimelineEventRoute>,
    own_user_id: Option<OwnedUserId>,
    view: Arc<PublishedView>,
    settle_requests: tokio::sync::mpsc::UnboundedReceiver<SettleRequest>,
    media: Arc<AsyncMutex<TimelineMediaRegistry>>,
    items: SdkItems,
    annotations: KnownAnnotationIds,
    hydrate_sender_profiles: bool,
    updates: TimelineUpdates,
    snapshot: TimelineViewSnapshot,
}

/// The SDK items a view stream's rows are projected from, in the stream's
/// own (skip-relative) indices.
type SdkItems = eyeball_im::Vector<Arc<SdkTimelineItem>>;

/// Open a view stream. Lookups and installs go through `slots`; SDK builds,
/// `/context`, the sparse-history probe and row projection run between its
/// lock scopes. Publication is the last, synchronous step.
async fn open_view(
    slots: &mut Slots<'_>,
    approval_history: &ApprovalHistory,
    emit: TimelineViewUpdateEmit,
    client: &Client,
    request: NativeTimelineOpenRequest,
    resolved_focus_route: Option<NativeTimelineEventRoute>,
) -> Result<NativeTimelineOpenReadback, &'static str> {
    let session_generation = slots.session_generation();
    let room_id = parse_room_id(&request.room_id)?;
    let room_id_string = room_id.to_string();
    client
        .get_room(&room_id)
        .ok_or("d0.3-timeline-room-not-found")?;
    // Inbox history pages stop as soon as a view protects the room. The open
    // does not wait for an SDK page that is already in flight: that page
    // only adds older events the view would load anyway.
    let approval_history = approval_history.room(&room_id_string)?.protect();
    let requested_position = request.position;
    let mut focus_route = None;
    let (timeline, view_position, pagination) = match &requested_position {
        NativeTimelineOpenPosition::Normal { viewport } => {
            let room = client
                .get_room(&room_id)
                .ok_or("v-timeline-normal-room-not-found")?;
            let has_unread = room.is_marked_unread()
                || room.num_unread_messages() > 0
                || room.num_unread_notifications() > 0
                || room.num_unread_mentions() > 0;
            let (live, _) = slots.live(client, &room_id_string).await?;
            let current_live_tail = live_tail_event_id(&live).await;
            let unread_plan = unread_open_plan(client, &live).await;
            let unread_frontier = match &unread_plan {
                LastReadOpenPlan::InLive { event_id }
                | LastReadOpenPlan::Unavailable { event_id } => Some(event_id.clone()),
                LastReadOpenPlan::LiveBottom => None,
            };
            let has_unread = has_unread
                || unread_frontier
                    .as_deref()
                    .is_some_and(|anchor| Some(anchor) != current_live_tail.as_deref());
            let now_ms = unix_time_ms();
            let selected_position = resolve_normal_open_position(
                has_unread,
                unread_frontier,
                current_live_tail.as_deref(),
                now_ms,
                viewport,
            )?;
            match selected_position {
                TimelineViewPosition::LiveBottom => {
                    let pagination = live_pagination_state(&live).await;
                    (live, TimelineViewPosition::LiveBottom, pagination)
                }
                TimelineViewPosition::Unread { .. } => {
                    // Both available and unavailable markers keep this live
                    // provider. The presenter restores only an available row;
                    // an unavailable target remains an explicit user action.
                    let pagination = live_pagination_state(&live).await;
                    (live, selected_position, pagination)
                }
                TimelineViewPosition::Restored {
                    anchor_event_id: Some(ref anchor_event_id),
                } => {
                    open_focused_unread(
                        slots,
                        &room,
                        &room_id_string,
                        &live,
                        anchor_event_id,
                        "v-timeline-normal-anchor-invalid",
                        "v-timeline-normal-open-failed",
                    )
                    .await?
                }
                TimelineViewPosition::Focused { .. }
                | TimelineViewPosition::Thread { .. }
                | TimelineViewPosition::Restored {
                    anchor_event_id: None,
                } => unreachable!("normal open only selects live, unread, or anchored restore"),
            }
        }
        NativeTimelineOpenPosition::LiveBottom => {
            let (live, _) = slots.live(client, &room_id_string).await?;
            let pagination = live_pagination_state(&live).await;
            (live, TimelineViewPosition::LiveBottom, pagination)
        }
        NativeTimelineOpenPosition::Focused { event_id } => {
            let event_id = parse_event_id(event_id)?;
            let room = client
                .get_room(&room_id)
                .ok_or("v-timeline-focused-room-not-found")?;
            let (timeline, position, pagination, route) = open_focused(
                slots,
                client,
                &room,
                &room_id_string,
                &event_id,
                resolved_focus_route,
            )
            .await?;
            focus_route = Some(route);
            (timeline, position, pagination)
        }
        NativeTimelineOpenPosition::Thread { root_event_id } => {
            let room = client
                .get_room(&room_id)
                .ok_or("v-timeline-thread-room-not-found")?;
            open_thread(slots, &room, &room_id_string, root_event_id).await?
        }
        NativeTimelineOpenPosition::Unread => {
            let room = client
                .get_room(&room_id)
                .ok_or("v-timeline-unread-room-not-found")?;
            let has_unread = room.is_marked_unread()
                || room.num_unread_messages() > 0
                || room.num_unread_notifications() > 0
                || room.num_unread_mentions() > 0;
            if !has_unread {
                return Err("v-timeline-unread-open-no-unread");
            }
            let (live, _) = slots.live(client, &room_id_string).await?;
            match unread_open_plan(client, &live).await {
                LastReadOpenPlan::InLive { event_id } => {
                    let pagination = live_pagination_state(&live).await;
                    (
                        live,
                        TimelineViewPosition::Unread {
                            anchor_event_id: event_id,
                        },
                        pagination,
                    )
                }
                LastReadOpenPlan::Unavailable { event_id } => {
                    open_focused_unread(
                        slots,
                        &room,
                        &room_id_string,
                        &live,
                        &event_id,
                        "v-timeline-unread-frontier-unavailable",
                        "v-timeline-unread-open-failed",
                    )
                    .await?
                }
                LastReadOpenPlan::LiveBottom => {
                    let pagination = live_pagination_state(&live).await;
                    (live, TimelineViewPosition::LiveBottom, pagination)
                }
            }
        }
    };
    let own_user_id = client.user_id().map(ToOwned::to_owned);
    let action_authority = room_action_authority(timeline.room(), own_user_id.as_deref()).await;
    let focus = if matches!(view_position, TimelineViewPosition::Thread { .. }) {
        ViewFocus::Thread
    } else if timeline.live_back_pagination_status().await.is_some() {
        ViewFocus::Live
    } else {
        ViewFocus::Event
    };
    let pagination = TimelinePaginationState {
        backward: pagination.backward,
        forward: focus.initial_forward(),
    };
    let stream_number = slots.with(NativeTimelineRegistry::next_stream_number).await;
    let unclaimed_ttl = slots.with(|registry| registry.unclaimed_stream_ttl).await;
    let subscription_key = format!(
        "{}:{}",
        view_subscription_key(&room_id_string, &view_position),
        stream_number
    );
    let media = Arc::new(AsyncMutex::new(TimelineMediaRegistry::new(
        session_generation,
        subscription_key.clone(),
    )));
    // Subscribe before materializing the initial rows so no SDK update can
    // fall into a snapshot-to-stream gap. Every open owns a distinct
    // stream, revision counter, update task, and media registry.
    let (items, updates) = timeline.subscribe().await;
    let hydrate_sender_profiles = items
        .iter()
        .filter_map(|item| item.as_event())
        .any(|event| !matches!(event.sender_profile(), TimelineDetails::Ready(_)));
    let mut rows: Vec<super::TimelineViewRow> = {
        let mut registry = media.lock().await;
        items
            .iter()
            .map(|item| {
                project_timeline_item_with_media(
                    item,
                    own_user_id.as_deref(),
                    action_authority,
                    &mut registry,
                )
            })
            .collect()
    };
    enrich_view_rows(timeline.room(), &mut rows).await;
    let mut annotations = KnownAnnotationIds::default();
    annotations.remember_rows(&rows);
    let read_state =
        project_live_read_state(&timeline, &view_position, own_user_id.as_deref()).await;
    let (view, settle_requests) = PublishedView::new(PublishedViewInit {
        session_generation,
        room_id: room_id_string.clone(),
        focus,
        emitter: ViewDeltaEmitter::new(
            emit,
            session_generation,
            subscription_key.clone(),
            room_id_string.clone(),
            Arc::new(AtomicU64::new(0)),
        ),
        unclaimed_ttl,
        rows,
        position: view_position.clone(),
        pagination,
        read_state,
        pinned_event_ids: project_pinned_event_ids(timeline.room()),
        action_authority,
    });
    let view = Arc::new(view);
    let snapshot = view.snapshot();
    // Publish ownership only after all awaited initialization succeeds.
    // Cancellation before this point drops the local history protection.
    let view = PreparedView {
        approval_history,
        stream_id: subscription_key,
        room_id: room_id_string,
        timeline,
        position: view_position,
        focus_route,
        own_user_id,
        view,
        settle_requests,
        media,
        items,
        annotations,
        hydrate_sender_profiles,
        updates: Box::pin(updates),
        snapshot,
    };
    Ok(slots
        .with(move |registry| registry.publish_view(view))
        .await)
}

/// Place a `Focused` open: a notification, permalink, search hit or reply
/// jump. The live provider opens first, so the room is behind whatever
/// the target turns out to be.
///
/// - In the live window: present the live provider at the target. It keeps
///   growing with sync and can follow the live tail.
/// - Elsewhere in the room: build an event-focused timeline.
/// - An in-thread reply: the room timeline never shows it. Present the
///   live provider at the thread root when the live window has it, else at
///   the bottom, and report the thread so shells open it at the reply.
/// - Missing, or an event-focused build that fails: present the live
///   bottom and report why instead of failing the open.
async fn open_focused(
    slots: &mut Slots<'_>,
    client: &Client,
    room: &Room,
    room_id: &str,
    event_id: &OwnedEventId,
    resolved_route: Option<NativeTimelineEventRoute>,
) -> Result<
    (
        Arc<Timeline>,
        TimelineViewPosition,
        TimelinePaginationState,
        NativeTimelineEventRoute,
    ),
    &'static str,
> {
    let route = match resolved_route {
        Some(route) => route,
        None => resolve_event_route(room, event_id).await,
    };
    let (live, _) = slots.live(client, room_id).await?;
    let live_ids = live_window_event_ids(&live).await;
    match plan_focused_open(event_id.as_str(), route, &live_ids) {
        FocusedOpenPlan::Live {
            anchor_event_id,
            route,
        } => {
            let position = match anchor_event_id {
                Some(target_event_id) => TimelineViewPosition::Focused { target_event_id },
                None => TimelineViewPosition::LiveBottom,
            };
            let pagination = live_pagination_state(&live).await;
            return Ok((live, position, pagination, route));
        }
        FocusedOpenPlan::EventFocused => {}
    }
    let Some(timeline) = slots
        .focused(room, room_id, event_id, FOCUSED_CONTEXT_EVENT_COUNT)
        .await
    else {
        // `/context` failed. Keep the room usable at its live bottom
        // rather than failing the whole open.
        let pagination = live_pagination_state(&live).await;
        return Ok((
            live,
            TimelineViewPosition::LiveBottom,
            pagination,
            NativeTimelineEventRoute::Unreachable,
        ));
    };
    if timeline.is_threaded() {
        // `Automatic` thread mode filtered the window to the target's
        // thread, with no back gap when it holds the root. That is the
        // thread, not the room: never present it as a room position.
        slots.forget_focused(room_id, event_id).await;
        let root = threaded_focus_root(&timeline, event_id)
            .await
            .map(|root| root.to_string());
        let position = match root.clone().filter(|root| live_ids.contains(root)) {
            Some(target_event_id) => TimelineViewPosition::Focused { target_event_id },
            None => TimelineViewPosition::LiveBottom,
        };
        let route = match root {
            Some(root_event_id) => NativeTimelineEventRoute::Thread { root_event_id },
            None => NativeTimelineEventRoute::Unreachable,
        };
        let pagination = live_pagination_state(&live).await;
        return Ok((live, position, pagination, route));
    }
    let target_event_id = resolve_rendered_focus_target(room, &timeline, event_id).await;
    Ok((
        timeline,
        TimelineViewPosition::Focused { target_event_id },
        TimelinePaginationState {
            backward: TimelinePageState::Available,
            forward: TimelinePageState::Available,
        },
        NativeTimelineEventRoute::Room,
    ))
}

async fn open_focused_unread(
    slots: &mut Slots<'_>,
    room: &Room,
    room_id: &str,
    live: &Arc<Timeline>,
    anchor_event_id: &str,
    invalid_id: &'static str,
    open_failed: &'static str,
) -> Result<(Arc<Timeline>, TimelineViewPosition, TimelinePaginationState), &'static str> {
    let event_id = parse_event_id(anchor_event_id).map_err(|_| invalid_id)?;
    let timeline = slots
        .focused(room, room_id, &event_id, FOCUSED_CONTEXT_EVENT_COUNT)
        .await
        .ok_or(open_failed)?;
    if timeline.is_threaded() {
        // A read marker or restored anchor on an in-thread reply (another
        // client wrote it). The thread-only window is not the room. The
        // reply's room position is its thread root: place the live
        // provider there when the live window has it, else at the bottom.
        slots.forget_focused(room_id, &event_id).await;
        let root = threaded_focus_root(&timeline, &event_id)
            .await
            .map(|root| root.to_string());
        let live_ids = live_window_event_ids(live).await;
        let position = match root.filter(|root| live_ids.contains(root)) {
            Some(anchor_event_id) => TimelineViewPosition::Unread { anchor_event_id },
            None => TimelineViewPosition::LiveBottom,
        };
        return Ok((live.clone(), position, live_pagination_state(live).await));
    }
    let anchor_event_id = resolve_rendered_focus_target(room, &timeline, &event_id).await;
    Ok((
        timeline,
        TimelineViewPosition::Unread { anchor_event_id },
        TimelinePaginationState {
            backward: TimelinePageState::Available,
            forward: TimelinePageState::Available,
        },
    ))
}

async fn open_thread(
    slots: &mut Slots<'_>,
    room: &Room,
    room_id: &str,
    root_event_id: &str,
) -> Result<(Arc<Timeline>, TimelineViewPosition, TimelinePaginationState), &'static str> {
    let root_event_id = parse_thread_root_event_id_for_open(root_event_id)?;
    let timeline = slots
        .thread(room, room_id, &root_event_id)
        .await
        .ok_or("v-timeline-thread-open-failed")?;
    Ok((
        timeline,
        TimelineViewPosition::Thread {
            root_event_id: root_event_id.to_string(),
        },
        TimelinePaginationState {
            backward: TimelinePageState::Available,
            forward: TimelinePageState::Available,
        },
    ))
}

async fn live_tail_event_id(live: &Timeline) -> Option<String> {
    let items = live.items().await;
    items.iter().rev().find_map(|item| {
        item.as_event()
            .and_then(|event| event.event_id().map(|event_id| event_id.to_string()))
    })
}

async fn live_event_ids(live: &Timeline) -> Vec<String> {
    let items = live.items().await;
    items
        .iter()
        .filter_map(|item| {
            item.as_event()
                .and_then(|event| event.event_id().map(|event_id| event_id.to_string()))
        })
        .collect()
}

async fn unread_open_plan(client: &Client, live: &Timeline) -> LastReadOpenPlan {
    let live_ids = live_event_ids(live).await;
    let (raw, anchor) = navigation_read_state(live, client.user_id()).await;
    let target = anchor.or(raw);
    plan_unread_open(&live_ids, target.as_deref())
}

/// Event ids the live provider's view stream renders now. A view
/// subscription sees only the SDK skip window (the newest items until it
/// paginates back), not every item the timeline holds, so a target can be
/// placed on the live provider only when it is in this window.
async fn live_window_event_ids(live: &Timeline) -> Vec<String> {
    let (items, _updates) = live.subscribe().await;
    items
        .iter()
        .filter_map(|item| {
            item.as_event()
                .and_then(|event| event.event_id().map(|event_id| event_id.to_string()))
        })
        .collect()
}

/// Bound on one user-initiated page. Product requests retry three times at
/// 30 s each; a page someone is waiting on fails sooner and can be retried.
const USER_PAGINATION_TIMEOUT: Duration = Duration::from_secs(20);

async fn paginate_view(
    slots: &mut Slots<'_>,
    request: NativeTimelineViewPaginationRequest,
) -> Result<TimelineViewSnapshot, &'static str> {
    let (timeline, view) = slots
        .with(|registry| registry.stream_target(&request.stream_id))
        .await?;
    let page_state = |reached_end: bool| {
        if reached_end {
            TimelinePageState::Exhausted
        } else {
            TimelinePageState::Available
        }
    };
    let change = match (request.direction, view.focus()) {
        (NativeTimelineDirection::Backwards, _) => {
            let reached_start = timeout(
                USER_PAGINATION_TIMEOUT,
                timeline.paginate_backwards(PAGINATION_BATCH_SIZE),
            )
            .await
            .map_err(|_| "v-timeline-view-paginate-timeout")?
            .map_err(|_| "v-timeline-view-paginate-backwards-failed")?;
            ViewChange {
                backward: Some(page_state(reached_start)),
                ..ViewChange::default()
            }
        }
        // The live provider is always at the newest event, and the SDK
        // cannot page a thread forward. Neither asks the SDK.
        (NativeTimelineDirection::Forwards, ViewFocus::Live) => ViewChange {
            forward: Some(TimelinePageState::Exhausted),
            ..ViewChange::default()
        },
        (NativeTimelineDirection::Forwards, ViewFocus::Thread) => ViewChange::default(),
        (NativeTimelineDirection::Forwards, ViewFocus::Event) => {
            let reached_end = timeout(
                USER_PAGINATION_TIMEOUT,
                timeline.paginate_forwards(PAGINATION_BATCH_SIZE),
            )
            .await
            .map_err(|_| "v-timeline-view-paginate-timeout")?
            .map_err(|_| "v-timeline-view-paginate-forwards-failed")?;
            ViewChange {
                forward: Some(page_state(reached_end)),
                ..ViewChange::default()
            }
        }
    };
    // Let the update task publish the rows this page queued, then publish
    // the page state as its own batch so a stale readback cannot drop it.
    view.settled().await;
    view.publish(change);
    Ok(view.snapshot())
}

/// Promote a stream's unread placement to the live bottom. See
/// [`NativeTimelineRegistry::follow_live_tail`].
async fn follow_live_tail_in(
    slots: &mut Slots<'_>,
    client: &Client,
    request: NativeTimelineFollowLiveRequest,
) -> Result<TimelineViewSnapshot, &'static str> {
    let raw_observed = request.observed_live_tail_event_id.trim();
    if raw_observed.is_empty() {
        return Err("v-timeline-follow-live-tail-required");
    }
    let observed = parse_action_event_id(raw_observed, "v-timeline-follow-live-tail-invalid")?;
    let (timeline, view, uses_live_provider) = slots
        .with(|registry| {
            let (timeline, view) = registry.stream_target(&request.stream_id)?;
            let room_id = &registry.view_streams[&request.stream_id].room_id;
            let uses_live_provider = registry
                .cached_live(room_id)
                .is_some_and(|live| Arc::ptr_eq(&live, &timeline));
            Ok::<_, &'static str>((timeline, view, uses_live_provider))
        })
        .await?;
    if !uses_live_provider {
        return Err("v-timeline-follow-live-tail-not-loaded");
    }
    if view.position() != TimelineViewPosition::LiveBottom {
        match timeline.latest_event_id().await {
            Some(tail) if tail == observed => {
                // The update task reads the position for every later read
                // state, so deltas stop reporting the open-time anchor.
                view.set_position(TimelineViewPosition::LiveBottom);
                publish_current_read_state(&timeline, &view, client.user_id()).await;
            }
            _ => return Err("v-timeline-follow-live-tail-not-loaded"),
        }
    }
    Ok(view.snapshot())
}

/// Publish the stream's read state as a command just left it, so the
/// command's readback (read from the published view) and the delta stream
/// agree on it.
async fn publish_current_read_state(
    timeline: &Timeline,
    view: &PublishedView,
    own_user_id: Option<&UserId>,
) {
    let position = view.position();
    let read_state = project_live_read_state(timeline, &position, own_user_id).await;
    if read_state != view.read_state() {
        view.publish(ViewChange {
            read_state: Some(read_state),
            read_state_position: Some(position),
            ..ViewChange::default()
        });
    }
}

/// Open (or reuse) the live timeline a room-level Mark as Read acts on.
async fn open_room_read_timeline_in(
    slots: &mut Slots<'_>,
    client: &Client,
    room_id: &str,
) -> Result<Arc<Timeline>, &'static str> {
    let (timeline, built) = slots
        .live(client, room_id)
        .await
        .map_err(|_| "v-rooms-room-read-state-mark-read-failed")?;
    if built {
        // Room-list Mark as Read does not mount the room. Keep its
        // timeline only as the first candidate for eviction.
        slots.with(|registry| registry.demote_live(room_id)).await;
    }
    Ok(timeline)
}

async fn event_readback_in(
    slots: &mut Slots<'_>,
    client: &Client,
    room_id: &str,
    event_id: &str,
) -> Result<NativeTimelineEventReadback, &'static str> {
    let room_id = parse_room_id(room_id)?.to_string();
    let event_id = parse_event_id(event_id)?;
    let timeline = match slots.cached_focused(&room_id, &event_id).await {
        Some(timeline) => timeline,
        None => {
            let room = client
                .get_room(parse_room_id(&room_id)?.as_ref())
                .ok_or("v-crypto.6-event-room-not-found")?;
            slots
                .build_focused(&room, &room_id, &event_id, 0)
                .await
                .ok_or("v-crypto.6-event-open-failed")?
        }
    };
    let (items, _updates) = timeline.subscribe().await;
    let mut item = items
        .iter()
        .filter_map(|item| project_item(item, client.user_id()))
        .find(|item| item.event_id == event_id.as_str())
        .ok_or("v-crypto.6-event-not-found")?;
    if let Some(room) = client.get_room(parse_room_id(&room_id)?.as_ref()) {
        enrich_native_reactions(&room, event_id.as_str(), &mut item.reactions, true).await;
    }
    Ok(NativeTimelineEventReadback {
        session_generation: slots.session_generation(),
        room_id,
        event_id: event_id.to_string(),
        item,
    })
}

async fn toggle_reaction_in(
    slots: &mut Slots<'_>,
    client: &Client,
    room_id: &str,
    target_event_id: &str,
    key: &str,
) -> Result<NativeReactionMutationResult, &'static str> {
    let room_id = parse_room_id(room_id)?.to_string();
    let target_event_id = parse_event_id(target_event_id)?;
    validate_reaction_key(key)?;
    slots.live(client, &room_id).await?;
    let timeline = reaction_timeline(slots, &room_id, &target_event_id).await?;
    let added = timeline
        .toggle_reaction(&TimelineEventItemId::EventId(target_event_id.clone()), key)
        .await
        .map_err(|_| "v-send.2-reaction-toggle-failed")?;
    let readback = reaction_readback(slots, client, &room_id, &target_event_id, key, false).await?;
    Ok(NativeReactionMutationResult {
        room_id,
        target_event_id: target_event_id.to_string(),
        key: key.to_owned(),
        mutation: if added {
            NativeReactionMutation::Added
        } else {
            NativeReactionMutation::Removed
        },
        readback,
    })
}

/// The open timeline that renders `target_event_id`.
///
/// The live timeline hides thread replies and only covers the recent tail,
/// so a reaction on a thread reply or a permalinked event must go through
/// the thread, view or focused timeline that holds that row. Falls back to
/// the live timeline, whose error then reports the missing row.
async fn reaction_timeline(
    slots: &mut Slots<'_>,
    room_id: &str,
    target_event_id: &EventId,
) -> Result<Arc<Timeline>, &'static str> {
    let candidates = slots
        .with(|registry| registry.reaction_candidates(room_id))
        .await?;
    let live = candidates[0].clone();
    for timeline in candidates {
        if timeline.item_by_event_id(target_event_id).await.is_some() {
            return Ok(timeline);
        }
    }
    Ok(live)
}

/// Idempotently ensure an approval annotation exists. This intentionally
/// does *not* call `toggle_reaction`: an existing reaction remains present.
async fn ensure_reaction_in(
    slots: &mut Slots<'_>,
    client: &Client,
    room_id: &str,
    target_event_id: &str,
    key: &str,
) -> Result<NativeReactionMutationResult, &'static str> {
    let room_id = parse_room_id(room_id)?.to_string();
    let target_event_id = parse_event_id(target_event_id)?;
    validate_reaction_key(key)?;
    let before = reaction_readback(slots, client, &room_id, &target_event_id, key, false).await?;
    if before.as_ref().is_some_and(|reaction| reaction.me) {
        return Ok(NativeReactionMutationResult {
            room_id,
            target_event_id: target_event_id.to_string(),
            key: key.to_owned(),
            mutation: NativeReactionMutation::AlreadyPresent,
            readback: before,
        });
    }

    let room = client
        .get_room(parse_room_id(&room_id)?.as_ref())
        .ok_or("v-send.2-reaction-room-not-found")?;
    // A reaction the SDK still holds after a recoverable error is already
    // shown as the user's local echo and will retry; reporting it failed
    // would invite a second, duplicate annotation. Same rule as redaction.
    accept_still_queued(
        send_event_via_room_queue(
            &room,
            ReactionEventContent::from(Annotation::new(target_event_id.clone(), key.to_owned()))
                .into(),
        )
        .await,
        "v-send.2-reaction-ensure-failed",
    )?;

    let readback = reaction_readback(slots, client, &room_id, &target_event_id, key, false).await?;
    Ok(NativeReactionMutationResult {
        room_id,
        target_event_id: target_event_id.to_string(),
        key: key.to_owned(),
        mutation: NativeReactionMutation::Added,
        readback,
    })
}

/// Redact any reaction annotation selected in the viewer. Aggregated
/// annotations are not timeline rows, so this queues the redaction on the
/// room send queue rather than calling `Timeline::redact`.
async fn redact_reaction_in(
    slots: &mut Slots<'_>,
    client: &Client,
    room_id: &str,
    target_event_id: &str,
    reaction_event_id: &str,
    key: &str,
) -> Result<NativeReactionMutationResult, &'static str> {
    let room_id = parse_room_id(room_id)?.to_string();
    let target_event_id = parse_event_id(target_event_id)?;
    let reaction_event_id = parse_event_id(reaction_event_id)?;
    validate_reaction_key(key)?;
    let selected_reaction = reaction_readback(slots, client, &room_id, &target_event_id, key, true)
        .await?
        .ok_or("v-send.2-reaction-redact-annotation-not-found")?;
    if !reaction_contains_event_id(&selected_reaction, &reaction_event_id) {
        return Err("v-send.2-reaction-redact-annotation-not-found");
    }
    let room = client
        .get_room(parse_room_id(&room_id)?.as_ref())
        .ok_or("v-send.2-reaction-room-not-found")?;
    accept_still_queued(
        redact_via_room_queue(&room, &reaction_event_id, Some("Removed reaction")).await,
        "v-send.2-reaction-redact-failed",
    )?;
    let readback = reaction_readback(slots, client, &room_id, &target_event_id, key, true).await?;
    Ok(NativeReactionMutationResult {
        room_id,
        target_event_id: target_event_id.to_string(),
        key: key.to_owned(),
        mutation: NativeReactionMutation::Redacted,
        readback,
    })
}

async fn reaction_readback(
    slots: &mut Slots<'_>,
    client: &Client,
    room_id: &str,
    target_event_id: &OwnedEventId,
    key: &str,
    network: bool,
) -> Result<Option<NativeTimelineReaction>, &'static str> {
    let (live, _) = slots.live(client, room_id).await?;
    let (items, _updates) = live.subscribe().await;
    if let Some(item) = items
        .iter()
        .filter_map(|item| project_item(item, client.user_id()))
        .find(|item| item.event_id == target_event_id.as_str())
    {
        // Live viewport owns this event. After toggle-remove the live item
        // can correctly omit the key while a focused Event timeline still
        // projects the previous aggregation (event-cache / local Sent echo).
        // Falling through made `ensure_reaction` report AlreadyPresent and
        // skip the re-add. Keep the focused timeline; dropping it here
        // desyncs the live room timeline's local annotation echo.
        let mut reaction = item
            .reactions
            .into_iter()
            .find(|reaction| reaction.key == key);
        if let (Some(reaction), Some(room)) = (
            reaction.as_mut(),
            client.get_room(parse_room_id(room_id)?.as_ref()),
        ) {
            enrich_native_reactions(
                &room,
                target_event_id.as_str(),
                std::slice::from_mut(reaction),
                network,
            )
            .await;
        }
        return Ok(reaction);
    }

    // Notifications may target a message outside the currently open
    // viewport. A focused native timeline is the same authoritative owner
    // used for event readback; no JS relation inspection is involved.
    let timeline = match slots.cached_focused(room_id, target_event_id).await {
        Some(timeline) => timeline,
        None => {
            let room = client
                .get_room(parse_room_id(room_id)?.as_ref())
                .ok_or("v-send.2-reaction-room-not-found")?;
            let Some(timeline) = slots
                .build_focused(&room, room_id, target_event_id, 0)
                .await
            else {
                // Toggle/ensure already applied. Missing focused history must
                // not fail the mutation; viewer open uses event_readback.
                return Ok(None);
            };
            timeline
        }
    };
    let (items, _updates) = timeline.subscribe().await;
    let mut reaction = items
        .iter()
        .filter_map(|item| project_item(item, client.user_id()))
        .find(|item| item.event_id == target_event_id.as_str())
        .and_then(|item| {
            item.reactions
                .into_iter()
                .find(|reaction| reaction.key == key)
        });
    if let (Some(reaction), Some(room)) = (
        reaction.as_mut(),
        client.get_room(parse_room_id(room_id)?.as_ref()),
    ) {
        enrich_native_reactions(
            &room,
            target_event_id.as_str(),
            std::slice::from_mut(reaction),
            network,
        )
        .await;
    }
    Ok(reaction)
}

fn unix_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn newest_frontier_in_live<'a>(
    live_event_ids: &[String],
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    for candidate in candidates {
        if let Some(index) = live_event_ids.iter().position(|id| id == candidate) {
            if best
                .as_ref()
                .is_none_or(|(best_index, _)| index > *best_index)
            {
                best = Some((index, candidate.to_owned()));
            }
        }
    }
    best.map(|(_, id)| id)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LastReadOpenPlan {
    /// Last-read is already in the live window. Stay live and place there.
    InLive { event_id: String },
    /// Last-read is outside the live window. Retain it for explicit navigation.
    Unavailable { event_id: String },
    /// Newest frontier cannot be established without walking history.
    LiveBottom,
}

// Only the SDK stream graph establishes order. Receipt timestamps describe
// acknowledgement time, not the order of the acknowledged events.
fn plan_unread_open(live_event_ids: &[String], candidate: Option<&str>) -> LastReadOpenPlan {
    match candidate {
        Some(event_id) if live_event_ids.iter().any(|id| id == event_id) => {
            LastReadOpenPlan::InLive {
                event_id: event_id.to_owned(),
            }
        }
        Some(event_id) => LastReadOpenPlan::Unavailable {
            event_id: event_id.to_owned(),
        },
        None => LastReadOpenPlan::LiveBottom,
    }
}

/// Bounded like the focused-anchor timestamp lookup: a routed open must not
/// hang on a slow homeserver.
const EVENT_ROUTE_TIMEOUT: Duration = Duration::from_secs(5);

/// Resolve where a routed event lives from the event itself: the event cache,
/// else one `/event` request. The thread relation is read from the raw event
/// the same way the SDK's event-focused cache decides on thread mode, and it
/// is in the clear for encrypted events too.
async fn resolve_event_route(
    room: &Room,
    event_id: &matrix_sdk::ruma::EventId,
) -> NativeTimelineEventRoute {
    match timeout(
        EVENT_ROUTE_TIMEOUT,
        room.load_or_fetch_event(event_id, None),
    )
    .await
    {
        Ok(Ok(event)) => event_route_for_thread_root(
            event_id,
            matrix_sdk::serde_helpers::extract_thread_root(event.raw()),
        ),
        Ok(Err(error)) if event_fetch_is_missing(&error) => NativeTimelineEventRoute::Missing,
        Ok(Err(_)) | Err(_) => NativeTimelineEventRoute::Unreachable,
    }
}

fn event_route_for_thread_root(
    event_id: &matrix_sdk::ruma::EventId,
    thread_root: Option<OwnedEventId>,
) -> NativeTimelineEventRoute {
    match thread_root {
        Some(root) if root != event_id => NativeTimelineEventRoute::Thread {
            root_event_id: root.to_string(),
        },
        _ => NativeTimelineEventRoute::Room,
    }
}

/// `/event` answers 404 for an event that does not exist or that history
/// visibility hides, and 403 when the account may not read the room.
fn event_fetch_is_missing(error: &matrix_sdk::Error) -> bool {
    error
        .as_client_api_error()
        .is_some_and(|error| matches!(error.status_code.as_u16(), 403 | 404))
}

/// The thread root of `target` inside a thread-filtered event-focused
/// timeline. The SDK keeps the root private, but the target's own item
/// carries it.
async fn threaded_focus_root(
    timeline: &Timeline,
    target: &matrix_sdk::ruma::EventId,
) -> Option<OwnedEventId> {
    timeline
        .item_by_event_id(target)
        .await?
        .content()
        .thread_root()
}

#[derive(Debug, PartialEq, Eq)]
enum FocusedOpenPlan {
    /// Present the live provider at `anchor_event_id`, or at its bottom.
    Live {
        anchor_event_id: Option<String>,
        route: NativeTimelineEventRoute,
    },
    /// Room history outside the live window: build an event-focused timeline.
    EventFocused,
}

/// Choose the provider for a `Focused` open from the target's route and the
/// live provider's rendered window.
fn plan_focused_open(
    target: &str,
    route: NativeTimelineEventRoute,
    live_window_ids: &[String],
) -> FocusedOpenPlan {
    let in_live = |id: &str| live_window_ids.iter().any(|live| live == id);
    match route {
        // The live window holds it, so it is a room event whatever `/event`
        // said.
        NativeTimelineEventRoute::Room | NativeTimelineEventRoute::Unreachable
            if in_live(target) =>
        {
            FocusedOpenPlan::Live {
                anchor_event_id: Some(target.to_owned()),
                route: NativeTimelineEventRoute::Room,
            }
        }
        // An unreachable `/event` may still be served by `/context`.
        NativeTimelineEventRoute::Room | NativeTimelineEventRoute::Unreachable => {
            FocusedOpenPlan::EventFocused
        }
        NativeTimelineEventRoute::Thread { root_event_id } => FocusedOpenPlan::Live {
            anchor_event_id: in_live(&root_event_id).then(|| root_event_id.clone()),
            route: NativeTimelineEventRoute::Thread { root_event_id },
        },
        NativeTimelineEventRoute::Missing => FocusedOpenPlan::Live {
            anchor_event_id: None,
            route: NativeTimelineEventRoute::Missing,
        },
    }
}

/// Pick the rendered event a focused `/context` window should place on when
/// its target has no item of its own. Rendered items arrive in stream order;
/// the newest one at or before the target's timestamp is the nearest visible
/// predecessor. With no timestamp (the target could not be loaded) fall back
/// to the newest rendered item, which sits closest to the unread frontier.
fn rendered_focus_anchor(
    rendered: &[(String, u64)],
    target: &str,
    target_ts_ms: Option<u64>,
) -> Option<String> {
    if rendered.iter().any(|(id, _)| id == target) {
        return Some(target.to_owned());
    }
    let chosen = match target_ts_ms {
        Some(target_ts) => rendered
            .iter()
            .rev()
            .find(|(_, ts)| *ts <= target_ts)
            .or_else(|| rendered.first()),
        None => rendered.last(),
    };
    chosen.map(|(id, _)| id.clone())
}

/// A read marker set by another client often names a reaction, edit,
/// redaction, or hidden event. The SDK still centres the focused window on it
/// but never renders it as an item, so presenters could not place the
/// viewport. Resolve to a rendered neighbour instead of failing the open.
async fn resolve_rendered_focus_target(
    room: &Room,
    timeline: &Timeline,
    target: &matrix_sdk::ruma::EventId,
) -> String {
    let rendered: Vec<(String, u64)> = timeline
        .items()
        .await
        .iter()
        .filter_map(|item| {
            let event = item.as_event()?;
            Some((
                event.event_id()?.to_string(),
                u64::from(event.timestamp().0),
            ))
        })
        .collect();
    if rendered.iter().any(|(id, _)| id == target.as_str()) {
        return target.to_string();
    }
    let target_ts_ms = match timeout(
        Duration::from_secs(5),
        room.load_or_fetch_event(target, None),
    )
    .await
    {
        Ok(Ok(event)) => event.timestamp().map(|ts| u64::from(ts.0)),
        Ok(Err(_)) | Err(_) => None,
    };
    rendered_focus_anchor(&rendered, target.as_str(), target_ts_ms)
        .unwrap_or_else(|| target.to_string())
}

/// Resolve a raw marker to the nearest rendered predecessor in stream order.
/// An edit of an old message belongs at the edit's position, not its target.
fn visible_predecessor(raw_ids: &[String], visible_ids: &[String], marker: &str) -> Option<String> {
    let marker_index = raw_ids.iter().position(|id| id == marker)?;
    raw_ids[..=marker_index]
        .iter()
        .rev()
        .find(|id| visible_ids.contains(id))
        .cloned()
}

/// Room-level last-read candidates. Live timelines set
/// `hide_threaded_events`, so `Timeline::latest_user_read_receipt` only
/// loads `ReceiptThread::Main`. Other clients (and this test fixture) still
/// write unthreaded receipts; restoration must compare both.
async fn own_unthreaded_and_main_receipt_event_ids(room: &Room, user: &UserId) -> Vec<String> {
    let mut ids = Vec::new();
    for receipt_type in [EventReceiptType::Read, EventReceiptType::ReadPrivate] {
        for thread in [&ReceiptThread::Unthreaded, &ReceiptThread::Main] {
            if let Ok(Some((event_id, _))) = room
                .load_user_receipt(receipt_type.clone(), thread, user)
                .await
            {
                ids.push(event_id.to_string());
            }
        }
    }
    ids
}

async fn navigation_read_state(
    timeline: &Timeline,
    own_user_id: Option<&UserId>,
) -> (Option<String>, Option<String>) {
    let visible_ids: Vec<String> = timeline
        .items()
        .await
        .iter()
        .filter_map(|item| {
            item.as_event()
                .and_then(|event| event.event_id().map(ToString::to_string))
        })
        .collect();
    // Read at most the bounded live window from the in-memory SDK cache. This
    // does not page storage or traverse linked history to compare markers.
    let mut raw_ids = Vec::new();
    if let Ok((cache, _handles)) = timeline.room().event_cache().await {
        let mut visited = 0usize;
        let _ = cache
            .rfind_map_event_in_memory_by(|event| {
                visited += 1;
                if let Some(id) = event.event_id() {
                    raw_ids.push(id.to_string());
                }
                (visited >= 300).then_some(())
            })
            .await;
        raw_ids.reverse();
    }
    if raw_ids.is_empty() {
        raw_ids.clone_from(&visible_ids);
    }
    let fully_read = timeline
        .room()
        .fully_read_event_id()
        .map(|id| id.to_string());
    let (receipt_ids, timeline_receipt, receipt_anchor) = match own_user_id {
        Some(user) => {
            let receipt_ids =
                own_unthreaded_and_main_receipt_event_ids(timeline.room(), user).await;
            let timeline_receipt = timeline
                .latest_user_read_receipt(user)
                .await
                .map(|(id, _)| id.to_string());
            let receipt_anchor = timeline
                .latest_user_read_receipt_timeline_event_id(user)
                .await
                .map(|id| id.to_string());
            (receipt_ids, timeline_receipt, receipt_anchor)
        }
        None => (Vec::new(), None, None),
    };
    let raw = newest_frontier_in_live(
        &raw_ids,
        fully_read
            .iter()
            .chain(receipt_ids.iter())
            .chain(timeline_receipt.iter())
            .map(String::as_str),
    )
    .or_else(|| timeline_receipt.clone())
    .or_else(|| receipt_ids.into_iter().next())
    .or(fully_read);
    let anchor = raw
        .as_deref()
        .and_then(|marker| visible_predecessor(&raw_ids, &visible_ids, marker))
        .or_else(|| {
            (raw == timeline_receipt)
                .then_some(receipt_anchor)
                .flatten()
        });
    (raw, anchor)
}

/// Whether the SDK reports a live provider at the start of the room now. It
/// is the only authority: a gappy sync, a cross-process reload or an
/// auto-shrink reopens history the provider once reached.
async fn live_hit_start(live: &Timeline) -> bool {
    matches!(
        live.live_back_pagination_status().await,
        Some((
            PaginationStatus::Idle {
                hit_timeline_start: true
            },
            _
        ))
    )
}

/// Backward pagination a new stream on a live provider starts from.
async fn live_pagination_state(live: &Timeline) -> TimelinePaginationState {
    pagination_state_from_hit_start(live_hit_start(live).await)
}

fn should_restore_viewport(
    has_unread: bool,
    now_ms: u64,
    current_live_tail_event_id: Option<&str>,
    viewport: &NativeTimelineViewportHint,
) -> bool {
    if has_unread {
        return viewport.at_bottom
            && viewport.live_tail_event_id.as_deref().is_some()
            && current_live_tail_event_id.is_some()
            && viewport.live_tail_event_id.as_deref() == current_live_tail_event_id;
    }
    if viewport.at_bottom {
        return true;
    }
    let Some(updated_at_ms) = viewport.updated_at_ms else {
        return false;
    };
    if now_ms.saturating_sub(updated_at_ms) > NATIVE_TIMELINE_VIEWPORT_RESTORE_TTL_MS {
        return false;
    }
    viewport
        .restored_anchor_event_id
        .as_deref()
        .and_then(|event_id| parse_event_id(event_id).ok())
        .is_some()
}

fn resolve_normal_open_position(
    has_unread: bool,
    fully_read_event_id: Option<String>,
    current_live_tail_event_id: Option<&str>,
    now_ms: u64,
    viewport: &NativeTimelineViewportHint,
) -> Result<TimelineViewPosition, &'static str> {
    let can_restore =
        should_restore_viewport(has_unread, now_ms, current_live_tail_event_id, viewport);
    if can_restore && viewport.at_bottom {
        return Ok(TimelineViewPosition::LiveBottom);
    }
    if has_unread {
        // A supported room can be unread without a frontier already present in
        // the live graph (never-opened channel, or only a stale `m.fully_read`
        // outside the bounded live window). With no comparable live frontier,
        // open live instead of focused-opening historical months.
        let anchor_event_id = match fully_read_event_id {
            Some(anchor_event_id) => anchor_event_id,
            None => return Ok(TimelineViewPosition::LiveBottom),
        };
        return Ok(TimelineViewPosition::Unread { anchor_event_id });
    }
    if can_restore {
        if let Some(anchor_event_id) = viewport
            .restored_anchor_event_id
            .as_deref()
            .and_then(|event_id| parse_event_id(event_id).ok())
            .map(|event_id| event_id.to_string())
        {
            return Ok(TimelineViewPosition::Restored {
                anchor_event_id: Some(anchor_event_id),
            });
        }
    }
    Ok(TimelineViewPosition::LiveBottom)
}

fn view_subscription_key(room_id: &str, position: &TimelineViewPosition) -> String {
    match position {
        TimelineViewPosition::LiveBottom => format!("live:{room_id}"),
        TimelineViewPosition::Unread { .. } => format!("unread:{room_id}"),
        TimelineViewPosition::Focused { target_event_id } => {
            format!("focused:{room_id}:{target_event_id}")
        }
        TimelineViewPosition::Thread { root_event_id } => {
            format!("thread:{room_id}:{root_event_id}")
        }
        TimelineViewPosition::Restored {
            anchor_event_id: Some(anchor_event_id),
        } => format!("restored:{room_id}:{anchor_event_id}"),
        TimelineViewPosition::Restored {
            anchor_event_id: None,
        } => format!("restored:{room_id}:none"),
    }
}

struct ViewUpdateOwnerInput {
    own_user_id: Option<OwnedUserId>,
    view: Arc<PublishedView>,
    settle_requests: tokio::sync::mpsc::UnboundedReceiver<SettleRequest>,
    media: Arc<AsyncMutex<TimelineMediaRegistry>>,
    items: SdkItems,
    annotations: KnownAnnotationIds,
    timeline: Arc<Timeline>,
    hydrate_sender_profiles: bool,
}

/// Re-project a stream's whole window, for a room-authority change. The
/// window is the task's own SDK-item mirror, in the same skip-relative
/// indices as the diff stream, so later ops still address the right rows.
async fn reproject_view_window(
    room: &Room,
    items: &SdkItems,
    own_user_id: Option<&UserId>,
    action_authority: TimelineRoomActionAuthority,
    media: &AsyncMutex<TimelineMediaRegistry>,
    annotations: &mut KnownAnnotationIds,
) -> Vec<super::TimelineViewRow> {
    let mut rows: Vec<super::TimelineViewRow> = {
        let mut registry = media.lock().await;
        registry.retain_items(items.iter().map(|item| item.unique_id().0.as_str()));
        items
            .iter()
            .map(|item| {
                project_timeline_item_with_media(item, own_user_id, action_authority, &mut registry)
            })
            .collect()
    };
    annotations.fill_rows(&mut rows);
    enrich_view_rows(room, &mut rows).await;
    annotations.remember_rows(&rows);
    rows
}

fn spawn_view_update_owner(
    input: ViewUpdateOwnerInput,
    updates: impl futures_util::Stream<Item = Vec<VectorDiff<Arc<SdkTimelineItem>>>> + Send + 'static,
) -> JoinHandle<()> {
    let ViewUpdateOwnerInput {
        own_user_id,
        view,
        mut settle_requests,
        media,
        mut items,
        mut annotations,
        timeline,
        hydrate_sender_profiles,
    } = input;
    let client = timeline.room().client();
    // Room-scoped and typed: only this room's power-level changes reach the
    // handler. A one-slot channel coalesces bursts into one recompute.
    let (power_authority_tx, mut power_authority_rx) = tokio::sync::mpsc::channel(1);
    let power_handler = client.add_room_event_handler(
        timeline.room().room_id(),
        move |_event: matrix_sdk::ruma::events::room::power_levels::SyncRoomPowerLevelsEvent| {
            let power_authority_tx = power_authority_tx.clone();
            async move {
                let _ = power_authority_tx.try_send(());
            }
        },
    );
    let power_handler = client.event_handler_drop_guard(power_handler);
    tokio::spawn(async move {
        let _power_handler = power_handler;
        let room = timeline.room();
        let own_user_id = own_user_id.as_deref();
        let mut room_info = room.subscribe_info();
        let mut read_receipts = timeline.subscribe_own_user_read_receipts_changed().await;
        let live_focus = view.focus() == ViewFocus::Live;
        let mut pagination_updates: std::pin::Pin<
            Box<dyn futures_util::Stream<Item = PaginationStatus> + Send>,
        > = match timeline.live_back_pagination_status().await {
            Some((current, stream)) => {
                view.publish(ViewChange {
                    backward: Some(backward_page_state(current)),
                    ..ViewChange::default()
                });
                Box::pin(stream)
            }
            None => Box::pin(stream::pending()),
        };

        // The SDK intentionally leaves sender profiles unresolved until the
        // room member list is synchronized. Hydrate it without blocking live
        // timeline diffs; profile completion is emitted back through `updates`
        // as Set operations, so an existing session gains avatars/display names
        // without requiring a logout/login cycle.
        let member_hydration = async {
            if hydrate_sender_profiles {
                timeline.fetch_members().await;
            }
        };
        tokio::pin!(member_hydration);
        let mut members_hydrated = !hydrate_sender_profiles;

        futures_util::pin_mut!(updates);

        while !view.is_expired() {
            // Biased: queued SDK diffs publish before metadata that describes
            // them (a pagination status) and before a settle request is
            // answered.
            tokio::select! {
                biased;
                Some(diffs) = updates.next() => {
                    for diff in diffs.iter().cloned() {
                        diff.apply(&mut items);
                    }
                    // The SDK clears a live room's linked chunk after a gappy
                    // sync, which reopens history this stream may have
                    // reached. Its status stream is deduplicated, so a stream
                    // latched by its own page may never see that `Idle`.
                    let unlatch_backward = live_focus
                        && diffs
                            .iter()
                            .any(|diff| matches!(diff, VectorDiff::Clear | VectorDiff::Reset { .. }));
                    // Authority is recomputed only when the room's power
                    // levels or info change (the branches below).
                    let action_authority = view.action_authority();
                    let mut ops = {
                        let mut registry = media.lock().await;
                        registry.retain_items(items.iter().map(|item| item.unique_id().0.as_str()));
                        project_timeline_diffs_with_media(
                            &diffs,
                            own_user_id,
                            action_authority,
                            &mut registry,
                        )
                    };
                    annotations.fill_ops(&mut ops);
                    let mut ops = view.without_unchanged_sets(ops);
                    let reopens_history = unlatch_backward
                        && view.pagination().backward == TimelinePageState::Exhausted;
                    if ops.is_empty() && !reopens_history {
                        continue;
                    }
                    enrich_view_delta_ops(room, &mut ops).await;
                    annotations.remember_ops(&ops);
                    let position = view.position();
                    let read_state = project_live_read_state(&timeline, &position, own_user_id).await;
                    view.publish(ViewChange {
                        ops,
                        read_state: Some(read_state),
                        read_state_position: Some(position),
                        unlatch_backward,
                        ..ViewChange::default()
                    });
                }
                () = &mut member_hydration, if !members_hydrated => {
                    members_hydrated = true;
                }
                Some(_) = room_info.next() => {
                    let position = view.position();
                    let read_state = project_live_read_state(&timeline, &position, own_user_id).await;
                    let pinned_event_ids = project_pinned_event_ids(room);
                    let action_authority = room_action_authority(room, own_user_id).await;
                    let read_changed = read_state != view.read_state();
                    let pins_changed = pinned_event_ids != view.pinned_event_ids();
                    let authority_changed = action_authority != view.action_authority();
                    if !read_changed && !pins_changed && !authority_changed {
                        continue;
                    }
                    let ops = if authority_changed {
                        let rows = reproject_view_window(
                            room,
                            &items,
                            own_user_id,
                            action_authority,
                            &media,
                            &mut annotations,
                        )
                        .await;
                        vec![super::TimelineViewDeltaOp::Reset { rows }]
                    } else {
                        Vec::new()
                    };
                    view.publish(ViewChange {
                        ops,
                        read_state: read_changed.then_some(read_state),
                        read_state_position: Some(position),
                        pinned_event_ids: pins_changed.then_some(pinned_event_ids),
                        action_authority: authority_changed.then_some(action_authority),
                        ..ViewChange::default()
                    });
                }
                Some(()) = power_authority_rx.recv() => {
                    let action_authority = room_action_authority(room, own_user_id).await;
                    if action_authority == view.action_authority() {
                        continue;
                    }
                    let rows = reproject_view_window(
                        room,
                        &items,
                        own_user_id,
                        action_authority,
                        &media,
                        &mut annotations,
                    )
                    .await;
                    view.publish(ViewChange {
                        ops: vec![super::TimelineViewDeltaOp::Reset { rows }],
                        action_authority: Some(action_authority),
                        ..ViewChange::default()
                    });
                }
                Some(()) = read_receipts.next() => {
                    let position = view.position();
                    let read_state = project_live_read_state(&timeline, &position, own_user_id).await;
                    if read_state == view.read_state() {
                        continue;
                    }
                    view.publish(ViewChange {
                        read_state: Some(read_state),
                        read_state_position: Some(position),
                        ..ViewChange::default()
                    });
                }
                Some(status) = pagination_updates.next() => {
                    view.publish(ViewChange {
                        backward: Some(backward_page_state(status)),
                        ..ViewChange::default()
                    });
                }
                Some(settled) = settle_requests.recv() => {
                    let _ = settled.send(());
                }
                else => break,
            }
        }
    })
}

fn pagination_state_from_hit_start(hit_start: bool) -> TimelinePaginationState {
    TimelinePaginationState {
        backward: if hit_start {
            TimelinePageState::Exhausted
        } else {
            TimelinePageState::Available
        },
        forward: TimelinePageState::Available,
    }
}

/// Map a live provider's SDK back-pagination status. Every `Idle` is
/// authoritative, so `Idle { hit_timeline_start: false }` un-latches a stream
/// whose history the SDK reopened.
fn backward_page_state(status: PaginationStatus) -> TimelinePageState {
    match status {
        PaginationStatus::Paginating => TimelinePageState::Loading,
        PaginationStatus::Idle {
            hit_timeline_start: true,
        } => TimelinePageState::Exhausted,
        PaginationStatus::Idle {
            hit_timeline_start: false,
        } => TimelinePageState::Available,
    }
}

async fn project_live_read_state(
    timeline: &Timeline,
    position: &TimelineViewPosition,
    own_user_id: Option<&UserId>,
) -> TimelineReadState {
    let (own_read_event_id, visible_anchor) = navigation_read_state(timeline, own_user_id).await;
    let unread_anchor_event_id = visible_anchor.or_else(|| match position {
        TimelineViewPosition::Unread { anchor_event_id } => Some(anchor_event_id.clone()),
        _ => own_read_event_id.clone(),
    });
    // Capture the receipt frontier BEFORE observing displayed items. If a new
    // message races between these reads, its displayed tail will be paired with
    // an older receipt target, which the exact-target write rejects. Reversing
    // these reads could pair an unseen new message with an old displayed row.
    let receipt_tail_event_id = timeline.latest_event_id().await.map(|id| id.to_string());
    let visible_tail_event_id = timeline.items().await.iter().rev().find_map(|item| {
        item.as_event()
            .filter(|event| !event.is_local_echo())
            .and_then(|event| event.event_id().map(ToString::to_string))
    });
    TimelineReadState {
        visible_tail_event_id,
        receipt_tail_event_id,
        own_read_event_id,
        unread_anchor_event_id,
        is_marked_unread: timeline.room().is_marked_unread(),
    }
}

/// Project the room's current `m.room.pinned_events` list as product event ids.
fn project_pinned_event_ids(room: &Room) -> Vec<String> {
    room.pinned_event_ids()
        .unwrap_or_default()
        .into_iter()
        .map(|event_id| event_id.to_string())
        .collect()
}

async fn user_can_pin_events(room: &Room, own_user_id: Option<&UserId>) -> bool {
    room_action_authority(room, own_user_id)
        .await
        .can_pin_events
}

async fn room_action_authority(
    room: &Room,
    own_user_id: Option<&UserId>,
) -> TimelineRoomActionAuthority {
    let Some(user_id) = own_user_id else {
        return TimelineRoomActionAuthority::default();
    };
    let Ok(levels) = room.power_levels().await else {
        return TimelineRoomActionAuthority::default();
    };
    TimelineRoomActionAuthority {
        can_pin_events: levels.user_can_send_state(user_id, StateEventType::RoomPinnedEvents),
        can_redact_own: levels.user_can_redact_own_event(user_id),
        can_redact_other: levels.user_can_redact_event_of_other(user_id),
    }
}

fn reaction_contains_event_id(
    reaction: &NativeTimelineReaction,
    reaction_event_id: &OwnedEventId,
) -> bool {
    reaction
        .senders
        .iter()
        .any(|sender| sender.reaction_event_id.as_deref() == Some(reaction_event_id.as_str()))
}

fn parse_room_id(room_id: &str) -> Result<OwnedRoomId, &'static str> {
    OwnedRoomId::try_from(room_id.trim()).map_err(|_| "d0.3-timeline-invalid-room-id")
}

fn parse_action_room_id(room_id: &str) -> Result<OwnedRoomId, &'static str> {
    OwnedRoomId::try_from(room_id.trim()).map_err(|_| "d0.4-send-invalid-room-id")
}

fn parse_event_id(event_id: &str) -> Result<OwnedEventId, &'static str> {
    OwnedEventId::try_from(event_id.trim()).map_err(|_| "v-crypto.6-invalid-event-id")
}

fn parse_thread_root_event_id_for_open(event_id: &str) -> Result<OwnedEventId, &'static str> {
    let trimmed = event_id.trim();
    if trimmed.is_empty() {
        return Err("v-timeline-thread-root-invalid");
    }
    OwnedEventId::try_from(trimmed).map_err(|_| "v-timeline-thread-root-invalid")
}

fn parse_action_event_id(
    event_id: &str,
    diagnostic_id: &'static str,
) -> Result<OwnedEventId, &'static str> {
    OwnedEventId::try_from(event_id.trim()).map_err(|_| diagnostic_id)
}

fn normalize_edit_formatted_body(
    body: &str,
    formatted_body: Option<&str>,
) -> Result<Option<String>, &'static str> {
    let formatted_body = formatted_body
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|html| should_attach_formatted_body(body, Some(html)));
    crate::app::send::validate_outbound_text_payload(body, formatted_body)?;
    Ok(formatted_body.map(str::to_owned))
}

async fn validate_forward_encryption(
    source_room: &Room,
    target_room: &Room,
    confirmed_encryption_downgrade: bool,
) -> Result<(), &'static str> {
    let source = project_forward_encryption_read(
        source_room.latest_encryption_state().await,
        "v-timeline-forward-source-encryption-unavailable",
    )?;
    let target = project_forward_encryption_read(
        target_room.latest_encryption_state().await,
        "v-timeline-forward-target-encryption-unavailable",
    )?;
    validate_forward_encryption_status(source, target, confirmed_encryption_downgrade)
}

fn project_forward_encryption_read<E>(
    result: Result<EncryptionState, E>,
    unavailable_diagnostic: &'static str,
) -> Result<RoomEncryptionStatus, &'static str> {
    result
        .map(project_forward_encryption_state)
        .map_err(|_| unavailable_diagnostic)
}

fn project_forward_encryption_state(state: EncryptionState) -> RoomEncryptionStatus {
    if state.is_unknown() {
        RoomEncryptionStatus::Unknown
    } else if state.is_encrypted() {
        RoomEncryptionStatus::Encrypted
    } else {
        RoomEncryptionStatus::NotEncrypted
    }
}

fn validate_forward_encryption_status(
    source: RoomEncryptionStatus,
    target: RoomEncryptionStatus,
    confirmed_encryption_downgrade: bool,
) -> Result<(), &'static str> {
    if source == RoomEncryptionStatus::Unknown {
        return Err("v-timeline-forward-source-encryption-unavailable");
    }
    if target == RoomEncryptionStatus::Unknown {
        return Err("v-timeline-forward-target-encryption-unavailable");
    }
    if source == RoomEncryptionStatus::Encrypted
        && target == RoomEncryptionStatus::NotEncrypted
        && !confirmed_encryption_downgrade
    {
        return Err("v-timeline-forward-encryption-downgrade-not-confirmed");
    }
    Ok(())
}

async fn load_forwardable_text(
    room: &Room,
    event_id: &OwnedEventId,
) -> Result<(String, String), &'static str> {
    let timeline_event = room
        .load_or_fetch_event(event_id, None)
        .await
        .map_err(|_| "v-timeline-forward-event-unavailable")?;
    let sync_event = timeline_event
        .raw()
        .deserialize()
        .map_err(|_| "v-timeline-forward-event-decode-failed")?;
    match sync_event {
        AnySyncTimelineEvent::MessageLike(AnySyncMessageLikeEvent::RoomMessage(message)) => {
            let original = message
                .as_original()
                .ok_or("v-timeline-forward-event-redacted")?;
            let body = forwardable_text_body(&original.content.msgtype)
                .ok_or("v-timeline-forward-unsupported-event")?;
            Ok((original.sender.to_string(), body.to_owned()))
        }
        _ => Err("v-timeline-forward-unsupported-event"),
    }
}

fn forwardable_text_body(msgtype: &MessageType) -> Option<&str> {
    match msgtype {
        MessageType::Text(content) => Some(&content.body),
        MessageType::Notice(content) => Some(&content.body),
        MessageType::Emote(content) => Some(&content.body),
        _ => None,
    }
}

async fn load_forwardable_media(
    room: &Room,
    event_id: &OwnedEventId,
) -> Result<AnyMessageLikeEventContent, &'static str> {
    let timeline_event = room
        .load_or_fetch_event(event_id, None)
        .await
        .map_err(|_| "v-timeline-forward-media-event-unavailable")?;
    let sync_event = timeline_event
        .raw()
        .deserialize()
        .map_err(|_| "v-timeline-forward-media-event-decode-failed")?;
    match sync_event {
        AnySyncTimelineEvent::MessageLike(AnySyncMessageLikeEvent::RoomMessage(message)) => {
            let original = message
                .as_original()
                .ok_or("v-timeline-forward-media-event-redacted")?;
            let sender = original.sender.to_string();
            let mut msgtype = original.content.msgtype.clone();
            match &mut msgtype {
                MessageType::Image(content) => {
                    content.body = format_forwarded_media_body(&sender, &content.body);
                }
                MessageType::File(content) => {
                    content.body = format_forwarded_media_body(&sender, &content.body);
                }
                MessageType::Audio(content) => {
                    content.body = format_forwarded_media_body(&sender, &content.body);
                }
                MessageType::Video(content) => {
                    content.body = format_forwarded_media_body(&sender, &content.body);
                }
                _ => return Err("v-timeline-forward-media-unsupported-event"),
            }
            Ok(AnyMessageLikeEventContent::RoomMessage(
                RoomMessageEventContent::new(msgtype),
            ))
        }
        AnySyncTimelineEvent::MessageLike(AnySyncMessageLikeEvent::Sticker(sticker)) => {
            let original = sticker
                .as_original()
                .ok_or("v-timeline-forward-media-event-redacted")?;
            let sender = original.sender.to_string();
            Ok(AnyMessageLikeEventContent::Sticker(
                StickerEventContent::with_source(
                    format_forwarded_media_body(&sender, &original.content.body),
                    original.content.info.clone(),
                    original.content.source.clone(),
                ),
            ))
        }
        _ => Err("v-timeline-forward-media-unsupported-event"),
    }
}

async fn load_reply_draft_preview(
    room: &Room,
    event_id: &OwnedEventId,
    start_thread: bool,
) -> Result<NativeComposerReplyDraft, &'static str> {
    let timeline_event = room
        .load_or_fetch_event(event_id, None)
        .await
        .map_err(|_| "v-timeline-reply-draft-event-unavailable")?;
    let sync_event = timeline_event
        .raw()
        .deserialize()
        .map_err(|_| "v-timeline-reply-draft-event-decode-failed")?;
    match sync_event {
        AnySyncTimelineEvent::MessageLike(AnySyncMessageLikeEvent::RoomMessage(message)) => {
            let original = message
                .as_original()
                .ok_or("v-timeline-reply-draft-event-redacted")?;
            let body = original.content.body().to_owned();
            let formatted_body = match original.content.msgtype {
                MessageType::Text(ref content) => content.formatted.as_ref(),
                MessageType::Notice(ref content) => content.formatted.as_ref(),
                MessageType::Emote(ref content) => content.formatted.as_ref(),
                _ => None,
            }
            .filter(|formatted| formatted.format == MessageFormat::Html)
            .map(|formatted| formatted.body.trim().to_owned())
            .filter(|html| !html.is_empty() && html != body.trim());
            let existing_thread_root = match &original.content.relates_to {
                Some(Relation::Thread(thread)) => Some(thread.event_id.to_string()),
                _ => None,
            };
            Ok(reply_draft_from_parts(
                event_id,
                original.sender.as_str(),
                body,
                formatted_body,
                existing_thread_root,
                start_thread,
            ))
        }
        AnySyncTimelineEvent::MessageLike(AnySyncMessageLikeEvent::Sticker(sticker)) => {
            let original = sticker
                .as_original()
                .ok_or("v-timeline-reply-draft-event-redacted")?;
            Ok(sticker_reply_draft(
                event_id,
                original.sender.as_str(),
                &original.content,
                start_thread,
            ))
        }
        _ => Err("v-timeline-reply-draft-unsupported-event"),
    }
}

fn reply_draft_from_parts(
    event_id: &OwnedEventId,
    sender_id: &str,
    body: String,
    formatted_body: Option<String>,
    existing_thread_root: Option<String>,
    start_thread: bool,
) -> NativeComposerReplyDraft {
    // "Reply in thread" on a child stays in that child's existing thread.
    // Only a non-thread event can become a new thread root.
    let thread_root_event_id = if start_thread {
        existing_thread_root.or_else(|| Some(event_id.to_string()))
    } else {
        existing_thread_root
    };
    NativeComposerReplyDraft {
        draft_revision: 0,
        event_id: event_id.to_string(),
        sender_id: sender_id.to_owned(),
        body,
        formatted_body,
        thread_root_event_id,
    }
}

fn sticker_reply_draft(
    event_id: &OwnedEventId,
    sender_id: &str,
    content: &StickerEventContent,
    start_thread: bool,
) -> NativeComposerReplyDraft {
    let existing_thread_root = match &content.relates_to {
        Some(Relation::Thread(thread)) => Some(thread.event_id.to_string()),
        _ => None,
    };
    reply_draft_from_parts(
        event_id,
        sender_id,
        content.body.clone(),
        None,
        existing_thread_root,
        start_thread,
    )
}

fn validate_reaction_key(key: &str) -> Result<(), &'static str> {
    if key.trim().is_empty() || key.len() > 255 {
        return Err("v-send.2-reaction-invalid-key");
    }
    Ok(())
}

async fn snapshot_from_timeline(
    session_generation: u64,
    room_id: String,
    timeline: &Timeline,
    is_encrypted: bool,
    hit_start: bool,
    local_user: Option<&matrix_sdk::ruma::UserId>,
) -> Result<NativeTimelineSnapshot, &'static str> {
    let (items, _updates) = timeline.subscribe().await;
    let mut items: Vec<NativeTimelineItem> = items
        .iter()
        .filter_map(|item| project_item(item, local_user))
        .collect();
    enrich_native_items(timeline.room(), &mut items, false).await;
    Ok(NativeTimelineSnapshot {
        session_generation,
        room_id,
        is_encrypted,
        items,
        hit_start,
        utd: NativeUtdStatus {
            phase: NativeUtdPhase::Idle,
            pending_count: 0,
            unavailable_count: 0,
            recovered_count: 0,
        },
    })
}

fn project_item(
    item: &SdkTimelineItem,
    local_user: Option<&matrix_sdk::ruma::UserId>,
) -> Option<NativeTimelineItem> {
    let event = item.as_event()?;
    let event_id = event.event_id()?.to_string();
    let content = event.content();
    Some(NativeTimelineItem {
        item_id: item.unique_id().0.clone(),
        event_id,
        sender: event.sender().to_string(),
        event_type: safe_event_type(content),
        body: safe_body(content),
        origin_server_ts: event.timestamp().get().into(),
        decryption_state: decryption_state(content),
        reactions: project_reactions(content, local_user),
    })
}

fn project_reactions(
    content: &SdkTimelineItemContent,
    local_user: Option<&matrix_sdk::ruma::UserId>,
) -> Vec<NativeTimelineReaction> {
    content
        .reactions()
        .into_iter()
        .flat_map(|reactions| reactions.iter())
        .map(|(key, by_sender)| NativeTimelineReaction {
            key: key.clone(),
            count: by_sender.len().try_into().unwrap_or(u32::MAX),
            me: local_user.is_some_and(|user_id| by_sender.contains_key(user_id)),
            senders: by_sender
                .iter()
                .map(|(user_id, info)| NativeTimelineReactionSender {
                    user_id: user_id.to_string(),
                    // 0.19 dropped ReactionStatus. Local echoes that have been
                    // accepted expose Sent { event_id }. Remote reactions
                    // (send_state: None) are filled from the room event cache.
                    reaction_event_id: reaction_event_id_from_send_state(&info.send_state),
                })
                .collect(),
        })
        .collect()
}

fn decryption_state(content: &SdkTimelineItemContent) -> Option<NativeDecryptionState> {
    let encrypted = content.as_unable_to_decrypt()?;
    let unavailable = match encrypted {
        EncryptedMessage::MegolmV1AesSha2 { cause, .. } => is_currently_unavailable(*cause),
        EncryptedMessage::OlmV1Curve25519AesSha2 { .. } | EncryptedMessage::Unknown => true,
    };
    Some(if unavailable {
        NativeDecryptionState::Unavailable
    } else {
        NativeDecryptionState::Pending
    })
}

fn is_currently_unavailable(cause: UtdCause) -> bool {
    matches!(
        cause,
        UtdCause::SentBeforeWeJoined
            | UtdCause::HistoricalMessageAndBackupIsDisabled
            | UtdCause::WithheldBySender
    )
}

fn safe_event_type(content: &SdkTimelineItemContent) -> String {
    if content.is_redacted() {
        return "m.room.redacted".to_owned();
    }
    content
        .event_type_str()
        .unwrap_or_else(|| "m.room.unknown".to_owned())
}

fn safe_body(content: &SdkTimelineItemContent) -> String {
    safe_body_from_parts(
        content.is_redacted(),
        content.is_unable_to_decrypt(),
        content.as_message().map(|message| message.body()),
    )
}

fn safe_body_from_parts(redacted: bool, unable_to_decrypt: bool, body: Option<&str>) -> String {
    if redacted {
        REDACTED_PLACEHOLDER.to_owned()
    } else if unable_to_decrypt {
        UTD_PLACEHOLDER.to_owned()
    } else if let Some(body) = body {
        body.to_owned()
    } else {
        UNSUPPORTED_PLACEHOLDER.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_reasons_are_trimmed_bounded_and_never_echoed_in_diagnostics() {
        assert_eq!(
            normalize_timeline_action_reason(Some("  abuse  "), "too-long").unwrap(),
            Some("abuse".to_owned())
        );
        assert_eq!(
            normalize_timeline_action_reason(Some("  "), "too-long").unwrap(),
            None
        );
        assert_eq!(
            normalize_timeline_action_reason(Some(&"x".repeat(513)), "too-long"),
            Err("too-long")
        );
    }

    #[test]
    fn text_forward_owner_rejects_semantically_lossy_message_variants() {
        let text = MessageType::Text(
            matrix_sdk::ruma::events::room::message::TextMessageEventContent::plain("hello"),
        );
        let location = MessageType::Location(
            matrix_sdk::ruma::events::room::message::LocationMessageEventContent::new(
                "location".to_owned(),
                "geo:1,2".to_owned(),
            ),
        );
        assert_eq!(forwardable_text_body(&text), Some("hello"));
        assert_eq!(forwardable_text_body(&location), None);
    }

    #[test]
    fn forward_encryption_policy_fails_closed_and_requires_downgrade_confirmation() {
        use RoomEncryptionStatus::{Encrypted, NotEncrypted, Unknown};

        assert_eq!(
            validate_forward_encryption_status(Unknown, NotEncrypted, false),
            Err("v-timeline-forward-source-encryption-unavailable")
        );
        assert_eq!(
            validate_forward_encryption_status(NotEncrypted, Unknown, false),
            Err("v-timeline-forward-target-encryption-unavailable")
        );
        assert_eq!(
            validate_forward_encryption_status(Encrypted, NotEncrypted, false),
            Err("v-timeline-forward-encryption-downgrade-not-confirmed")
        );
        assert!(validate_forward_encryption_status(Encrypted, NotEncrypted, true).is_ok());
        assert!(validate_forward_encryption_status(NotEncrypted, NotEncrypted, false).is_ok());
        assert!(validate_forward_encryption_status(NotEncrypted, Encrypted, false).is_ok());
        assert!(validate_forward_encryption_status(Encrypted, Encrypted, false).is_ok());
        assert_eq!(
            project_forward_encryption_read::<()>(
                Err(()),
                "v-timeline-forward-source-encryption-unavailable"
            ),
            Err("v-timeline-forward-source-encryption-unavailable")
        );
        assert_eq!(
            project_forward_encryption_read::<()>(
                Err(()),
                "v-timeline-forward-target-encryption-unavailable"
            ),
            Err("v-timeline-forward-target-encryption-unavailable")
        );
    }

    #[test]
    fn poll_vote_selection_is_validated_by_core_semantics() {
        let answers = HashSet::from(["a".to_owned(), "b".to_owned()]);
        assert_eq!(
            validate_poll_vote_selection(vec!["a".to_owned()], &answers, 1, false).unwrap(),
            vec!["a".to_owned()]
        );
        assert_eq!(
            validate_poll_vote_selection(Vec::new(), &answers, 1, false).unwrap(),
            Vec::<String>::new()
        );
        assert_eq!(
            validate_poll_vote_selection(vec!["a".to_owned()], &answers, 1, true),
            Err("v-timeline-poll-vote-closed")
        );
        assert_eq!(
            validate_poll_vote_selection(vec!["a".to_owned(), "b".to_owned()], &answers, 1, false,),
            Err("v-timeline-poll-vote-too-many-answers")
        );
        assert_eq!(
            validate_poll_vote_selection(vec!["unknown".to_owned()], &answers, 1, false),
            Err("v-timeline-poll-vote-invalid-answer")
        );
        assert_eq!(
            validate_poll_vote_selection(vec!["a".to_owned(), "a".to_owned()], &answers, 2, false,),
            Err("v-timeline-poll-vote-duplicate-answer")
        );
        let oversized_id = "x".repeat(65);
        let oversized_set = HashSet::from([oversized_id.clone()]);
        let selected = validate_poll_vote_selection(vec![oversized_id], &oversized_set, 1, false)
            .expect("timeline semantics permit only an exact projected option before wire bounds");
        assert!(poll_response_content("$poll:example.org", &selected).is_err());

        let too_many = (0..21).map(|index| format!("a{index}")).collect::<Vec<_>>();
        let many_answers = too_many.iter().cloned().collect::<HashSet<_>>();
        let selected = validate_poll_vote_selection(too_many, &many_answers, 25, false)
            .expect("timeline max is checked independently from global wire bounds");
        assert!(poll_response_content("$poll:example.org", &selected).is_err());
    }

    fn assert_thread_reply(
        thread: &matrix_sdk::ruma::events::relation::Thread,
        root: &str,
        reply_to: &str,
    ) {
        assert_eq!(thread.event_id.as_str(), root);
        assert_eq!(
            thread
                .in_reply_to
                .as_ref()
                .map(|reply| reply.event_id.as_str()),
            Some(reply_to)
        );
        assert!(!thread.is_falling_back);
    }

    #[test]
    fn reply_in_thread_on_a_child_preserves_root_across_text_and_poll_builders() {
        let child = OwnedEventId::try_from("$child:example.org").unwrap();
        let draft = reply_draft_from_parts(
            &child,
            "@alice:example.org",
            "child message".to_owned(),
            None,
            Some("$root:example.org".to_owned()),
            true,
        );
        assert_eq!(draft.event_id, "$child:example.org");
        assert_eq!(
            draft.thread_root_event_id.as_deref(),
            Some("$root:example.org")
        );

        let reply_to = draft.event_id.parse().unwrap();
        let thread_root = draft.thread_root_event_id.clone().unwrap().parse().unwrap();
        let text = message_content(
            "reply".to_owned(),
            None,
            None,
            None,
            false,
            Some(reply_to),
            Some(thread_root),
        )
        .unwrap();
        match text.relates_to.as_ref() {
            Some(Relation::Thread(thread)) => {
                assert_thread_reply(thread, "$root:example.org", "$child:example.org")
            }
            relation => panic!("expected text thread reply, got {relation:?}"),
        }

        let normalized = normalize_poll("Continue?", &["Yes".into(), "No".into()], 1).unwrap();
        let mut poll = poll_start_content(&normalized).unwrap();
        apply_poll_start_relations(
            &mut poll,
            Some(draft.event_id.parse().unwrap()),
            Some(draft.thread_root_event_id.unwrap().parse().unwrap()),
        );
        match poll.relates_to.as_ref() {
            Some(matrix_sdk::ruma::events::room::message::RelationWithoutReplacement::Thread(
                thread,
            )) => assert_thread_reply(thread, "$root:example.org", "$child:example.org"),
            relation => panic!("expected poll thread reply, got {relation:?}"),
        }
    }

    #[test]
    fn reply_draft_thread_root_resolution_distinguishes_new_and_existing_threads() {
        let selected = OwnedEventId::try_from("$selected:example.org").unwrap();
        assert_eq!(
            reply_draft_from_parts(
                &selected,
                "@alice:example.org",
                "message".into(),
                None,
                None,
                true,
            )
            .thread_root_event_id
            .as_deref(),
            Some("$selected:example.org")
        );
        assert_eq!(
            reply_draft_from_parts(
                &selected,
                "@alice:example.org",
                "message".into(),
                None,
                Some("$root:example.org".into()),
                false,
            )
            .thread_root_event_id
            .as_deref(),
            Some("$root:example.org")
        );
        assert!(reply_draft_from_parts(
            &selected,
            "@alice:example.org",
            "message".into(),
            None,
            None,
            false,
        )
        .thread_root_event_id
        .is_none());
    }

    #[test]
    fn sticker_reply_draft_has_an_accurate_preview_and_preserves_its_thread() {
        let content: StickerEventContent = serde_json::from_value(serde_json::json!({
            "body": "Waving fox",
            "info": {},
            "url": "mxc://example.org/sticker",
            "m.relates_to": {
                "rel_type": "m.thread",
                "event_id": "$root:example.org",
                "m.in_reply_to": { "event_id": "$parent:example.org" }
            }
        }))
        .unwrap();
        let event_id = OwnedEventId::try_from("$sticker:example.org").unwrap();
        let draft = sticker_reply_draft(&event_id, "@alice:example.org", &content, true);
        assert_eq!(draft.event_id, "$sticker:example.org");
        assert_eq!(draft.sender_id, "@alice:example.org");
        assert_eq!(draft.body, "Waving fox");
        assert!(draft.formatted_body.is_none());
        assert_eq!(
            draft.thread_root_event_id.as_deref(),
            Some("$root:example.org")
        );
    }

    #[test]
    fn timeline_edit_uses_the_shared_combined_outbound_byte_cap() {
        let at_limit = "x".repeat(crate::app::send::MAX_OUTBOUND_TEXT_PAYLOAD_BYTES);
        assert_eq!(normalize_edit_formatted_body(&at_limit, None), Ok(None));

        let over_limit = "x".repeat(crate::app::send::MAX_OUTBOUND_TEXT_PAYLOAD_BYTES + 1);
        assert_eq!(
            normalize_edit_formatted_body(&over_limit, None),
            Err("d0.4-send-text-payload-too-large")
        );

        let body = "fallback";
        let html = "x".repeat(crate::app::send::MAX_OUTBOUND_TEXT_PAYLOAD_BYTES - body.len());
        assert_eq!(
            normalize_edit_formatted_body(body, Some(&html)),
            Ok(Some(html))
        );
        let html = "x".repeat(crate::app::send::MAX_OUTBOUND_TEXT_PAYLOAD_BYTES - body.len() + 1);
        assert_eq!(
            normalize_edit_formatted_body(body, Some(&html)),
            Err("d0.4-send-text-payload-too-large")
        );
    }

    #[test]
    fn reaction_redaction_requires_the_selected_annotation_event_id() {
        let reaction = NativeTimelineReaction {
            key: "✅".into(),
            count: 1,
            me: false,
            senders: vec![NativeTimelineReactionSender {
                user_id: "@alice:example.org".into(),
                reaction_event_id: Some("$reaction:example.org".into()),
            }],
        };
        let selected = OwnedEventId::try_from("$reaction:example.org").unwrap();
        let unrelated = OwnedEventId::try_from("$unrelated:example.org").unwrap();

        assert!(reaction_contains_event_id(&reaction, &selected));
        assert!(!reaction_contains_event_id(&reaction, &unrelated));
    }

    #[test]
    fn recovered_remote_annotation_id_is_enough_for_redact_validation() {
        let mut reaction = NativeTimelineReaction {
            key: "👍".into(),
            count: 1,
            me: false,
            senders: vec![NativeTimelineReactionSender {
                user_id: "@bob:example.org".into(),
                reaction_event_id: None,
            }],
        };
        let recovered = OwnedEventId::try_from("$bob-reaction:example.org").unwrap();
        assert!(!reaction_contains_event_id(&reaction, &recovered));
        reaction.senders[0].reaction_event_id = Some(recovered.to_string());
        assert!(reaction_contains_event_id(&reaction, &recovered));
    }

    #[test]
    fn invalid_room_ids_are_rejected_before_sdk_lookup() {
        assert_eq!(
            parse_room_id("not-a-room").unwrap_err(),
            "d0.3-timeline-invalid-room-id"
        );
    }

    #[test]
    fn reaction_keys_are_validated_before_any_sdk_write() {
        assert_eq!(
            validate_reaction_key(" ").unwrap_err(),
            "v-send.2-reaction-invalid-key"
        );
        assert!(validate_reaction_key("✅").is_ok());
    }

    #[test]
    fn reaction_keys_reject_empty_and_overlong_values() {
        assert_eq!(
            validate_reaction_key("").unwrap_err(),
            "v-send.2-reaction-invalid-key"
        );
        assert_eq!(
            validate_reaction_key(&"x".repeat(256)).unwrap_err(),
            "v-send.2-reaction-invalid-key"
        );
        assert!(validate_reaction_key(&"x".repeat(255)).is_ok());
    }

    #[test]
    fn reaction_event_ids_are_validated_before_sdk_write() {
        assert_eq!(
            parse_event_id("not-an-event").unwrap_err(),
            "v-crypto.6-invalid-event-id"
        );
        assert!(parse_event_id("$event:example.org").is_ok());
    }

    #[test]
    fn normal_open_prefers_native_unread_frontier_over_a_restored_anchor() {
        let selected = resolve_normal_open_position(
            true,
            Some("$fully-read:example.org".into()),
            None,
            1_700_000_000_000,
            &NativeTimelineViewportHint {
                restored_anchor_event_id: Some("$restored:example.org".into()),
                updated_at_ms: Some(1_700_000_000_000),
                ..NativeTimelineViewportHint::default()
            },
        )
        .unwrap();
        assert_eq!(
            selected,
            TimelineViewPosition::Unread {
                anchor_event_id: "$fully-read:example.org".into(),
            }
        );
    }

    #[test]
    fn normal_open_uses_a_validated_restored_anchor_only_when_no_unread_frontier_exists() {
        assert_eq!(
            resolve_normal_open_position(
                false,
                None,
                None,
                1_700_000_000_000,
                &NativeTimelineViewportHint {
                    restored_anchor_event_id: Some("$restored:example.org".into()),
                    updated_at_ms: Some(1_700_000_000_000),
                    ..NativeTimelineViewportHint::default()
                },
            )
            .unwrap(),
            TimelineViewPosition::Restored {
                anchor_event_id: Some("$restored:example.org".into()),
            }
        );
        assert_eq!(
            resolve_normal_open_position(
                false,
                None,
                None,
                1_700_000_000_000,
                &NativeTimelineViewportHint {
                    restored_anchor_event_id: Some("not-an-event".into()),
                    updated_at_ms: Some(1_700_000_000_000),
                    ..NativeTimelineViewportHint::default()
                },
            )
            .unwrap(),
            TimelineViewPosition::LiveBottom
        );
    }

    #[test]
    fn normal_open_falls_back_to_live_bottom_when_unread_has_no_frontier() {
        // A supported channel with unread activity but no `m.fully_read`
        // marker (a room this device has never opened/read) must still open.
        assert_eq!(
            resolve_normal_open_position(
                true,
                None,
                None,
                1_700_000_000_000,
                &NativeTimelineViewportHint {
                    restored_anchor_event_id: Some("$restored:example.org".into()),
                    updated_at_ms: Some(1_700_000_000_000),
                    ..NativeTimelineViewportHint::default()
                },
            ),
            Ok(TimelineViewPosition::LiveBottom)
        );
    }

    #[test]
    fn normal_open_keeps_unread_frontier_when_present() {
        assert_eq!(
            resolve_normal_open_position(
                true,
                Some("$fully-read:example.org".into()),
                None,
                1_700_000_000_000,
                &NativeTimelineViewportHint {
                    restored_anchor_event_id: Some("$restored:example.org".into()),
                    updated_at_ms: Some(1_700_000_000_000),
                    ..NativeTimelineViewportHint::default()
                },
            )
            .unwrap(),
            TimelineViewPosition::Unread {
                anchor_event_id: "$fully-read:example.org".into(),
            }
        );
    }

    #[test]
    fn normal_open_restores_live_bottom_when_unread_tip_still_matches() {
        assert_eq!(
            resolve_normal_open_position(
                true,
                Some("$fully-read:example.org".into()),
                Some("$tail:example.org"),
                1_700_000_000_000,
                &NativeTimelineViewportHint {
                    at_bottom: true,
                    live_tail_event_id: Some("$tail:example.org".into()),
                    ..NativeTimelineViewportHint::default()
                },
            )
            .unwrap(),
            TimelineViewPosition::LiveBottom
        );
    }

    #[test]
    fn newest_live_frontier_picks_the_latest_candidate_in_the_window() {
        let live = vec![
            "$old-fully-read:example.org".into(),
            "$mid:example.org".into(),
            "$receipt:example.org".into(),
            "$live-tip:example.org".into(),
        ];
        assert_eq!(
            newest_frontier_in_live(
                &live,
                ["$old-fully-read:example.org", "$receipt:example.org"],
            )
            .as_deref(),
            Some("$receipt:example.org")
        );
    }

    #[test]
    fn newest_live_frontier_ignores_candidates_outside_the_window() {
        let live = vec!["$recent:example.org".into(), "$tip:example.org".into()];
        assert_eq!(
            newest_frontier_in_live(&live, ["$july-fully-read:example.org"]),
            None
        );
        assert_eq!(
            newest_frontier_in_live(
                &live,
                ["$july-fully-read:example.org", "$recent:example.org"],
            )
            .as_deref(),
            Some("$recent:example.org")
        );
        assert_eq!(newest_frontier_in_live(&live, [] as [&str; 0]), None);
    }

    #[test]
    fn last_read_in_live_is_available_without_focused_provider() {
        assert_eq!(
            plan_unread_open(&["$a".into(), "$b".into()], Some("$a")),
            LastReadOpenPlan::InLive {
                event_id: "$a".into()
            }
        );
    }

    #[test]
    fn missing_last_read_is_preserved_as_explicit_navigation_target() {
        assert_eq!(
            plan_unread_open(&["$b".into()], Some("$old")),
            LastReadOpenPlan::Unavailable {
                event_id: "$old".into()
            }
        );
    }

    #[test]
    fn hidden_frontier_uses_stream_predecessor_not_edited_message() {
        let raw = ["$old", "$new", "$edit-old", "$reaction"].map(String::from);
        let visible = ["$old", "$new"].map(String::from);
        assert_eq!(
            visible_predecessor(&raw, &visible, "$edit-old"),
            Some("$new".into())
        );
        assert_eq!(
            visible_predecessor(&raw, &visible, "$reaction"),
            Some("$new".into())
        );
        assert_eq!(visible_predecessor(&raw, &visible, "$unknown"), None);
        assert_eq!(
            visible_predecessor(&raw, &visible, "$old"),
            Some("$old".into())
        );
    }

    #[test]
    fn focused_open_on_unrendered_marker_resolves_to_rendered_predecessor() {
        let rendered = vec![
            ("$a".to_owned(), 100u64),
            ("$b".to_owned(), 200),
            ("$c".to_owned(), 300),
        ];
        // A rendered target is returned unchanged.
        assert_eq!(
            rendered_focus_anchor(&rendered, "$b", Some(200)),
            Some("$b".into())
        );
        // A reaction sent between $b and $c lands on $b.
        assert_eq!(
            rendered_focus_anchor(&rendered, "$reaction", Some(250)),
            Some("$b".into())
        );
        // A marker older than the whole window lands on its oldest row.
        assert_eq!(
            rendered_focus_anchor(&rendered, "$ancient", Some(5)),
            Some("$a".into())
        );
        // Without a timestamp the newest rendered row is the closest guess.
        assert_eq!(
            rendered_focus_anchor(&rendered, "$mystery", None),
            Some("$c".into())
        );
        // An empty window cannot be resolved; the caller keeps the raw target.
        assert_eq!(rendered_focus_anchor(&[], "$x", Some(1)), None);
    }

    #[test]
    fn normal_open_falls_back_to_live_when_unread_frontier_is_outside_live_graph() {
        assert_eq!(
            resolve_normal_open_position(
                true,
                None,
                None,
                1_700_000_000_000,
                &NativeTimelineViewportHint::default(),
            ),
            Ok(TimelineViewPosition::LiveBottom)
        );
    }

    #[test]
    fn normal_open_ignores_stale_historical_restore_hints() {
        let now = 1_700_000_000_000_u64;
        assert_eq!(
            resolve_normal_open_position(
                false,
                None,
                None,
                now,
                &NativeTimelineViewportHint {
                    restored_anchor_event_id: Some("$restored:example.org".into()),
                    updated_at_ms: Some(now - NATIVE_TIMELINE_VIEWPORT_RESTORE_TTL_MS - 1),
                    ..NativeTimelineViewportHint::default()
                },
            )
            .unwrap(),
            TimelineViewPosition::LiveBottom
        );
    }

    #[test]
    fn view_delta_batch_can_carry_live_read_and_pagination_metadata() {
        let batch = TimelineViewDeltaBatch {
            schema_version: TIMELINE_VIEW_SCHEMA_VERSION,
            session_generation: 3,
            stream_id: "live:!room:example.org:1".into(),
            room_id: "!room:example.org".into(),
            revision: 4,
            ops: Vec::new(),
            read_state: Some(TimelineReadState {
                visible_tail_event_id: None,
                receipt_tail_event_id: None,
                own_read_event_id: Some("$read:example.org".into()),
                unread_anchor_event_id: None,
                is_marked_unread: false,
            }),
            pagination: Some(TimelinePaginationState {
                backward: TimelinePageState::Exhausted,
                forward: TimelinePageState::Available,
            }),
            pinned_event_ids: Some(vec!["$pin:example.org".into()]),
        };
        let json = serde_json::to_value(&batch).unwrap();
        assert_eq!(json["revision"], 4);
        assert!(json["ops"].as_array().unwrap().is_empty());
        assert_eq!(json["readState"]["ownReadEventId"], "$read:example.org");
        assert_eq!(json["pagination"]["backward"], "exhausted");

        assert_eq!(
            backward_page_state(PaginationStatus::Idle {
                hit_timeline_start: true
            }),
            TimelinePageState::Exhausted
        );
        assert_eq!(
            backward_page_state(PaginationStatus::Paginating),
            TimelinePageState::Loading
        );
        // The SDK un-latches after shrinking the room to its newest chunk.
        assert_eq!(
            backward_page_state(PaginationStatus::Idle {
                hit_timeline_start: false
            }),
            TimelinePageState::Available
        );
    }

    #[test]
    fn view_subscription_keys_keep_focuses_isolated_from_live_timeline() {
        assert_eq!(
            view_subscription_key("!room:example.org", &TimelineViewPosition::LiveBottom),
            "live:!room:example.org"
        );
        assert_eq!(
            view_subscription_key(
                "!room:example.org",
                &TimelineViewPosition::Focused {
                    target_event_id: "$one:example.org".into()
                }
            ),
            "focused:!room:example.org:$one:example.org"
        );
        assert_eq!(
            view_subscription_key(
                "!room:example.org",
                &TimelineViewPosition::Unread {
                    anchor_event_id: "$one:example.org".into(),
                }
            ),
            "unread:!room:example.org"
        );
        let focused = view_subscription_key(
            "!room:example.org",
            &TimelineViewPosition::Focused {
                target_event_id: "$root:example.org".into(),
            },
        );
        let thread = view_subscription_key(
            "!room:example.org",
            &TimelineViewPosition::Thread {
                root_event_id: "$root:example.org".into(),
            },
        );
        assert_eq!(focused, "focused:!room:example.org:$root:example.org");
        assert_eq!(thread, "thread:!room:example.org:$root:example.org");
        assert_ne!(focused, thread);
    }

    #[test]
    fn thread_root_open_rejects_invalid_id_shapes_without_echoing_the_id() {
        assert_eq!(
            parse_thread_root_event_id_for_open("").unwrap_err(),
            "v-timeline-thread-root-invalid"
        );
        assert_eq!(
            parse_thread_root_event_id_for_open("   ").unwrap_err(),
            "v-timeline-thread-root-invalid"
        );
        assert_eq!(
            parse_thread_root_event_id_for_open("not-an-event").unwrap_err(),
            "v-timeline-thread-root-invalid"
        );
        assert!(parse_thread_root_event_id_for_open("$root:example.org").is_ok());
        assert!(!parse_thread_root_event_id_for_open("not-an-event")
            .unwrap_err()
            .contains("not-an-event"));
    }

    #[test]
    fn pagination_request_targets_the_opened_stream_not_a_room() {
        let request: NativeTimelineViewPaginationRequest =
            serde_json::from_value(serde_json::json!({
                "streamId": "focused:!room:example.org:$one:example.org",
                "direction": "backwards"
            }))
            .unwrap();
        assert_eq!(
            request.stream_id,
            "focused:!room:example.org:$one:example.org"
        );
        assert_eq!(request.direction, NativeTimelineDirection::Backwards);
    }

    #[test]
    fn read_state_request_targets_the_opened_stream_and_action() {
        let request: NativeTimelineReadStateRequest = serde_json::from_value(serde_json::json!({
            "streamId": "live:!room:example.org",
            "action": "mark_unread",
            "intent": "explicit_user"
        }))
        .unwrap();
        assert_eq!(request.stream_id, "live:!room:example.org");
        assert_eq!(request.action, NativeTimelineReadAction::MarkUnread);
        assert_eq!(request.intent, NativeTimelineReadIntent::ExplicitUser);
    }

    #[test]
    fn automatic_read_only_acknowledges_the_exact_current_live_tail() {
        let observed = "$observed:example.org";
        let observed_id = OwnedEventId::try_from(observed).unwrap();
        assert_eq!(
            plan_live_read_target(
                Some(observed_id.clone()),
                NativeTimelineReadIntent::AutomaticVisibility,
                Some(observed)
            ),
            Ok(LiveReadTargetPlan::Send(observed_id))
        );

        let newer = OwnedEventId::try_from("$newer:example.org").unwrap();
        assert_eq!(
            plan_live_read_target(
                Some(newer),
                NativeTimelineReadIntent::AutomaticVisibility,
                Some(observed)
            ),
            Ok(LiveReadTargetPlan::NoOp),
            "a newer SDK tail must never be acknowledged by an older visibility observation"
        );
        assert_eq!(
            plan_live_read_target(
                None,
                NativeTimelineReadIntent::AutomaticVisibility,
                Some(observed)
            ),
            Ok(LiveReadTargetPlan::NoOp)
        );
    }

    #[test]
    fn folded_event_frontier_targets_the_receipt_event_and_rejects_later_arrivals() {
        // matrix-sdk-ui 0.18 latest_event_id includes folded edits/reactions;
        // the displayed message ID deliberately remains unchanged. Platforms
        // echo the Core-projected receipt identity, never the message row ID.
        for frontier in ["$edit:example.org", "$reaction:example.org"] {
            let receipt_id = OwnedEventId::try_from(frontier).unwrap();
            assert_eq!(
                plan_live_read_target(
                    Some(receipt_id.clone()),
                    NativeTimelineReadIntent::AutomaticVisibility,
                    Some(frontier)
                ),
                Ok(LiveReadTargetPlan::Send(receipt_id))
            );
            assert_eq!(
                plan_live_read_target(
                    Some(OwnedEventId::try_from("$new-message:example.org").unwrap()),
                    NativeTimelineReadIntent::AutomaticVisibility,
                    Some(frontier)
                ),
                Ok(LiveReadTargetPlan::NoOp),
                "new message after paired snapshot must never be marked unseen"
            );
        }
    }

    #[test]
    fn read_intent_contract_rejects_ambiguous_targets() {
        assert_eq!(
            plan_live_read_target(None, NativeTimelineReadIntent::AutomaticVisibility, None),
            Err("v-timeline-read-observed-tail-required")
        );
        assert_eq!(
            plan_live_read_target(
                None,
                NativeTimelineReadIntent::AutomaticVisibility,
                Some("not-an-event")
            ),
            Err("v-timeline-read-observed-tail-invalid")
        );
        assert_eq!(
            plan_live_read_target(
                None,
                NativeTimelineReadIntent::ExplicitUser,
                Some("$unexpected:example.org")
            ),
            Err("v-timeline-read-observed-tail-unexpected")
        );
        assert_eq!(
            plan_live_read_target(None, NativeTimelineReadIntent::ExplicitUser, None),
            Ok(LiveReadTargetPlan::ClearUnreadFlag)
        );
    }

    #[test]
    fn newest_frontier_in_live_picks_the_later_comparable_candidate() {
        let live = ["$follow-live-older".into(), "$follow-live-newer".into()];
        assert_eq!(
            newest_frontier_in_live(&live, ["$follow-live-older", "$follow-live-newer"]),
            Some("$follow-live-newer".into())
        );
    }

    #[test]
    fn restoration_frontier_loads_unthreaded_and_main_receipts() {
        let source = include_str!("live.rs");
        let helper = source
            .split("async fn own_unthreaded_and_main_receipt_event_ids")
            .nth(1)
            .and_then(|rest| rest.split("async fn navigation_read_state").next())
            .expect("own receipt helper");
        assert!(helper.contains("load_user_receipt"));
        assert!(helper.contains("ReceiptThread::Unthreaded"));
        assert!(helper.contains("ReceiptThread::Main"));
        assert!(helper.contains("EventReceiptType::ReadPrivate"));
        let mark_read_start = source.find("async fn mark_live_timeline_read").unwrap();
        let mark_read_end = source[mark_read_start..]
            .find("fn remember_agent_approval_decision")
            .map(|offset| mark_read_start + offset)
            .unwrap();
        let mark_read_source = &source[mark_read_start..mark_read_end];
        assert!(!mark_read_source.contains("ReceiptThread::Unthreaded"));
    }

    #[test]
    fn room_read_state_sends_receipts_and_clears_marked_unread_without_a_view_stream() {
        let source = include_str!("live.rs");
        assert!(source.contains("pub async fn set_room_read_state"));
        assert!(source.contains("self.open(client, &room_id_string).await"));
        assert!(source.contains("v-rooms-room-read-state-mark-read-failed"));
        assert!(source.contains(
            "mark_live_timeline_read(&timeline, NativeTimelineReadIntent::ExplicitUser, None)"
        ));
        assert!(source.contains("fully_read_marker(Some(event_id.clone()))"));
        assert!(source.contains("private_read_receipt(Some(event_id))"));
        assert!(source.contains("set_unread_flag(false)"));
        assert!(source.contains("set_unread_flag(true)"));
        assert!(source.contains("v-rooms-room-read-state-room-not-found"));
    }

    #[test]
    fn exact_read_receipts_target_one_event_for_server_counts_and_private_receipt() {
        let event_id = OwnedEventId::try_from("$tail:example.org").unwrap();
        let receipts = exact_read_receipts(event_id.clone());
        assert_eq!(receipts.fully_read.as_ref(), Some(&event_id));
        assert_eq!(receipts.private_read_receipt.as_ref(), Some(&event_id));
        assert!(receipts.public_read_receipt.is_none());

        let sent = room_readback_from_mark(&LiveTimelineReadMark {
            acknowledged_event_id: Some(event_id.clone()),
            unread_flag_cleared: false,
        })
        .expect("a sent fully-read marker is success");
        assert!(sent.receipt_sent);
        assert_eq!(
            sent.acknowledged_event_id.as_deref(),
            Some(event_id.as_str())
        );
        assert!(!sent.unread_flag_cleared);
        let cleared = room_readback_from_mark(&LiveTimelineReadMark {
            acknowledged_event_id: None,
            unread_flag_cleared: true,
        })
        .expect("clearing the unread flag without a receipt target is success");
        assert!(!cleared.receipt_sent);
        assert!(cleared.acknowledged_event_id.is_none());
        assert!(cleared.unread_flag_cleared);
        assert_eq!(
            room_readback_from_mark(&LiveTimelineReadMark {
                acknowledged_event_id: None,
                unread_flag_cleared: false,
            }),
            Err("v-rooms-room-read-state-mark-read-failed")
        );

        let cargo_lock = include_str!("../../../../../Cargo.lock");
        assert!(cargo_lock.contains("name = \"matrix-sdk-ui\"\nversion = \"0.19.1\""));
        let source = include_str!("live.rs");
        assert!(source.contains("Pinned matrix-sdk-ui 0.19 invariant"));
        assert!(source.contains("also when receipt deduplication removes every unchanged marker"));
        let mark_read_start = source.find("async fn mark_live_timeline_read").unwrap();
        let mark_read_end = source[mark_read_start..]
            .find("fn remember_agent_approval_decision")
            .map(|offset| mark_read_start + offset)
            .unwrap();
        let mark_read_source = &source[mark_read_start..mark_read_end];
        assert!(mark_read_source.contains("timeline.latest_event_id().await"));
        assert!(mark_read_source.contains("timeline.is_threaded()"));
        assert!(mark_read_source.contains("send_single_receipt(ReceiptType::ReadPrivate"));
        assert!(mark_read_source.contains("send_multiple_receipts(exact_read_receipts"));
        assert!(!mark_read_source.contains("ReceiptThread::Unthreaded"));
        assert!(!mark_read_source.contains("items.iter().rev().find_map"));
        assert!(source.contains("position_allows_mark_read(&stream.position)"));
        assert!(source.contains("TimelineViewPosition::LiveBottom | TimelineViewPosition::Thread"));
    }

    #[test]
    fn safe_body_projection_never_exposes_unavailable_event_content() {
        assert_eq!(
            safe_body_from_parts(true, false, Some("ignored")),
            REDACTED_PLACEHOLDER
        );
        assert_eq!(
            safe_body_from_parts(false, true, Some("ignored")),
            UTD_PLACEHOLDER
        );
        assert_eq!(
            safe_body_from_parts(false, false, None),
            UNSUPPORTED_PLACEHOLDER
        );
        assert_eq!(
            safe_body_from_parts(false, false, Some("clear text")),
            "clear text"
        );
    }

    #[test]
    fn sdk_utd_causes_map_to_honest_pending_and_unavailable_states() {
        for cause in [
            UtdCause::SentBeforeWeJoined,
            UtdCause::HistoricalMessageAndBackupIsDisabled,
            UtdCause::WithheldBySender,
        ] {
            assert!(is_currently_unavailable(cause));
        }
        for cause in [
            UtdCause::Unknown,
            UtdCause::VerificationViolation,
            UtdCause::UnsignedDevice,
            UtdCause::UnknownDevice,
            UtdCause::WithheldForUnverifiedOrInsecureDevice,
            UtdCause::HistoricalMessageAndDeviceIsUnverified,
        ] {
            assert!(!is_currently_unavailable(cause));
        }
    }

    #[test]
    fn live_registry_reconciles_pending_to_automatic_decrypted_readback() {
        let mut registry = NativeTimelineRegistry::new(11);
        let mut pending = NativeTimelineSnapshot {
            session_generation: 11,
            room_id: "!room:example.org".into(),
            is_encrypted: true,
            items: vec![NativeTimelineItem {
                item_id: "item-1".into(),
                event_id: "$event".into(),
                sender: "@alice:example.org".into(),
                event_type: "m.room.encrypted".into(),
                body: UTD_PLACEHOLDER.into(),
                origin_server_ts: 42,
                decryption_state: Some(NativeDecryptionState::Pending),
                reactions: vec![],
            }],
            hit_start: false,
            utd: NativeUtdStatus {
                phase: NativeUtdPhase::Idle,
                pending_count: 0,
                unavailable_count: 0,
                recovered_count: 0,
            },
        };
        registry.reconcile_utd(&mut pending, UtdRecoveryKind::RetryDecrypt);
        assert_eq!(pending.utd.phase, NativeUtdPhase::Recovering);
        assert_eq!(pending.utd.pending_count, 1);

        let mut decrypted = NativeTimelineSnapshot {
            items: vec![NativeTimelineItem {
                body: "clear text".into(),
                event_type: "m.room.message".into(),
                decryption_state: None,
                ..pending.items[0].clone()
            }],
            ..pending
        };
        registry.reconcile_utd(&mut decrypted, UtdRecoveryKind::RetryDecrypt);
        assert_eq!(decrypted.utd.phase, NativeUtdPhase::Idle);
        assert_eq!(decrypted.utd.pending_count, 0);
        assert_eq!(decrypted.utd.recovered_count, 1);
        assert_eq!(decrypted.items[0].body, "clear text");

        let first_op_id = registry
            .utd_recovery
            .get("!room:example.org")
            .unwrap()
            .op_id;
        let mut later_pending = NativeTimelineSnapshot {
            items: vec![NativeTimelineItem {
                item_id: "item-2".into(),
                event_id: "$event-2".into(),
                body: UTD_PLACEHOLDER.into(),
                event_type: "m.room.encrypted".into(),
                decryption_state: Some(NativeDecryptionState::Pending),
                ..decrypted.items[0].clone()
            }],
            ..decrypted
        };
        registry.reconcile_utd(&mut later_pending, UtdRecoveryKind::RetryDecrypt);
        let second_session = registry.utd_recovery.get("!room:example.org").unwrap();
        assert!(second_session.op_id > first_op_id);
        assert!(second_session.phase.is_active());

        later_pending.items[0].decryption_state = Some(NativeDecryptionState::Unavailable);
        registry.reconcile_utd(&mut later_pending, UtdRecoveryKind::RetryDecrypt);
        assert_eq!(later_pending.utd.phase, NativeUtdPhase::Unavailable);

        let mut later_decrypted = NativeTimelineSnapshot {
            items: vec![NativeTimelineItem {
                body: "later clear text".into(),
                event_type: "m.room.message".into(),
                decryption_state: None,
                ..later_pending.items[0].clone()
            }],
            ..later_pending
        };
        registry.reconcile_utd(&mut later_decrypted, UtdRecoveryKind::RetryDecrypt);
        assert_eq!(later_decrypted.utd.phase, NativeUtdPhase::Idle);
        assert_eq!(later_decrypted.utd.recovered_count, 1);
    }

    #[test]
    fn focused_event_readback_schema_excludes_crypto_material() {
        let readback = NativeTimelineEventReadback {
            session_generation: 3,
            room_id: "!room:example.org".into(),
            event_id: "$event".into(),
            item: NativeTimelineItem {
                item_id: "item".into(),
                event_id: "$event".into(),
                sender: "@alice:example.org".into(),
                event_type: "m.room.message".into(),
                body: "safe body".into(),
                origin_server_ts: 42,
                decryption_state: None,
                reactions: vec![],
            },
        };
        let json = serde_json::to_string(&readback).unwrap();
        for forbidden in [
            "sessionId",
            "sessionKey",
            "senderKey",
            "deviceId",
            "ciphertext",
        ] {
            assert!(!json.contains(forbidden));
        }
        assert!(json.contains("safe body"));
    }

    #[test]
    fn approval_reaction_readback_preserves_aggregate_and_adds_exact_local_echo() {
        let existing = vec![NativeTimelineReaction {
            key: "✅".into(),
            count: 2,
            me: false,
            senders: vec![NativeTimelineReactionSender {
                user_id: "@hermes:example.org".into(),
                reaction_event_id: Some("$seed".into()),
            }],
        }];

        let readback =
            approval_reaction_readback(&existing, "✅", "@alice:example.org", "$decision".into());

        assert_eq!(readback.key, "✅");
        assert_eq!(readback.count, 3);
        assert!(readback.me);
        assert_eq!(readback.senders.len(), 2);
        assert_eq!(readback.senders[1].user_id, "@alice:example.org");
        assert_eq!(
            readback.senders[1].reaction_event_id.as_deref(),
            Some("$decision")
        );
    }

    #[test]
    fn approval_reaction_readback_does_not_double_count_existing_self() {
        let existing = vec![NativeTimelineReaction {
            key: "❌".into(),
            count: 4,
            me: true,
            senders: vec![NativeTimelineReactionSender {
                user_id: "@alice:example.org".into(),
                reaction_event_id: None,
            }],
        }];

        let readback =
            approval_reaction_readback(&existing, "❌", "@alice:example.org", "$remote".into());

        assert_eq!(readback.count, 4);
        assert_eq!(readback.senders.len(), 1);
        assert_eq!(
            readback.senders[0].reaction_event_id.as_deref(),
            Some("$remote")
        );
    }

    #[test]
    fn approval_decision_memory_evicts_oldest_and_refreshes_existing_entry() {
        let mut decisions = VecDeque::new();
        for index in 0..MAX_FOCUSED_EVENT_READBACKS {
            remember_agent_approval_decision(
                &mut decisions,
                ("!room:example.org".into(), format!("$event-{index}")),
            );
        }
        remember_agent_approval_decision(
            &mut decisions,
            ("!room:example.org".into(), "$event-0".into()),
        );
        remember_agent_approval_decision(
            &mut decisions,
            ("!room:example.org".into(), "$newest".into()),
        );

        assert_eq!(decisions.len(), MAX_FOCUSED_EVENT_READBACKS);
        assert!(decisions.contains(&("!room:example.org".into(), "$event-0".into())));
        assert!(!decisions.contains(&("!room:example.org".into(), "$event-1".into())));
        assert_eq!(
            decisions.back().map(|entry| entry.1.as_str()),
            Some("$newest")
        );
    }

    #[test]
    fn approval_decision_registry_serializes_only_the_same_exact_event() {
        let mut registry = ApprovalDecisionRegistry::default();
        let first_key = ("!room:example.org".into(), "$first".into());
        let second_key = ("!room:example.org".into(), "$second".into());
        let other_room_key = ("!other:example.org".into(), "$first".into());

        let first = registry.lock_for(&first_key);
        let duplicate = registry.lock_for(&first_key);
        let unrelated = registry.lock_for(&second_key);
        let other_room = registry.lock_for(&other_room_key);

        assert!(Arc::ptr_eq(&first, &duplicate));
        assert!(!Arc::ptr_eq(&first, &unrelated));
        assert!(!Arc::ptr_eq(&first, &other_room));
        registry.remember(first_key.clone());
        assert!(registry.is_completed(&first_key));
        assert!(!registry.is_completed(&second_key));
        assert!(!registry.is_completed(&other_room_key));
    }

    #[test]
    fn approval_decision_registry_discards_expired_per_event_locks() {
        let mut registry = ApprovalDecisionRegistry::default();
        let old_key = ("!room:example.org".into(), "$old".into());
        let old = registry.lock_for(&old_key);
        drop(old);

        let new_key = ("!room:example.org".into(), "$new".into());
        let _new = registry.lock_for(&new_key);

        assert!(!registry.in_flight.contains_key(&old_key));
        assert!(registry.in_flight.contains_key(&new_key));
    }

    #[test]
    fn composer_send_owner_uses_room_send_queue_without_extra_content() {
        let source = include_str!("live.rs");
        for marker in [
            "enqueue_event_via_room_queue",
            "send_event_via_room_queue",
            "wait_for_queued_send",
            "unwedge_queued_send",
            "abort_queued_send",
        ] {
            assert!(
                source.contains(marker),
                "native send owner must use {marker}"
            );
        }
        let send_text_start = source.find("pub async fn send_text(").unwrap();
        let send_poll_start = source.find("pub async fn send_poll(").unwrap();
        let send_text = &source[send_text_start..send_poll_start];
        assert!(send_text.contains("enqueue_event_via_room_queue"));
        assert!(send_text.contains("repeated_text_send"));
        assert!(!send_text.contains(&format!("{}{}", "room.", "send(")));
        assert!(!send_text.contains("send_message_to_room"));
        assert!(!source.contains(&format!("{}{}", "send().", "with_extra")));

        let ensure_start = source
            .find("Idempotently ensure an approval annotation exists.")
            .unwrap();
        let redact_start = source
            .find("Redact any reaction annotation selected in the viewer.")
            .unwrap();
        let ensure = &source[ensure_start..redact_start];
        assert!(ensure.contains("send_event_via_room_queue"));
        assert!(!ensure.contains(&format!("{}{}", "room.", "send(")));
    }

    #[test]
    fn session_start_subscribes_event_cache_and_pin_panel_uses_cache() {
        let source = include_str!("live.rs");
        assert!(source.contains("client.event_cache().subscribe()"));
        assert!(source.contains("pinned_events_snapshot"));
        assert!(source.contains("snapshot_pinned_events"));
        assert!(source.contains("room.pin_event"));
        assert!(source.contains("room.unpin_event"));
        assert!(!source.contains(&format!("{}{}", "TimelineFocus::", "PinnedEvents")));
    }
}
