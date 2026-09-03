//! D0.3 live Matrix SDK timeline ownership and privacy-safe projection.
//!
//! SDK timeline objects stay inside the Rust session. The webview receives a
//! product snapshot containing only stable identifiers, sender IDs,
//! event types, timestamps, and safe display text.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Weak};

use eyeball_im::VectorDiff;
use futures_util::{stream, StreamExt};
use matrix_sdk::{
    event_cache::PaginationStatus,
    room::{calls::CallError, edit::EditedContent, Receipts},
    ruma::{
        events::{
            reaction::ReactionEventContent,
            receipt::{ReceiptThread, ReceiptType as EventReceiptType},
            relation::Annotation,
            room::message::{
                MessageFormat, MessageType, Relation, RoomMessageEventContent,
                RoomMessageEventContentWithoutRelation,
            },
            sticker::StickerEventContent,
            AnyMessageLikeEventContent, AnySyncMessageLikeEvent, AnySyncStateEvent,
            AnySyncTimelineEvent, Mentions, StateEventType,
        },
        OwnedEventId, OwnedRoomId, OwnedUserId, UserId,
    },
    Client, EncryptionState, Room,
};
use matrix_sdk_crypto::types::events::UtdCause;
use matrix_sdk_ui::timeline::{
    EncryptedMessage, MsgLikeKind, ReactionStatus, Timeline, TimelineBuilder, TimelineDetails,
    TimelineEventFocusThreadMode, TimelineEventItemId, TimelineFocus,
    TimelineItem as SdkTimelineItem, TimelineItemContent as SdkTimelineItemContent,
    TimelineReadReceiptTracking,
};
use serde::{Deserialize, Serialize};
use tokio::time::{timeout, Duration};
use tokio::{sync::Mutex as AsyncMutex, task::JoinHandle};

use crate::app::agent_approvals::{plan_agent_approval, AgentApprovalDecisionStatus};
use crate::app::send::{
    apply_poll_start_relations, edit_message_content, message_content, normalize_poll,
    parse_edit_event_id, parse_reply_event_id, parse_send_room_id, parse_thread_root_event_id,
    parse_transaction_id, poll_response_content, poll_start_content, send_message_to_room,
    MatrixPollRespondResult, MatrixSendPollResult, MatrixSendTextResult, SendQueue,
};
use crate::app::utd_recovery::{UtdRecoveryCoordinator, UtdRecoveryKind, MAX_EVENT_IDS_PER_BATCH};
use crate::dto::{RoomEncryptionStatus, TimelineEncryptedUnavailableItem};

use super::{
    format_forwarded_media_body, format_forwarded_plain_body, project_timeline_diffs_with_media,
    project_timeline_item_with_media, reply_draft_readback, should_attach_formatted_body,
    ComposerDraftRegistry, NativeAgentApprovalDecisionRequest, NativeAgentApprovalDecisionResult,
    NativeComposerReplyDraft, NativeComposerReplyDraftReadback, NativeDecryptionState,
    NativeReactionMutation, NativeReactionMutationResult, NativeTimelineActionKind,
    NativeTimelineActionReadback, NativeTimelineCloseRequest, NativeTimelineDirection,
    NativeTimelineEventReadback, NativeTimelineItem, NativeTimelineJumpLatestRequest,
    NativeTimelineOpenPosition, NativeTimelineOpenReadback, NativeTimelineOpenRequest,
    NativeTimelineReaction, NativeTimelineReactionSender, NativeTimelineReadAction,
    NativeTimelineReadIntent, NativeTimelineReadStateReadback, NativeTimelineReadStateRequest,
    NativeTimelineSnapshot, NativeTimelineViewPaginationRequest, NativeTimelineViewportHint,
    NativeUtdPhase, NativeUtdStatus, TimelineMediaRegistry, TimelineMediaSource, TimelinePageState,
    TimelinePaginationState, TimelineReadState, TimelineRoomActionAuthority,
    TimelineViewCapabilities, TimelineViewDeltaBatch, TimelineViewPosition, TimelineViewSnapshot,
    TimelineViewUpdateEmit, UtdIndex, UtdPhase, UtdReasonCode, ViewDeltaEmitter,
    NATIVE_TIMELINE_ACTION_SCHEMA_VERSION, NATIVE_TIMELINE_OPEN_SCHEMA_VERSION,
    NATIVE_TIMELINE_VIEWPORT_RESTORE_TTL_MS, TIMELINE_VIEW_SCHEMA_VERSION,
};

const PAGINATION_BATCH_SIZE: u16 = 30;
const REDACTED_PLACEHOLDER: &str = "Message removed";
const UTD_PLACEHOLDER: &str = "Unable to decrypt this message";
const UNSUPPORTED_PLACEHOLDER: &str = "Unsupported event";
const MAX_FOCUSED_EVENT_READBACKS: usize = 256;
const FOCUSED_CONTEXT_EVENT_COUNT: u16 = 25;
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

fn exact_read_receipts(event_id: OwnedEventId) -> Receipts {
    Receipts::new()
        .fully_read_marker(Some(event_id.clone()))
        .private_read_receipt(Some(event_id))
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
) -> Result<Option<OwnedEventId>, &'static str> {
    // Use the same SDK-owned latest-event resolver as Timeline::mark_as_read;
    // hand-walking visible items diverges for local echoes, focus, and threads.
    match plan_live_read_target(
        timeline.latest_event_id().await,
        intent,
        observed_live_tail_event_id,
    )? {
        LiveReadTargetPlan::NoOp => Ok(None),
        LiveReadTargetPlan::ClearUnreadFlag => {
            // Explicit Mark Read must still clear a manually marked-unread room
            // when the room has no receipt-capable remote event.
            timeline
                .room()
                .set_unread_flag(false)
                .await
                .map_err(|_| "v-timeline-clear-empty-unread-failed")?;
            Ok(None)
        }
        LiveReadTargetPlan::Send(event_id) => {
            timeline
                // Pinned matrix-sdk-ui 0.18 invariant: `Timeline::send_multiple_receipts`
                // clears the SDK room's unread flag after a submitted marker update and
                // also when receipt deduplication removes every unchanged marker. Keep
                // this evidence in the adjacent regression test when upgrading the SDK.
                .send_multiple_receipts(exact_read_receipts(event_id.clone()))
                .await
                .map_err(|_| "v-timeline-send-read-markers-failed")?;
            Ok(Some(event_id))
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
    hit_start: bool,
}

struct ViewStreamEntry {
    room_id: String,
    timeline: Arc<Timeline>,
    position: TimelineViewPosition,
    hit_start: Arc<AtomicBool>,
    media: Arc<AsyncMutex<TimelineMediaRegistry>>,
}

pub struct NativeTimelineRegistry {
    session_generation: u64,
    entries: HashMap<String, LiveTimelineEntry>,
    focused_entries: HashMap<(String, String), Arc<Timeline>>,
    view_streams: HashMap<String, ViewStreamEntry>,
    view_update_tasks: HashMap<String, JoinHandle<()>>,
    view_revisions: HashMap<String, Arc<AtomicU64>>,
    next_view_stream_id: u64,
    utd_index: UtdIndex,
    utd_recovery: UtdRecoveryCoordinator,
}

/// Shared handle so Core and the desktop session own one live registry.
pub struct NativeTimelineOwner {
    client: Client,
    emit: TimelineViewUpdateEmit,
    registry: tokio::sync::Mutex<NativeTimelineRegistry>,
    /// Serializes duplicate decisions per exact event without monopolizing the
    /// global timeline registry or blocking unrelated approval prompts.
    approval_decisions: Arc<std::sync::Mutex<ApprovalDecisionRegistry>>,
    drafts: tokio::sync::Mutex<ComposerDraftRegistry>,
    sends: tokio::sync::Mutex<SendQueue>,
}

impl NativeTimelineOwner {
    pub fn new(client: &Client, emit: TimelineViewUpdateEmit, session_generation: u64) -> Self {
        Self {
            client: client.clone(),
            emit,
            registry: tokio::sync::Mutex::new(NativeTimelineRegistry::new(session_generation)),
            approval_decisions: Arc::new(
                std::sync::Mutex::new(ApprovalDecisionRegistry::default()),
            ),
            drafts: tokio::sync::Mutex::new(ComposerDraftRegistry::new()),
            sends: tokio::sync::Mutex::new(SendQueue::new(session_generation)),
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
            .registry
            .lock()
            .await
            .resolve_media(handle_id)
            .await
            .ok_or("p4-s33-media-unknown-handle")?;
        let request = matrix_sdk::media::MediaRequestParameters {
            source: source.source,
            format: matrix_sdk::media::MediaFormat::File,
        };
        super::super::media::download_media_bounded(&self.client, &request, 32 * 1024 * 1024)
            .await
            .map_err(|error| match error {
                super::super::media::BoundedMediaError::TooLarge => "p4-s33-media-too-large",
                _ => "p4-s33-media-failed",
            })
    }

    pub async fn event_readback(
        &self,
        room_id: &str,
        event_id: &str,
    ) -> Result<NativeTimelineEventReadback, &'static str> {
        self.registry
            .lock()
            .await
            .event_readback(&self.client, room_id, event_id)
            .await
    }

    pub async fn paginate(
        &self,
        request: NativeTimelineViewPaginationRequest,
    ) -> Result<TimelineViewSnapshot, &'static str> {
        self.registry
            .lock()
            .await
            .paginate(&self.client, request)
            .await
    }

    pub async fn set_read_state(
        &self,
        request: NativeTimelineReadStateRequest,
    ) -> Result<NativeTimelineReadStateReadback, &'static str> {
        self.registry
            .lock()
            .await
            .set_read_state(&self.client, request)
            .await
    }

    /// Mark a room read or unread without requiring an already-open view stream.
    /// Context-menu Mark as Read uses this while the room is not mounted.
    pub async fn set_room_read_state(
        &self,
        room_id: &str,
        action: NativeTimelineReadAction,
    ) -> Result<(), &'static str> {
        self.registry
            .lock()
            .await
            .set_room_read_state(&self.client, room_id, action)
            .await
    }

    pub async fn toggle_reaction(
        &self,
        room_id: &str,
        event_id: &str,
        key: &str,
    ) -> Result<NativeReactionMutationResult, &'static str> {
        self.registry
            .lock()
            .await
            .toggle_reaction(&self.client, room_id, event_id, key)
            .await
    }

    pub async fn ensure_reaction(
        &self,
        room_id: &str,
        event_id: &str,
        key: &str,
    ) -> Result<NativeReactionMutationResult, &'static str> {
        self.registry
            .lock()
            .await
            .ensure_reaction(&self.client, room_id, event_id, key)
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
                return Ok(NativeAgentApprovalDecisionResult {
                    room_id,
                    event_id: event_id.to_string(),
                    status: AgentApprovalDecisionStatus::AlreadyDecided,
                    reaction: None,
                });
            }
            decisions.lock_for(&decision_key)
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
                    thread_mode: TimelineEventFocusThreadMode::Automatic {
                        hide_threaded_events: false,
                    },
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
        self.registry
            .lock()
            .await
            .redact_reaction(
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
        self.registry
            .lock()
            .await
            .open_at(self.emit.clone(), &self.client, request)
            .await
    }

    pub async fn jump_latest(
        &self,
        request: NativeTimelineJumpLatestRequest,
    ) -> Result<NativeTimelineOpenReadback, &'static str> {
        self.registry
            .lock()
            .await
            .jump_latest(self.emit.clone(), &self.client, request)
            .await
    }

    pub async fn snapshot(&self, stream_id: &str) -> Result<TimelineViewSnapshot, &'static str> {
        self.registry
            .lock()
            .await
            .view_snapshot_for_stream(&self.client, stream_id)
            .await
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
        let txn_id = parse_transaction_id(txn_id)?;
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
        let local_txn_id = {
            let mut sends = self.sends.lock().await;
            sends
                .enqueue_text(parsed_room.to_string(), body)
                .map_err(|error| error.diagnostic_id())?
                .local_txn_id
                .clone()
        };
        let send_result = send_message_to_room(&room, content, txn_id).await;
        {
            let mut sends = self.sends.lock().await;
            if send_result.is_ok() {
                let _ = sends.mark_sent(&local_txn_id);
            } else {
                let _ = sends.mark_failed(&local_txn_id, "d0.4-send-sdk-failed");
            }
        }
        let event_id = send_result?;
        Ok(MatrixSendTextResult {
            room_id: parsed_room.to_string(),
            event_id,
            local_txn_id,
            status: "sent",
        })
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
        let response = room
            .send(content)
            .await
            .map_err(|_| "v-send.3-poll-sdk-failed")?;
        Ok(MatrixSendPollResult {
            room_id: parsed_room.to_string(),
            event_id: response.response.event_id.to_string(),
            status: "sent",
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
        let response = room
            .send(content)
            .await
            .map_err(|_| "v-send.3-poll-response-sdk-failed")?;
        Ok(MatrixPollRespondResult {
            room_id: parsed_room.to_string(),
            poll_event_id,
            event_id: response.response.event_id.to_string(),
            status: "sent",
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
        let txn_id = parse_transaction_id(txn_id)?;
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
        let local_txn_id = {
            let mut sends = self.sends.lock().await;
            sends
                .enqueue_text(parsed_room.to_string(), body)
                .map_err(|error| error.diagnostic_id())?
                .local_txn_id
                .clone()
        };
        let send_result = send_message_to_room(&room, content, txn_id)
            .await
            .map_err(|_| "v-send.r-edit-sdk-failed");
        {
            let mut sends = self.sends.lock().await;
            if send_result.is_ok() {
                let _ = sends.mark_sent(&local_txn_id);
            } else {
                let _ = sends.mark_failed(&local_txn_id, "v-send.r-edit-sdk-failed");
            }
        }
        let event_id = send_result?;
        Ok(MatrixSendTextResult {
            room_id: parsed_room.to_string(),
            event_id,
            local_txn_id,
            status: "sent",
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
        let response = room
            .send(edit_content)
            .await
            .map_err(|_| "v-timeline-edit-send-failed")?;
        Ok(NativeTimelineActionReadback {
            schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
            action: NativeTimelineActionKind::EditText,
            room_id: room_id.to_string(),
            event_id: response.response.event_id.to_string(),
            status: "sent".into(),
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
            if let Some(entry) = registry.entries.get(room_id.as_str()) {
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
        room.redact(&event_id, reason.as_deref(), None)
            .await
            .map_err(|_| "v-timeline-redact-failed")?;
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
            if let Some(entry) = registry.entries.get(room_id.as_str()) {
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
        let sent_event_id = room
            .send(content)
            .await
            .map_err(|_| "v-timeline-poll-vote-send-failed")?
            .response
            .event_id
            .to_string();
        Ok(NativeTimelineActionReadback {
            schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
            action: NativeTimelineActionKind::PollVote,
            room_id: room_id.to_string(),
            event_id: sent_event_id,
            status: "voted".into(),
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
        let sent_event_id = room
            .send(content)
            .await
            .map_err(|_| "v-timeline-call-decline-send-failed")?
            .response
            .event_id
            .to_string();
        Ok(NativeTimelineActionReadback {
            schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
            action: NativeTimelineActionKind::CallDecline,
            room_id: room_id.to_string(),
            event_id: sent_event_id,
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
        let sent_event_id = target_room
            .send(content)
            .await
            .map_err(|_| "v-timeline-forward-send-failed")?
            .response
            .event_id
            .to_string();
        Ok(NativeTimelineActionReadback {
            schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
            action: NativeTimelineActionKind::ForwardText,
            room_id: target_room_id.to_string(),
            event_id: sent_event_id,
            status: "sent".into(),
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
        let sent_event_id = target_room
            .send(content)
            .await
            .map_err(|_| "v-timeline-forward-media-send-failed")?
            .response
            .event_id
            .to_string();
        Ok(NativeTimelineActionReadback {
            schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
            action: NativeTimelineActionKind::ForwardMedia,
            room_id: target_room_id.to_string(),
            event_id: sent_event_id,
            status: "sent".into(),
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
        Ok(reply_draft_readback(room_id_string, "set", Some(draft)))
    }

    pub async fn clear_reply_draft(
        &self,
        room_id: &str,
        expected_draft_revision: u64,
    ) -> Result<NativeComposerReplyDraftReadback, &'static str> {
        let room_id = parse_action_room_id(room_id)?;
        let room_id_string = room_id.to_string();
        let superseding_draft = self
            .drafts
            .lock()
            .await
            .compare_and_clear(&room_id_string, expected_draft_revision);
        Ok(match superseding_draft {
            Some(draft) => reply_draft_readback(room_id_string, "set", Some(draft)),
            None => reply_draft_readback(room_id_string, "cleared", None),
        })
    }

    pub async fn get_reply_draft(
        &self,
        room_id: &str,
    ) -> Result<NativeComposerReplyDraftReadback, &'static str> {
        let room_id = parse_action_room_id(room_id)?;
        let room_id_string = room_id.to_string();
        let draft = self.drafts.lock().await.get(&room_id_string).cloned();
        Ok(reply_draft_readback(
            room_id_string,
            if draft.is_some() { "set" } else { "empty" },
            draft,
        ))
    }
}

impl NativeTimelineRegistry {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            entries: HashMap::new(),
            focused_entries: HashMap::new(),
            view_streams: HashMap::new(),
            view_update_tasks: HashMap::new(),
            view_revisions: HashMap::new(),
            next_view_stream_id: 0,
            utd_index: UtdIndex::new(session_generation),
            utd_recovery: UtdRecoveryCoordinator::new(session_generation),
        }
    }

    pub async fn open(
        &mut self,
        client: &Client,
        room_id: &str,
    ) -> Result<NativeTimelineSnapshot, &'static str> {
        let room_id = parse_room_id(room_id)?;
        let room_id_string = room_id.to_string();
        if !self.entries.contains_key(&room_id_string) {
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
                .track_read_marker_and_receipts(TimelineReadReceiptTracking::AllEvents)
                .build()
                .await
                .map_err(|_| "d0.3-timeline-open-failed")?;
            self.entries.insert(
                room_id_string.clone(),
                LiveTimelineEntry {
                    timeline: Arc::new(timeline),
                    is_encrypted,
                    hit_start: false,
                },
            );
        }
        self.snapshot(client, &room_id_string).await
    }

    /// Open the requested view from the native timeline owner without
    /// collapsing an event-link focus into the live-bottom route.
    pub async fn open_at(
        &mut self,
        emit: TimelineViewUpdateEmit,
        client: &Client,
        request: NativeTimelineOpenRequest,
    ) -> Result<NativeTimelineOpenReadback, &'static str> {
        let room_id = parse_room_id(&request.room_id)?;
        let room_id_string = room_id.to_string();
        let requested_position = request.position;
        let (timeline, view_position, pagination) = match &requested_position {
            NativeTimelineOpenPosition::Normal { viewport } => {
                let room = client
                    .get_room(&room_id)
                    .ok_or("v-timeline-normal-room-not-found")?;
                let has_unread = room.is_marked_unread()
                    || room.num_unread_messages() > 0
                    || room.num_unread_notifications() > 0
                    || room.num_unread_mentions() > 0;
                // Live-tail matching for unread+at_bottom restore needs the
                // live timeline's current tip before placement is chosen.
                let current_live_tail = if viewport.at_bottom && has_unread {
                    self.open(client, &room_id_string).await?;
                    self.live_tail_event_id(&room_id_string).await
                } else {
                    None
                };
                // Last-read is the newest of own receipts + `m.fully_read` that
                // can be ordered without walking history. Unread counts come
                // from receipts; a stale fully-read marker cannot override them.
                let unread_plan = if has_unread {
                    self.open(client, &room_id_string).await?;
                    self.unread_open_plan(client, &room, &room_id_string).await
                } else {
                    UnreadOpenPlan::LiveBottom
                };
                let unread_frontier = match &unread_plan {
                    UnreadOpenPlan::InLive { event_id }
                    | UnreadOpenPlan::FocusedReceipt { event_id } => Some(event_id.clone()),
                    UnreadOpenPlan::LiveBottom => None,
                };
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
                        self.open(client, &room_id_string).await?;
                        let entry = self
                            .entries
                            .get(&room_id_string)
                            .expect("live timeline inserted by open");
                        (
                            entry.timeline.clone(),
                            TimelineViewPosition::LiveBottom,
                            TimelinePaginationState {
                                backward: if entry.hit_start {
                                    TimelinePageState::Exhausted
                                } else {
                                    TimelinePageState::Available
                                },
                                forward: TimelinePageState::Available,
                            },
                        )
                    }
                    TimelineViewPosition::Unread {
                        ref anchor_event_id,
                    } => {
                        // Element opens the live window and places last-read
                        // inside it. A focused historical provider is only
                        // for a receipt that is not yet in that live window.
                        self.open(client, &room_id_string).await?;
                        let live_ids = self.live_event_ids(&room_id_string).await;
                        if live_ids.iter().any(|id| id == anchor_event_id) {
                            let entry = self
                                .entries
                                .get(&room_id_string)
                                .expect("live timeline inserted by open");
                            (
                                entry.timeline.clone(),
                                selected_position,
                                live_pagination_state(entry),
                            )
                        } else {
                            self.open_focused_unread(
                                &room,
                                &room_id_string,
                                anchor_event_id,
                                "v-timeline-normal-anchor-invalid",
                                "v-timeline-normal-open-failed",
                            )
                            .await?
                        }
                    }
                    TimelineViewPosition::Restored {
                        anchor_event_id: Some(ref anchor_event_id),
                    } => {
                        self.open_focused_unread(
                            &room,
                            &room_id_string,
                            anchor_event_id,
                            "v-timeline-normal-anchor-invalid",
                            "v-timeline-normal-open-failed",
                        )
                        .await?
                    }
                    TimelineViewPosition::Focused { .. }
                    | TimelineViewPosition::Restored {
                        anchor_event_id: None,
                    } => unreachable!("normal open only selects live, unread, or anchored restore"),
                }
            }
            NativeTimelineOpenPosition::LiveBottom => {
                self.open(client, &room_id_string).await?;
                let entry = self
                    .entries
                    .get(&room_id_string)
                    .expect("live timeline inserted by open");
                (
                    entry.timeline.clone(),
                    TimelineViewPosition::LiveBottom,
                    TimelinePaginationState {
                        backward: if entry.hit_start {
                            TimelinePageState::Exhausted
                        } else {
                            TimelinePageState::Available
                        },
                        forward: TimelinePageState::Available,
                    },
                )
            }
            NativeTimelineOpenPosition::Focused { event_id } => {
                let event_id = parse_event_id(event_id)?;
                let key = (room_id_string.clone(), event_id.to_string());
                let room = client
                    .get_room(&room_id)
                    .ok_or("v-timeline-focused-room-not-found")?;
                if !self.focused_entries.contains_key(&key) {
                    if self.focused_entries.len() >= MAX_FOCUSED_EVENT_READBACKS {
                        if let Some(oldest_key) = self.focused_entries.keys().next().cloned() {
                            self.focused_entries.remove(&oldest_key);
                        }
                    }
                    let timeline = TimelineBuilder::new(&room)
                        .with_focus(TimelineFocus::Event {
                            target: event_id.clone(),
                            num_context_events: FOCUSED_CONTEXT_EVENT_COUNT,
                            thread_mode: TimelineEventFocusThreadMode::Automatic {
                                hide_threaded_events: false,
                            },
                        })
                        .build()
                        .await
                        .map_err(|_| "v-timeline-focused-open-failed")?;
                    self.focused_entries.insert(key.clone(), Arc::new(timeline));
                }
                let timeline = self
                    .focused_entries
                    .get(&key)
                    .expect("focused timeline present")
                    .clone();
                (
                    timeline,
                    TimelineViewPosition::Focused {
                        target_event_id: event_id.to_string(),
                    },
                    TimelinePaginationState {
                        backward: TimelinePageState::Available,
                        forward: TimelinePageState::Available,
                    },
                )
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
                self.open(client, &room_id_string).await?;
                match self.unread_open_plan(client, &room, &room_id_string).await {
                    UnreadOpenPlan::InLive { event_id } => {
                        let entry = self
                            .entries
                            .get(&room_id_string)
                            .expect("live timeline inserted by open");
                        (
                            entry.timeline.clone(),
                            TimelineViewPosition::Unread {
                                anchor_event_id: event_id,
                            },
                            live_pagination_state(entry),
                        )
                    }
                    UnreadOpenPlan::FocusedReceipt { event_id } => {
                        self.open_focused_unread(
                            &room,
                            &room_id_string,
                            &event_id,
                            "v-timeline-unread-frontier-unavailable",
                            "v-timeline-unread-open-failed",
                        )
                        .await?
                    }
                    UnreadOpenPlan::LiveBottom => {
                        let entry = self
                            .entries
                            .get(&room_id_string)
                            .expect("live timeline inserted by open");
                        (
                            entry.timeline.clone(),
                            TimelineViewPosition::LiveBottom,
                            live_pagination_state(entry),
                        )
                    }
                }
            }
        };
        let own_user_id = client.user_id().map(ToOwned::to_owned);
        let action_authority = room_action_authority(timeline.room(), own_user_id.as_deref()).await;
        self.next_view_stream_id = self.next_view_stream_id.saturating_add(1);
        let subscription_key = format!(
            "{}:{}",
            view_subscription_key(&room_id_string, &view_position),
            self.next_view_stream_id
        );
        let revision = Arc::new(AtomicU64::new(0));
        self.view_revisions
            .insert(subscription_key.clone(), revision.clone());
        let hit_start = Arc::new(AtomicBool::new(matches!(
            pagination.backward,
            TimelinePageState::Exhausted
        )));
        self.view_streams.insert(
            subscription_key.clone(),
            ViewStreamEntry {
                room_id: room_id_string.clone(),
                timeline: timeline.clone(),
                position: view_position.clone(),
                hit_start: hit_start.clone(),
                media: Arc::new(AsyncMutex::new(TimelineMediaRegistry::new(
                    self.session_generation,
                    subscription_key.clone(),
                ))),
            },
        );
        let media = self
            .view_streams
            .get(&subscription_key)
            .expect("view stream inserted before projection")
            .media
            .clone();
        // Subscribe before materializing the initial rows so no SDK update can
        // fall into a snapshot-to-stream gap. Every open owns a distinct
        // stream, revision counter, update task, and media registry.
        let (items, updates) = timeline.subscribe().await;
        let item_ids: Vec<String> = items
            .iter()
            .map(|item| item.unique_id().0.clone())
            .collect();
        let hydrate_sender_profiles = items
            .iter()
            .filter_map(|item| item.as_event())
            .any(|event| !matches!(event.sender_profile(), TimelineDetails::Ready(_)));
        let rows = {
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
        let snapshot = view_snapshot_from_items(
            TimelineViewSnapshotInput {
                session_generation: self.session_generation,
                room_id: room_id_string.clone(),
                position: view_position.clone(),
                pagination,
                own_user_id: own_user_id.clone(),
                revision: revision.load(Ordering::Acquire),
            },
            &timeline,
            rows,
        )
        .await;
        self.view_update_tasks.insert(
            subscription_key.clone(),
            spawn_view_update_owner(
                ViewUpdateOwnerInput {
                    emit,
                    session_generation: self.session_generation,
                    stream_id: subscription_key.clone(),
                    room_id: room_id_string.clone(),
                    own_user_id,
                    revision,
                    media,
                    item_ids,
                    timeline: timeline.clone(),
                    position: view_position.clone(),
                    hit_start,
                    hydrate_sender_profiles,
                },
                updates,
            ),
        );
        Ok(NativeTimelineOpenReadback {
            schema_version: NATIVE_TIMELINE_OPEN_SCHEMA_VERSION,
            stream_id: subscription_key,
            position: view_position,
            snapshot,
        })
    }

    pub async fn snapshot(
        &mut self,
        client: &Client,
        room_id: &str,
    ) -> Result<NativeTimelineSnapshot, &'static str> {
        let room_id = parse_room_id(room_id)?.to_string();
        let entry = self.entries.get(&room_id).ok_or("d0.3-timeline-not-open")?;
        let mut snapshot = snapshot_from_timeline(
            self.session_generation,
            room_id.clone(),
            &entry.timeline,
            entry.is_encrypted,
            entry.hit_start,
            client.user_id(),
        )
        .await?;
        self.reconcile_utd(&mut snapshot, UtdRecoveryKind::RetryDecrypt)?;
        Ok(snapshot)
    }

    pub async fn paginate(
        &mut self,
        client: &Client,
        request: NativeTimelineViewPaginationRequest,
    ) -> Result<TimelineViewSnapshot, &'static str> {
        let (room_id, timeline, position, pagination, media) = {
            let stream = self
                .view_streams
                .get_mut(&request.stream_id)
                .ok_or("v-timeline-view-not-open")?;
            let reached_end = match request.direction {
                NativeTimelineDirection::Backwards => stream
                    .timeline
                    .paginate_backwards(PAGINATION_BATCH_SIZE)
                    .await
                    .map_err(|_| "v-timeline-view-paginate-backwards-failed")?,
                NativeTimelineDirection::Forwards => stream
                    .timeline
                    .paginate_forwards(PAGINATION_BATCH_SIZE)
                    .await
                    .map_err(|_| "v-timeline-view-paginate-forwards-failed")?,
            };
            if request.direction == NativeTimelineDirection::Backwards {
                stream.hit_start.store(reached_end, Ordering::Release);
            }
            (
                stream.room_id.clone(),
                stream.timeline.clone(),
                stream.position.clone(),
                TimelinePaginationState {
                    backward: if stream.hit_start.load(Ordering::Acquire) {
                        TimelinePageState::Exhausted
                    } else {
                        TimelinePageState::Available
                    },
                    forward: TimelinePageState::Available,
                },
                stream.media.clone(),
            )
        };
        let revision = self
            .view_revisions
            .get(&request.stream_id)
            .ok_or("v-timeline-view-revision-missing")?
            .load(Ordering::Acquire);
        Ok(view_snapshot_from_timeline(
            TimelineViewSnapshotInput {
                session_generation: self.session_generation,
                room_id,
                position,
                pagination,
                own_user_id: client.user_id().map(ToOwned::to_owned),
                revision,
            },
            &timeline,
            media,
        )
        .await)
    }

    pub async fn set_read_state(
        &mut self,
        client: &Client,
        request: NativeTimelineReadStateRequest,
    ) -> Result<NativeTimelineReadStateReadback, &'static str> {
        let stream = self
            .view_streams
            .get(&request.stream_id)
            .ok_or("v-timeline-view-not-open")?;
        if request.action == NativeTimelineReadAction::MarkRead
            && stream.position != TimelineViewPosition::LiveBottom
        {
            return Err("v-timeline-read-requires-live-view");
        }
        let timeline = stream.timeline.clone();
        let (receipt_sent, acknowledged_event_id) = match request.action {
            NativeTimelineReadAction::MarkRead => {
                let acknowledged_event_id = mark_live_timeline_read(
                    &timeline,
                    request.intent,
                    request.observed_live_tail_event_id.as_deref(),
                )
                .await?;
                (Some(acknowledged_event_id.is_some()), acknowledged_event_id)
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
                (None, None)
            }
        };
        let snapshot = self
            .view_snapshot_for_stream(client, &request.stream_id)
            .await?;
        Ok(NativeTimelineReadStateReadback {
            action: request.action,
            receipt_sent,
            acknowledged_event_id: acknowledged_event_id.map(|event_id| event_id.to_string()),
            snapshot,
        })
    }

    /// Send receipts and/or the unread flag for a room that may not have a view.
    pub async fn set_room_read_state(
        &mut self,
        client: &Client,
        room_id: &str,
        action: NativeTimelineReadAction,
    ) -> Result<(), &'static str> {
        let room_id = parse_room_id(room_id)?;
        let room_id_string = room_id.to_string();
        let room = client
            .get_room(&room_id)
            .ok_or("v-rooms-room-read-state-room-not-found")?;
        match action {
            NativeTimelineReadAction::MarkRead => {
                self.open(client, &room_id_string).await?;
                let timeline = self
                    .entries
                    .get(&room_id_string)
                    .ok_or("d0.3-timeline-open-failed")?
                    .timeline
                    .clone();
                mark_live_timeline_read(&timeline, NativeTimelineReadIntent::ExplicitUser, None)
                    .await
                    .map_err(|_| "v-rooms-room-read-state-mark-read-failed")?;
            }
            NativeTimelineReadAction::MarkUnread => {
                room.set_unread_flag(true)
                    .await
                    .map_err(|_| "v-rooms-room-read-state-mark-unread-failed")?;
            }
        }
        Ok(())
    }

    pub async fn view_snapshot_for_stream(
        &self,
        client: &Client,
        stream_id: &str,
    ) -> Result<TimelineViewSnapshot, &'static str> {
        let stream = self
            .view_streams
            .get(stream_id)
            .ok_or("v-timeline-view-not-open")?;
        let revision = self
            .view_revisions
            .get(stream_id)
            .ok_or("v-timeline-view-revision-missing")?
            .load(Ordering::Acquire);
        Ok(view_snapshot_from_timeline(
            TimelineViewSnapshotInput {
                session_generation: self.session_generation,
                room_id: stream.room_id.clone(),
                position: stream.position.clone(),
                pagination: TimelinePaginationState {
                    backward: if stream.hit_start.load(Ordering::Acquire) {
                        TimelinePageState::Exhausted
                    } else {
                        TimelinePageState::Available
                    },
                    forward: TimelinePageState::Available,
                },
                own_user_id: client.user_id().map(ToOwned::to_owned),
                revision,
            },
            &stream.timeline,
            stream.media.clone(),
        )
        .await)
    }

    pub fn close_view(&mut self, request: NativeTimelineCloseRequest) -> bool {
        let removed = self.view_streams.remove(&request.stream_id).is_some();
        self.view_revisions.remove(&request.stream_id);
        if let Some(task) = self.view_update_tasks.remove(&request.stream_id) {
            task.abort();
        }
        removed
    }

    /// Rebind an opened stream to the live bottom and return a fresh stream
    /// ownership packet. The previous stream is closed; the caller must adopt
    /// the new stream id from readback.
    pub async fn jump_latest(
        &mut self,
        emit: TimelineViewUpdateEmit,
        client: &Client,
        request: NativeTimelineJumpLatestRequest,
    ) -> Result<NativeTimelineOpenReadback, &'static str> {
        let room_id = self
            .view_streams
            .get(&request.stream_id)
            .ok_or("v-timeline-view-not-open")?
            .room_id
            .clone();
        self.close_view(NativeTimelineCloseRequest {
            stream_id: request.stream_id,
        });
        self.open_at(
            emit,
            client,
            NativeTimelineOpenRequest {
                room_id,
                position: NativeTimelineOpenPosition::LiveBottom,
            },
        )
        .await
    }

    async fn live_tail_event_id(&self, room_id: &str) -> Option<String> {
        let entry = self.entries.get(room_id)?;
        let items = entry.timeline.items().await;
        items.iter().rev().find_map(|item| {
            item.as_event()
                .and_then(|event| event.event_id().map(|event_id| event_id.to_string()))
        })
    }

    async fn live_event_ids(&self, room_id: &str) -> Vec<String> {
        let Some(entry) = self.entries.get(room_id) else {
            return Vec::new();
        };
        let items = entry.timeline.items().await;
        items
            .iter()
            .filter_map(|item| {
                item.as_event()
                    .and_then(|event| event.event_id().map(|event_id| event_id.to_string()))
            })
            .collect()
    }

    async fn unread_open_plan(
        &self,
        client: &Client,
        room: &Room,
        room_id: &str,
    ) -> UnreadOpenPlan {
        let live_ids = self.live_event_ids(room_id).await;
        let signals = own_read_signals(room, client.user_id()).await;
        let mut receipts = signals.receipts;
        if let (Some(entry), Some(user_id)) = (self.entries.get(room_id), client.user_id()) {
            if let Some(event_id) = entry
                .timeline
                .latest_user_read_receipt_timeline_event_id(user_id)
                .await
            {
                let event_id = event_id.to_string();
                if !receipts.iter().any(|(existing, _)| existing == &event_id) {
                    receipts.push((event_id, None));
                }
            }
        }
        plan_unread_open(&live_ids, signals.fully_read.as_deref(), &receipts)
    }

    async fn open_focused_unread(
        &mut self,
        room: &Room,
        room_id: &str,
        anchor_event_id: &str,
        invalid_id: &'static str,
        open_failed: &'static str,
    ) -> Result<(Arc<Timeline>, TimelineViewPosition, TimelinePaginationState), &'static str> {
        let event_id = parse_event_id(anchor_event_id).map_err(|_| invalid_id)?;
        let key = (room_id.to_owned(), event_id.to_string());
        if !self.focused_entries.contains_key(&key) {
            if self.focused_entries.len() >= MAX_FOCUSED_EVENT_READBACKS {
                if let Some(oldest_key) = self.focused_entries.keys().next().cloned() {
                    self.focused_entries.remove(&oldest_key);
                }
            }
            let timeline = TimelineBuilder::new(room)
                .with_focus(TimelineFocus::Event {
                    target: event_id.clone(),
                    num_context_events: FOCUSED_CONTEXT_EVENT_COUNT,
                    thread_mode: TimelineEventFocusThreadMode::Automatic {
                        hide_threaded_events: false,
                    },
                })
                .build()
                .await
                .map_err(|_| open_failed)?;
            self.focused_entries.insert(key.clone(), Arc::new(timeline));
        }
        let timeline = self
            .focused_entries
            .get(&key)
            .expect("unread frontier timeline present")
            .clone();
        Ok((
            timeline,
            TimelineViewPosition::Unread {
                anchor_event_id: event_id.to_string(),
            },
            TimelinePaginationState {
                backward: TimelinePageState::Available,
                forward: TimelinePageState::Available,
            },
        ))
    }

    pub async fn resolve_media(&self, handle_id: &str) -> Option<TimelineMediaSource> {
        for stream in self.view_streams.values() {
            let registry = stream.media.lock().await;
            if registry.session_generation() != self.session_generation {
                continue;
            }
            if let Some(source) = registry.resolve(handle_id) {
                return Some(source.clone());
            }
        }
        None
    }

    pub async fn paginate_legacy(
        &mut self,
        client: &Client,
        room_id: &str,
        direction: NativeTimelineDirection,
    ) -> Result<NativeTimelineSnapshot, &'static str> {
        let room_id = parse_room_id(room_id)?.to_string();
        let entry = self
            .entries
            .get_mut(&room_id)
            .ok_or("d0.3-timeline-not-open")?;
        let reached_end = match direction {
            NativeTimelineDirection::Backwards => entry
                .timeline
                .paginate_backwards(PAGINATION_BATCH_SIZE)
                .await
                .map_err(|_| "d0.3-timeline-paginate-backwards-failed")?,
            NativeTimelineDirection::Forwards => entry
                .timeline
                .paginate_forwards(PAGINATION_BATCH_SIZE)
                .await
                .map_err(|_| "d0.3-timeline-paginate-forwards-failed")?,
        };
        if direction == NativeTimelineDirection::Backwards {
            entry.hit_start = reached_end;
        }
        let mut snapshot = snapshot_from_timeline(
            self.session_generation,
            room_id.clone(),
            &entry.timeline,
            entry.is_encrypted,
            entry.hit_start,
            client.user_id(),
        )
        .await?;
        self.reconcile_utd(&mut snapshot, UtdRecoveryKind::EncryptedHistoryRecovery)?;
        Ok(snapshot)
    }

    pub async fn event_readback(
        &mut self,
        client: &Client,
        room_id: &str,
        event_id: &str,
    ) -> Result<NativeTimelineEventReadback, &'static str> {
        let room_id = parse_room_id(room_id)?.to_string();
        let event_id = parse_event_id(event_id)?;
        let key = (room_id.clone(), event_id.to_string());
        if !self.focused_entries.contains_key(&key) {
            if self.focused_entries.len() >= MAX_FOCUSED_EVENT_READBACKS {
                if let Some(oldest_key) = self.focused_entries.keys().next().cloned() {
                    self.focused_entries.remove(&oldest_key);
                }
            }
            let room = client
                .get_room(parse_room_id(&room_id)?.as_ref())
                .ok_or("v-crypto.6-event-room-not-found")?;
            let timeline = TimelineBuilder::new(&room)
                .with_focus(TimelineFocus::Event {
                    target: event_id.clone(),
                    num_context_events: 0,
                    thread_mode: TimelineEventFocusThreadMode::Automatic {
                        hide_threaded_events: false,
                    },
                })
                .build()
                .await
                .map_err(|_| "v-crypto.6-event-open-failed")?;
            self.focused_entries.insert(key.clone(), Arc::new(timeline));
        }
        let timeline = self
            .focused_entries
            .get(&key)
            .expect("focused timeline inserted");
        let (items, _updates) = timeline.subscribe().await;
        let item = items
            .iter()
            .filter_map(|item| project_item(item, client.user_id()))
            .find(|item| item.event_id == event_id.as_str())
            .ok_or("v-crypto.6-event-not-found")?;
        Ok(NativeTimelineEventReadback {
            session_generation: self.session_generation,
            room_id,
            event_id: event_id.to_string(),
            item,
        })
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
        let room_id = parse_room_id(room_id)?.to_string();
        let target_event_id = parse_event_id(target_event_id)?;
        validate_reaction_key(key)?;
        self.open(client, &room_id).await?;
        let timeline = self
            .entries
            .get(&room_id)
            .ok_or("v-send.2-reaction-timeline-not-open")?
            .timeline
            .clone();
        let added = timeline
            .toggle_reaction(&TimelineEventItemId::EventId(target_event_id.clone()), key)
            .await
            .map_err(|_| "v-send.2-reaction-toggle-failed")?;
        let readback = self
            .reaction_readback(client, &room_id, &target_event_id, key)
            .await?;
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

    /// Idempotently ensure an approval annotation exists. This intentionally
    /// does *not* call `toggle_reaction`: an existing reaction remains present.
    pub async fn ensure_reaction(
        &mut self,
        client: &Client,
        room_id: &str,
        target_event_id: &str,
        key: &str,
    ) -> Result<NativeReactionMutationResult, &'static str> {
        let room_id = parse_room_id(room_id)?.to_string();
        let target_event_id = parse_event_id(target_event_id)?;
        validate_reaction_key(key)?;
        let before = self
            .reaction_readback(client, &room_id, &target_event_id, key)
            .await?;
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
        room.send(ReactionEventContent::from(Annotation::new(
            target_event_id.clone(),
            key.to_owned(),
        )))
        .await
        .map_err(|_| "v-send.2-reaction-ensure-failed")?;

        let readback = self
            .reaction_readback(client, &room_id, &target_event_id, key)
            .await?;
        Ok(NativeReactionMutationResult {
            room_id,
            target_event_id: target_event_id.to_string(),
            key: key.to_owned(),
            mutation: NativeReactionMutation::Added,
            readback,
        })
    }

    /// Redact any reaction annotation selected in the viewer. Aggregated
    /// annotations are not timeline rows, so this correctly uses the native
    /// room owner rather than `Timeline::redact`.
    pub async fn redact_reaction(
        &mut self,
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
        let selected_reaction = self
            .reaction_readback(client, &room_id, &target_event_id, key)
            .await?
            .ok_or("v-send.2-reaction-redact-annotation-not-found")?;
        if !reaction_contains_event_id(&selected_reaction, &reaction_event_id) {
            return Err("v-send.2-reaction-redact-annotation-not-found");
        }
        let room = client
            .get_room(parse_room_id(&room_id)?.as_ref())
            .ok_or("v-send.2-reaction-room-not-found")?;
        room.redact(&reaction_event_id, Some("Removed reaction"), None)
            .await
            .map_err(|_| "v-send.2-reaction-redact-failed")?;
        let readback = self
            .reaction_readback(client, &room_id, &target_event_id, key)
            .await?;
        Ok(NativeReactionMutationResult {
            room_id,
            target_event_id: target_event_id.to_string(),
            key: key.to_owned(),
            mutation: NativeReactionMutation::Redacted,
            readback,
        })
    }

    async fn reaction_readback(
        &mut self,
        client: &Client,
        room_id: &str,
        target_event_id: &OwnedEventId,
        key: &str,
    ) -> Result<Option<NativeTimelineReaction>, &'static str> {
        self.open(client, room_id).await?;
        let entry = self
            .entries
            .get(room_id)
            .ok_or("v-send.2-reaction-timeline-not-open")?;
        let (items, _updates) = entry.timeline.subscribe().await;
        if let Some(reaction) = items
            .iter()
            .filter_map(|item| project_item(item, client.user_id()))
            .find(|item| item.event_id == target_event_id.as_str())
            .and_then(|item| {
                item.reactions
                    .into_iter()
                    .find(|reaction| reaction.key == key)
            })
        {
            return Ok(Some(reaction));
        }

        // Notifications may target a message outside the currently open
        // viewport. A focused native timeline is the same authoritative owner
        // used for event readback; no JS relation inspection is involved.
        let focus_key = (room_id.to_owned(), target_event_id.to_string());
        if !self.focused_entries.contains_key(&focus_key) {
            let room = client
                .get_room(parse_room_id(room_id)?.as_ref())
                .ok_or("v-send.2-reaction-room-not-found")?;
            let timeline = TimelineBuilder::new(&room)
                .with_focus(TimelineFocus::Event {
                    target: target_event_id.clone(),
                    num_context_events: 0,
                    thread_mode: TimelineEventFocusThreadMode::Automatic {
                        hide_threaded_events: false,
                    },
                })
                .build()
                .await
                .map_err(|_| "v-send.2-reaction-readback-open-failed")?;
            self.focused_entries
                .insert(focus_key.clone(), Arc::new(timeline));
        }
        let timeline = self
            .focused_entries
            .get(&focus_key)
            .ok_or("v-send.2-reaction-readback-open-failed")?;
        let (items, _updates) = timeline.subscribe().await;
        Ok(items
            .iter()
            .filter_map(|item| project_item(item, client.user_id()))
            .find(|item| item.event_id == target_event_id.as_str())
            .and_then(|item| {
                item.reactions
                    .into_iter()
                    .find(|reaction| reaction.key == key)
            }))
    }

    fn reconcile_utd(
        &mut self,
        snapshot: &mut NativeTimelineSnapshot,
        kind: UtdRecoveryKind,
    ) -> Result<(), &'static str> {
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
            {
                self.utd_index
                    .mark_unavailable(
                        TimelineEncryptedUnavailableItem {
                            item_id: item.item_id.clone(),
                            event_id: item.event_id.clone(),
                            room_id: room_id.clone(),
                            reason: Some(reason.as_str().to_owned()),
                        },
                        reason,
                    )
                    .map_err(|_| "v-crypto.6-utd-index-failed")?;
            }
            match item.decryption_state {
                Some(NativeDecryptionState::Unavailable) => {}
                Some(NativeDecryptionState::Pending) => {
                    if self
                        .utd_index
                        .get(&room_id, &item.event_id)
                        .map(|e| e.phase)
                        == Some(UtdPhase::UnableToDecrypt)
                    {
                        self.utd_index
                            .begin_retry(&room_id, &item.event_id)
                            .map_err(|_| "v-crypto.6-utd-index-failed")?;
                    }
                }
                None => {}
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
            self.utd_index
                .mark_decrypted(&room_id, event_id)
                .map_err(|_| "v-crypto.6-utd-index-failed")?;
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
            let op_id = self
                .utd_recovery
                .begin(room_id.clone(), kind, pending_ids)
                .map_err(|_| "v-crypto.6-recovery-state-failed")?;
            self.utd_recovery
                .mark_in_flight(&room_id, op_id)
                .map_err(|_| "v-crypto.6-recovery-state-failed")?;
        }
        if let Some(session) = self.utd_recovery.get(&room_id).cloned() {
            if session.phase.is_active() {
                let recovered = session.recovered_count.saturating_add(newly_recovered);
                if utd_count == 0 {
                    self.utd_recovery
                        .succeed(&room_id, session.op_id, recovered, 0)
                        .map_err(|_| "v-crypto.6-recovery-state-failed")?;
                } else {
                    self.utd_recovery
                        .report_progress(&room_id, session.op_id, newly_recovered, utd_count)
                        .map_err(|_| "v-crypto.6-recovery-state-failed")?;
                }
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
        Ok(())
    }
}

impl Drop for NativeTimelineRegistry {
    fn drop(&mut self) {
        for (_, task) in self.view_update_tasks.drain() {
            task.abort();
        }
    }
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
enum UnreadOpenPlan {
    /// Last-read is already in the live window. Stay live and place there.
    InLive { event_id: String },
    /// Last-read is an own receipt outside the live window. Bounded focus.
    FocusedReceipt { event_id: String },
    /// Newest frontier cannot be established without walking history.
    LiveBottom,
}

struct OwnReadSignals {
    fully_read: Option<String>,
    receipts: Vec<(String, Option<u64>)>,
}

fn newest_receipt_event_id(receipts: &[(String, Option<u64>)]) -> Option<String> {
    receipts
        .iter()
        .max_by(|left, right| match (left.1, right.1) {
            (Some(left_ts), Some(right_ts)) => left_ts.cmp(&right_ts),
            (Some(_), None) => std::cmp::Ordering::Greater,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (None, None) => std::cmp::Ordering::Equal,
        })
        .map(|(event_id, _)| event_id.clone())
}

fn plan_unread_open(
    live_event_ids: &[String],
    fully_read: Option<&str>,
    receipts: &[(String, Option<u64>)],
) -> UnreadOpenPlan {
    let mut live_candidates: Vec<&str> = receipts.iter().map(|(id, _)| id.as_str()).collect();
    if let Some(event_id) = fully_read {
        live_candidates.push(event_id);
    }
    if let Some(event_id) = newest_frontier_in_live(live_event_ids, live_candidates) {
        return UnreadOpenPlan::InLive { event_id };
    }
    // Unread counts are receipt-based. A receipt we have but have not loaded
    // into the live window is still last-read; `m.fully_read` outside that
    // window is not, because it can sit months behind the receipt.
    if let Some(event_id) = newest_receipt_event_id(receipts) {
        return UnreadOpenPlan::FocusedReceipt { event_id };
    }
    UnreadOpenPlan::LiveBottom
}

async fn own_read_signals(room: &Room, own_user_id: Option<&UserId>) -> OwnReadSignals {
    let fully_read = room
        .fully_read_event_id()
        .map(|event_id| event_id.to_string());
    let mut receipts = Vec::new();
    let Some(user_id) = own_user_id else {
        return OwnReadSignals {
            fully_read,
            receipts,
        };
    };
    for receipt_type in [EventReceiptType::Read, EventReceiptType::ReadPrivate] {
        if let Ok(Some((event_id, receipt))) = room
            .load_user_receipt(receipt_type, ReceiptThread::Unthreaded, user_id)
            .await
        {
            let event_id = event_id.to_string();
            if receipts.iter().any(|(existing, _)| existing == &event_id) {
                continue;
            }
            let ts = receipt.ts.map(|ts| ts.get().into());
            receipts.push((event_id, ts));
        }
    }
    OwnReadSignals {
        fully_read,
        receipts,
    }
}

fn live_pagination_state(entry: &LiveTimelineEntry) -> TimelinePaginationState {
    TimelinePaginationState {
        backward: if entry.hit_start {
            TimelinePageState::Exhausted
        } else {
            TimelinePageState::Available
        },
        forward: TimelinePageState::Available,
    }
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
        TimelineViewPosition::Restored {
            anchor_event_id: Some(anchor_event_id),
        } => format!("restored:{room_id}:{anchor_event_id}"),
        TimelineViewPosition::Restored {
            anchor_event_id: None,
        } => format!("restored:{room_id}:none"),
    }
}

struct ViewUpdateOwnerInput {
    emit: TimelineViewUpdateEmit,
    session_generation: u64,
    stream_id: String,
    room_id: String,
    own_user_id: Option<OwnedUserId>,
    revision: Arc<AtomicU64>,
    media: Arc<AsyncMutex<TimelineMediaRegistry>>,
    item_ids: Vec<String>,
    timeline: Arc<Timeline>,
    position: TimelineViewPosition,
    hit_start: Arc<AtomicBool>,
    hydrate_sender_profiles: bool,
}
fn spawn_view_update_owner(
    input: ViewUpdateOwnerInput,
    updates: impl futures_util::Stream<Item = Vec<VectorDiff<Arc<SdkTimelineItem>>>> + Send + 'static,
) -> JoinHandle<()> {
    let ViewUpdateOwnerInput {
        emit,
        session_generation,
        stream_id,
        room_id,
        own_user_id,
        revision,
        media,
        mut item_ids,
        timeline,
        position,
        hit_start,
        hydrate_sender_profiles,
    } = input;
    let client = timeline.room().client();
    let (power_authority_tx, mut power_authority_rx) = tokio::sync::mpsc::unbounded_channel();
    let power_handler = client.add_event_handler(move |event: AnySyncStateEvent| {
        let power_authority_tx = power_authority_tx.clone();
        async move {
            if event.event_type() == StateEventType::RoomPowerLevels {
                let _ = power_authority_tx.send(());
            }
        }
    });
    let power_handler = client.event_handler_drop_guard(power_handler);
    tokio::spawn(async move {
        let _power_handler = power_handler;
        let emitter = ViewDeltaEmitter::new(emit, session_generation, stream_id, room_id, revision);
        let mut last_read_state =
            project_live_read_state(&timeline, &position, own_user_id.as_deref()).await;
        let mut last_pagination =
            pagination_state_from_hit_start(hit_start.load(Ordering::Acquire));
        let mut last_pinned_event_ids = project_pinned_event_ids(timeline.room());
        let mut last_action_authority =
            room_action_authority(timeline.room(), own_user_id.as_deref()).await;

        let mut room_info = timeline.room().subscribe_info();
        let mut read_receipts = timeline.subscribe_own_user_read_receipts_changed().await;
        let mut pagination_updates: std::pin::Pin<
            Box<dyn futures_util::Stream<Item = PaginationStatus> + Send>,
        > = match timeline.live_back_pagination_status().await {
            Some((current, stream)) => {
                last_pagination = pagination_state_from_status(current, &hit_start);
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

        loop {
            tokio::select! {
                () = &mut member_hydration, if !members_hydrated => {
                    members_hydrated = true;
                }
                Some(diffs) = updates.next() => {
                    let action_authority = room_action_authority(
                        timeline.room(),
                        own_user_id.as_deref(),
                    ).await;
                    apply_item_id_diffs(&mut item_ids, &diffs);
                    let ops = {
                        let mut registry = media.lock().await;
                        registry.retain_items(item_ids.iter().map(String::as_str));
                        project_timeline_diffs_with_media(
                            &diffs,
                            own_user_id.as_deref(),
                            action_authority,
                            &mut registry,
                        )
                    };
                    if ops.is_empty() {
                        continue;
                    }
                    emitter.emit(ops, None, None, None);
                }
                Some(_) = room_info.next() => {
                    let read_state =
                        project_live_read_state(&timeline, &position, own_user_id.as_deref()).await;
                    let pinned_event_ids = project_pinned_event_ids(timeline.room());
                    let action_authority =
                        room_action_authority(timeline.room(), own_user_id.as_deref()).await;
                    let read_changed = read_state != last_read_state;
                    let pins_changed = pinned_event_ids != last_pinned_event_ids;
                    let authority_changed = action_authority != last_action_authority;
                    if !read_changed && !pins_changed && !authority_changed {
                        continue;
                    }
                    if read_changed {
                        last_read_state = read_state.clone();
                    }
                    if pins_changed {
                        last_pinned_event_ids = pinned_event_ids.clone();
                    }
                    let ops = if authority_changed {
                        last_action_authority = action_authority;
                        let items = timeline.items().await;
                        item_ids = items
                            .iter()
                            .map(|item| item.unique_id().0.clone())
                            .collect();
                        let rows = {
                            let mut registry = media.lock().await;
                            registry.retain_items(item_ids.iter().map(String::as_str));
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
                        vec![super::TimelineViewDeltaOp::Reset { rows }]
                    } else {
                        Vec::new()
                    };
                    emitter.emit(
                        ops,
                        read_changed.then_some(read_state),
                        None,
                        pins_changed.then_some(pinned_event_ids),
                    );
                }
                Some(()) = power_authority_rx.recv() => {
                    let action_authority =
                        room_action_authority(timeline.room(), own_user_id.as_deref()).await;
                    if action_authority == last_action_authority {
                        continue;
                    }
                    last_action_authority = action_authority;
                    let items = timeline.items().await;
                    item_ids = items
                        .iter()
                        .map(|item| item.unique_id().0.clone())
                        .collect();
                    let rows = {
                        let mut registry = media.lock().await;
                        registry.retain_items(item_ids.iter().map(String::as_str));
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
                    emitter.emit(
                        vec![super::TimelineViewDeltaOp::Reset { rows }],
                        None,
                        None,
                        None,
                    );
                }
                Some(()) = read_receipts.next() => {
                    let read_state =
                        project_live_read_state(&timeline, &position, own_user_id.as_deref()).await;
                    if read_state == last_read_state {
                        continue;
                    }
                    last_read_state = read_state.clone();
                    emitter.emit(Vec::new(), Some(read_state), None, None);
                }
                Some(status) = pagination_updates.next() => {
                    let pagination = pagination_state_from_status(status, &hit_start);
                    if pagination == last_pagination {
                        continue;
                    }
                    last_pagination = pagination.clone();
                    emitter.emit(Vec::new(), None, Some(pagination), None);
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

fn pagination_state_from_status(
    status: PaginationStatus,
    hit_start: &AtomicBool,
) -> TimelinePaginationState {
    let backward = match status {
        PaginationStatus::Paginating => TimelinePageState::Loading,
        PaginationStatus::Idle {
            hit_timeline_start: true,
        } => {
            hit_start.store(true, Ordering::Release);
            TimelinePageState::Exhausted
        }
        PaginationStatus::Idle {
            hit_timeline_start: false,
        } => {
            if hit_start.load(Ordering::Acquire) {
                TimelinePageState::Exhausted
            } else {
                TimelinePageState::Available
            }
        }
    };
    TimelinePaginationState {
        backward,
        forward: TimelinePageState::Available,
    }
}

async fn project_live_read_state(
    timeline: &Timeline,
    position: &TimelineViewPosition,
    own_user_id: Option<&UserId>,
) -> TimelineReadState {
    let unread_anchor_event_id = match position {
        TimelineViewPosition::Unread { anchor_event_id } => Some(anchor_event_id.clone()),
        _ => None,
    };
    let own_read_event_id = match timeline.room().fully_read_event_id() {
        Some(event_id) => Some(event_id.to_string()),
        None => match own_user_id {
            Some(user_id) => timeline
                .latest_user_read_receipt_timeline_event_id(user_id)
                .await
                .map(|event_id| event_id.to_string()),
            None => None,
        },
    };
    TimelineReadState {
        own_read_event_id,
        unread_anchor_event_id,
        is_marked_unread: timeline.room().is_marked_unread(),
    }
}

/// Build the currently available native view snapshot from the SDK owner.
///
/// `revision` is zero until the native delta subscriber owns monotonically
/// advancing revisions. Treating repeated snapshot reads as deltas would hide
/// the missing owner boundary, so this contract makes that absence explicit.
async fn view_snapshot_from_timeline(
    input: TimelineViewSnapshotInput,
    timeline: &Timeline,
    media: Arc<AsyncMutex<TimelineMediaRegistry>>,
) -> TimelineViewSnapshot {
    let (items, _updates) = timeline.subscribe().await;
    let action_authority =
        room_action_authority(timeline.room(), input.own_user_id.as_deref()).await;
    let rows = {
        let mut registry = media.lock().await;
        registry.retain_items(items.iter().map(|item| item.unique_id().0.as_str()));
        items
            .iter()
            .map(|item| {
                project_timeline_item_with_media(
                    item,
                    input.own_user_id.as_deref(),
                    action_authority,
                    &mut registry,
                )
            })
            .collect()
    };
    view_snapshot_from_items(input, timeline, rows).await
}

fn apply_item_id_diffs(item_ids: &mut Vec<String>, diffs: &[VectorDiff<Arc<SdkTimelineItem>>]) {
    for diff in diffs {
        match diff {
            VectorDiff::Append { values } => {
                item_ids.extend(values.iter().map(|item| item.unique_id().0.clone()))
            }
            VectorDiff::Clear => item_ids.clear(),
            VectorDiff::PushFront { value } => item_ids.insert(0, value.unique_id().0.clone()),
            VectorDiff::PushBack { value } => item_ids.push(value.unique_id().0.clone()),
            VectorDiff::PopFront => {
                if !item_ids.is_empty() {
                    item_ids.remove(0);
                }
            }
            VectorDiff::PopBack => {
                item_ids.pop();
            }
            VectorDiff::Insert { index, value } => {
                if *index <= item_ids.len() {
                    item_ids.insert(*index, value.unique_id().0.clone());
                }
            }
            VectorDiff::Set { index, value } => {
                if let Some(item_id) = item_ids.get_mut(*index) {
                    *item_id = value.unique_id().0.clone();
                }
            }
            VectorDiff::Remove { index } => {
                if *index < item_ids.len() {
                    item_ids.remove(*index);
                }
            }
            VectorDiff::Truncate { length } => item_ids.truncate(*length),
            VectorDiff::Reset { values } => {
                *item_ids = values
                    .iter()
                    .map(|item| item.unique_id().0.clone())
                    .collect();
            }
        }
    }
}

struct TimelineViewSnapshotInput {
    session_generation: u64,
    room_id: String,
    position: TimelineViewPosition,
    pagination: TimelinePaginationState,
    own_user_id: Option<OwnedUserId>,
    revision: u64,
}

async fn view_snapshot_from_items(
    input: TimelineViewSnapshotInput,
    timeline: &Timeline,
    rows: Vec<super::TimelineViewRow>,
) -> TimelineViewSnapshot {
    let read_state =
        project_live_read_state(timeline, &input.position, input.own_user_id.as_deref()).await;
    TimelineViewSnapshot {
        schema_version: TIMELINE_VIEW_SCHEMA_VERSION,
        session_generation: input.session_generation,
        room_id: input.room_id,
        revision: input.revision,
        position: input.position,
        pagination: input.pagination,
        read_state,
        pinned_event_ids: project_pinned_event_ids(timeline.room()),
        rows,
        capabilities: TimelineViewCapabilities {
            mark_read: true,
            mark_unread: true,
            paginate_backward: true,
            paginate_forward: true,
        },
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
    let items = items
        .iter()
        .filter_map(|item| project_item(item, local_user))
        .collect();
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
                    reaction_event_id: match &info.status {
                        ReactionStatus::RemoteToRemote(event_id) => Some(event_id.to_string()),
                        ReactionStatus::LocalToLocal(_) | ReactionStatus::LocalToRemote(_) => None,
                    },
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
        assert!(validate_poll_vote_selection(Vec::new(), &answers, 1, false).is_ok());
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
    fn unread_plan_stays_live_when_the_receipt_is_in_the_live_window() {
        let live = vec![
            "$july-fully-read:example.org".into(),
            "$last-night-receipt:example.org".into(),
            "$overnight:example.org".into(),
        ];
        assert_eq!(
            plan_unread_open(
                &live,
                Some("$july-fully-read:example.org"),
                &[("$last-night-receipt:example.org".into(), Some(2))],
            ),
            UnreadOpenPlan::InLive {
                event_id: "$last-night-receipt:example.org".into(),
            }
        );
    }

    #[test]
    fn unread_plan_ignores_stale_fully_read_outside_live_when_a_receipt_exists() {
        let live = vec!["$recent:example.org".into(), "$tip:example.org".into()];
        assert_eq!(
            plan_unread_open(
                &live,
                Some("$july-fully-read:example.org"),
                &[("$last-night-receipt:example.org".into(), Some(2))],
            ),
            UnreadOpenPlan::FocusedReceipt {
                event_id: "$last-night-receipt:example.org".into(),
            }
        );
    }

    #[test]
    fn unread_plan_does_not_focus_a_stale_fully_read_without_a_receipt() {
        let live = vec!["$recent:example.org".into(), "$tip:example.org".into()];
        assert_eq!(
            plan_unread_open(&live, Some("$july-fully-read:example.org"), &[]),
            UnreadOpenPlan::LiveBottom
        );
    }

    #[test]
    fn unread_plan_picks_the_newer_receipt_by_receipt_timestamp() {
        assert_eq!(
            plan_unread_open(
                &[],
                Some("$july-fully-read:example.org"),
                &[
                    ("$old-public:example.org".into(), Some(1)),
                    ("$new-private:example.org".into(), Some(9)),
                ],
            ),
            UnreadOpenPlan::FocusedReceipt {
                event_id: "$new-private:example.org".into(),
            }
        );
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

        let hit_start = AtomicBool::new(false);
        assert_eq!(
            pagination_state_from_status(
                PaginationStatus::Idle {
                    hit_timeline_start: true
                },
                &hit_start
            ),
            TimelinePaginationState {
                backward: TimelinePageState::Exhausted,
                forward: TimelinePageState::Available,
            }
        );
        assert!(hit_start.load(Ordering::Acquire));
        assert_eq!(
            pagination_state_from_status(PaginationStatus::Paginating, &hit_start).backward,
            TimelinePageState::Loading
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
    fn room_read_state_sends_receipts_and_clears_marked_unread_without_a_view_stream() {
        let source = include_str!("live.rs");
        assert!(source.contains("pub async fn set_room_read_state"));
        assert!(source.contains("self.open(client, &room_id_string).await?"));
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

        let cargo_lock = include_str!("../../../../../Cargo.lock");
        assert!(cargo_lock.contains("name = \"matrix-sdk-ui\"\nversion = \"0.18.0\""));
        let source = include_str!("live.rs");
        assert!(source.contains("Pinned matrix-sdk-ui 0.18 invariant"));
        assert!(source.contains("also when receipt deduplication removes every unchanged marker"));
        let mark_read_start = source.find("async fn mark_live_timeline_read").unwrap();
        let mark_read_end = source[mark_read_start..]
            .find("fn remember_agent_approval_decision")
            .map(|offset| mark_read_start + offset)
            .unwrap();
        let mark_read_source = &source[mark_read_start..mark_read_end];
        assert!(mark_read_source.contains("timeline.latest_event_id().await"));
        assert!(!mark_read_source.contains("items.iter().rev().find_map"));
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
        registry
            .reconcile_utd(&mut pending, UtdRecoveryKind::RetryDecrypt)
            .unwrap();
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
        registry
            .reconcile_utd(&mut decrypted, UtdRecoveryKind::RetryDecrypt)
            .unwrap();
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
        registry
            .reconcile_utd(&mut later_pending, UtdRecoveryKind::RetryDecrypt)
            .unwrap();
        let second_session = registry.utd_recovery.get("!room:example.org").unwrap();
        assert!(second_session.op_id > first_op_id);
        assert!(second_session.phase.is_active());

        later_pending.items[0].decryption_state = Some(NativeDecryptionState::Unavailable);
        registry
            .reconcile_utd(&mut later_pending, UtdRecoveryKind::RetryDecrypt)
            .unwrap();
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
        registry
            .reconcile_utd(&mut later_decrypted, UtdRecoveryKind::RetryDecrypt)
            .unwrap();
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
}
