//! D0.3 live Matrix SDK timeline ownership and privacy-safe projection.
//!
//! SDK timeline objects stay inside the Rust session. The webview receives a
//! product snapshot containing only stable identifiers, sender IDs,
//! event types, timestamps, and safe display text.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use eyeball_im::VectorDiff;
use futures_util::StreamExt;
use matrix_sdk::{
    ruma::{
    ruma::{
        api::client::receipt::create_receipt::v3::ReceiptType,
        events::{reaction::ReactionEventContent, relation::Annotation},
        OwnedEventId, OwnedRoomId, OwnedUserId,
    },
    },
    Client,
};
use matrix_sdk_crypto::types::events::UtdCause;
use matrix_sdk_ui::timeline::{
    EncryptedMessage, ReactionStatus, Timeline, TimelineBuilder, TimelineEventFocusThreadMode,
    TimelineEventItemId, TimelineFocus, TimelineItem as SdkTimelineItem,
    TimelineItemContent as SdkTimelineItemContent,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::task::JoinHandle;

use crate::matrix::{
    dto::TimelineEncryptedUnavailableItem,
    utd_recovery::{UtdRecoveryCoordinator, UtdRecoveryKind},
};

use super::{
    project_timeline_diffs, project_timeline_item, TimelinePageState, TimelinePaginationState,
    TimelineReadState, TimelineViewCapabilities, TimelineViewDeltaBatch, TimelineViewPosition,
    TimelineViewSnapshot, UtdIndex, UtdPhase, UtdReasonCode, NATIVE_TIMELINE_VIEW_UPDATED_EVENT,
    TIMELINE_VIEW_SCHEMA_VERSION,
};

const PAGINATION_BATCH_SIZE: u16 = 30;
const REDACTED_PLACEHOLDER: &str = "Message removed";
const UTD_PLACEHOLDER: &str = "Unable to decrypt this message";
const UNSUPPORTED_PLACEHOLDER: &str = "Unsupported event";
const MAX_FOCUSED_EVENT_READBACKS: usize = 256;
const FOCUSED_CONTEXT_EVENT_COUNT: u16 = 25;

/// Version of the bounded native timeline-open contract.
///
/// This is an implementation foundation for the full V-TIMELINE DTO boundary;
/// it is not a claim that the flat legacy snapshot is the final presenter
/// payload.
pub const NATIVE_TIMELINE_OPEN_SCHEMA_VERSION: u32 = 1;

/// Requested initial position for one native timeline view.
///
/// Restored-viewport positions deliberately remain unimplemented until their
/// native viewport contract is ready. Unread opens resolve the native
/// fully-read frontier and must not be silently treated as live-bottom.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NativeTimelineOpenPosition {
    LiveBottom,
    Unread,
    Focused { event_id: String },
}

/// Typed input for the native timeline-open owner.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineOpenRequest {
    pub room_id: String,
    pub position: NativeTimelineOpenPosition,
}

/// Bounded authoritative result of opening the requested native timeline
/// position. This is the versioned, SDK-neutral view boundary; it has no
/// active React consumer until the complete presenter cutover is ready.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineOpenReadback {
    pub schema_version: u32,
    /// Opaque identifier carried by every delta from this exact opened view.
    pub stream_id: String,
    pub position: NativeTimelineOpenPosition,
    pub snapshot: TimelineViewSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeTimelineDirection {
    Backwards,
    Forwards,
}

/// A pagination request addresses the exact opened view, rather than assuming
/// a room has only one timeline. `stream_id` comes from `matrix_timeline_open`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineViewPaginationRequest {
    pub stream_id: String,
    pub direction: NativeTimelineDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeTimelineReadAction {
    MarkRead,
    MarkUnread,
}

/// A read-state transition always targets one opened native timeline view.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineReadStateRequest {
    pub stream_id: String,
    pub action: NativeTimelineReadAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineReadStateReadback {
    pub action: NativeTimelineReadAction,
    /// `None` for an account-data unread-flag change; otherwise whether the
    /// SDK actually sent a private read receipt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_sent: Option<bool>,
    pub snapshot: TimelineViewSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineItem {
    pub item_id: String,
    pub event_id: String,
    pub sender: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub body: String,
    pub origin_server_ts: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decryption_state: Option<NativeDecryptionState>,
    /// Aggregated reactions are projected by the native timeline owner. The
    /// webview never derives reaction ownership from a Matrix JS timeline.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reactions: Vec<NativeTimelineReaction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineReaction {
    pub key: String,
    pub count: u32,
    pub me: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub senders: Vec<NativeTimelineReactionSender>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineReactionSender {
    pub user_id: String,
    /// Remote reaction annotations can be redacted by their event id. Local
    /// echoes intentionally have no fabricated event id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reaction_event_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeReactionMutation {
    Added,
    Removed,
    AlreadyPresent,
    Redacted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeReactionMutationResult {
    pub room_id: String,
    pub target_event_id: String,
    pub key: String,
    pub mutation: NativeReactionMutation,
    /// State reprojected from the same Rust timeline owner after the SDK call.
    pub readback: Option<NativeTimelineReaction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeDecryptionState {
    Pending,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeUtdPhase {
    Idle,
    Recovering,
    Partial,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeUtdStatus {
    pub phase: NativeUtdPhase,
    pub pending_count: u32,
    pub unavailable_count: u32,
    pub recovered_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineSnapshot {
    pub session_generation: u64,
    pub room_id: String,
    pub is_encrypted: bool,
    pub items: Vec<NativeTimelineItem>,
    pub hit_start: bool,
    pub utd: NativeUtdStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineEventReadback {
    pub session_generation: u64,
    pub room_id: String,
    pub event_id: String,
    pub item: NativeTimelineItem,
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
    hit_start: bool,
}

pub struct NativeTimelineRegistry {
    session_generation: u64,
    entries: HashMap<String, LiveTimelineEntry>,
    focused_entries: HashMap<(String, String), Arc<Timeline>>,
    view_streams: HashMap<String, ViewStreamEntry>,
    view_update_tasks: HashMap<String, JoinHandle<()>>,
    view_revisions: HashMap<String, Arc<AtomicU64>>,
    utd_index: UtdIndex,
    utd_recovery: UtdRecoveryCoordinator,
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
            let is_encrypted = room
                .latest_encryption_state()
                .await
                .map_err(|_| "d0.5-timeline-encryption-state-unavailable")?
                .is_encrypted();
            let timeline = TimelineBuilder::new(&room)
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
        app: AppHandle,
        client: &Client,
        request: NativeTimelineOpenRequest,
    ) -> Result<NativeTimelineOpenReadback, &'static str> {
        let room_id = parse_room_id(&request.room_id)?;
        let room_id_string = room_id.to_string();
        let position = request.position;
        let (timeline, view_position, pagination) = match &position {
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
                let anchor_event_id = room
                    .fully_read_event_id()
                    .ok_or("v-timeline-unread-frontier-unavailable")?;
                let key = (room_id_string.clone(), anchor_event_id.to_string());
                if !self.focused_entries.contains_key(&key) {
                    if self.focused_entries.len() >= MAX_FOCUSED_EVENT_READBACKS {
                        if let Some(oldest_key) = self.focused_entries.keys().next().cloned() {
                            self.focused_entries.remove(&oldest_key);
                        }
                    }
                    let timeline = TimelineBuilder::new(&room)
                        .with_focus(TimelineFocus::Event {
                            target: anchor_event_id.clone(),
                            num_context_events: FOCUSED_CONTEXT_EVENT_COUNT,
                            thread_mode: TimelineEventFocusThreadMode::Automatic {
                                hide_threaded_events: false,
                            },
                        })
                        .build()
                        .await
                        .map_err(|_| "v-timeline-unread-open-failed")?;
                    self.focused_entries.insert(key.clone(), Arc::new(timeline));
                }
                let timeline = self
                    .focused_entries
                    .get(&key)
                    .expect("unread frontier timeline present")
                    .clone();
                (
                    timeline,
                    TimelineViewPosition::Unread {
                        anchor_event_id: anchor_event_id.to_string(),
                    },
                    TimelinePaginationState {
                        backward: TimelinePageState::Available,
                        forward: TimelinePageState::Available,
                    },
                )
            }
        };
        let own_user_id = client.user_id().map(ToOwned::to_owned);
        let subscription_key = view_subscription_key(&room_id_string, &position);
        let revision = self
            .view_revisions
            .entry(subscription_key.clone())
            .or_insert_with(|| Arc::new(AtomicU64::new(0)))
            .clone();
        self.view_streams
            .entry(subscription_key.clone())
            .or_insert_with(|| ViewStreamEntry {
                room_id: room_id_string.clone(),
                timeline: timeline.clone(),
                position: view_position.clone(),
                hit_start: matches!(pagination.backward, TimelinePageState::Exhausted),
            });
        let snapshot = if self.view_update_tasks.contains_key(&subscription_key) {
            view_snapshot_from_timeline(
                self.session_generation,
                room_id_string.clone(),
                view_position,
                pagination,
                &timeline,
                own_user_id,
                revision.load(Ordering::Acquire),
            )
            .await
        } else {
            // Subscribe before materializing the initial rows so no SDK update
            // can fall into a snapshot-to-stream gap.
            let (items, updates) = timeline.subscribe().await;
            let snapshot = view_snapshot_from_items(
                TimelineViewSnapshotInput {
                    session_generation: self.session_generation,
                    room_id: room_id_string.clone(),
                    position: view_position,
                    pagination,
                    own_user_id: own_user_id.clone(),
                    revision: revision.load(Ordering::Acquire),
                },
                &timeline,
                items
                    .iter()
                    .map(|item| project_timeline_item(item, own_user_id.as_deref()))
                    .collect(),
            )
            .await;
            self.view_update_tasks.insert(
                subscription_key.clone(),
                spawn_view_update_owner(
                    app,
                    self.session_generation,
                    subscription_key.clone(),
                    room_id_string.clone(),
                    own_user_id,
                    revision,
                    updates,
                ),
            );
            snapshot
        };
        Ok(NativeTimelineOpenReadback {
            schema_version: NATIVE_TIMELINE_OPEN_SCHEMA_VERSION,
            stream_id: subscription_key,
            position,
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
        let (room_id, timeline, position, pagination) = {
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
                stream.hit_start = reached_end;
            }
            (
                stream.room_id.clone(),
                stream.timeline.clone(),
                stream.position.clone(),
                TimelinePaginationState {
                    backward: if stream.hit_start {
                        TimelinePageState::Exhausted
                    } else {
                        TimelinePageState::Available
                    },
                    forward: TimelinePageState::Available,
                },
            )
        };
        let revision = self
            .view_revisions
            .get(&request.stream_id)
            .ok_or("v-timeline-view-revision-missing")?
            .load(Ordering::Acquire);
        Ok(view_snapshot_from_timeline(
            self.session_generation,
            room_id,
            position,
            pagination,
            &timeline,
            client.user_id().map(ToOwned::to_owned),
            revision,
        )
        .await)
    }

    pub async fn set_read_state(
        &mut self,
        client: &Client,
        request: NativeTimelineReadStateRequest,
    ) -> Result<NativeTimelineReadStateReadback, &'static str> {
        let timeline = self
            .view_streams
            .get(&request.stream_id)
            .ok_or("v-timeline-view-not-open")?
            .timeline
            .clone();
        let receipt_sent = match request.action {
            NativeTimelineReadAction::MarkRead => Some(
                timeline
                    .mark_as_read(ReceiptType::ReadPrivate)
                    .await
                    .map_err(|_| "v-timeline-view-mark-read-failed")?,
            ),
            NativeTimelineReadAction::MarkUnread => {
                timeline
                    .room()
                    .set_unread_flag(true)
                    .await
                    .map_err(|_| "v-timeline-view-mark-unread-failed")?;
                None
            }
        };
        let snapshot = self
            .view_snapshot_for_stream(client, &request.stream_id)
            .await?;
        Ok(NativeTimelineReadStateReadback {
            action: request.action,
            receipt_sent,
            snapshot,
        })
    }

    async fn view_snapshot_for_stream(
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
            self.session_generation,
            stream.room_id.clone(),
            stream.position.clone(),
            TimelinePaginationState {
                backward: if stream.hit_start {
                    TimelinePageState::Exhausted
                } else {
                    TimelinePageState::Available
                },
                forward: TimelinePageState::Available,
            },
            &stream.timeline,
            client.user_id().map(ToOwned::to_owned),
            revision,
        )
        .await)
    }

    pub async fn paginate_legacy(
        &mut self,        room_id: &str,
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
                .take(crate::matrix::utd_recovery::MAX_EVENT_IDS_PER_BATCH)
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

    ruma::{
        events::{reaction::ReactionEventContent, relation::Annotation},
        OwnedEventId, OwnedRoomId, OwnedUserId,
    },
    session_generation: u64,
    stream_id: String,
    room_id: String,
    own_user_id: Option<OwnedUserId>,
    revision: Arc<AtomicU64>,
    updates: impl futures_util::Stream<Item = Vec<VectorDiff<Arc<SdkTimelineItem>>>> + Send + 'static,
    ) -> JoinHandle<()> {     tokio::spawn(async move {         futures_util::pin_mut!(updates);         while let Some(diffs) = updates.next().await {             let ops = project_timeline_diffs(&diffs,
    own_user_id.as_deref());             if ops.is_empty() {                 continue;             }             let next_revision = revision.fetch_add(1,
    Ordering::AcqRel).saturating_add(1);             let _ = app.emit(                 NATIVE_TIMELINE_VIEW_UPDATED_EVENT,
    TimelineViewDeltaBatch {                     schema_version: TIMELINE_VIEW_SCHEMA_VERSION,
    session_generation,
    stream_id: stream_id.clone(),
    room_id: room_id.clone(),
    revision: next_revision,
    ops,
    },
    );         }     }) }  /// Build the currently available native view snapshot from the SDK owner. /// /// `revision` is zero until the native delta subscriber owns monotonically /// advancing revisions. Treating repeated snapshot reads as deltas would hide /// the missing owner boundary,
    room_id: String,
    position: TimelineViewPosition,
    pagination: TimelinePaginationState,
    timeline: &Timeline,
    own_user_id: Option<OwnedUserId>,
    ) -> TimelineViewSnapshot {     let (items,
    _updates) = timeline.subscribe().await;     let own_read_event_id = match own_user_id.as_ref() {         Some(user_id) => timeline             .latest_user_read_receipt_timeline_event_id(&user_id)             .await             .map(|event_id| event_id.to_string()),
    None => None,
    };     TimelineViewSnapshot {         schema_version: TIMELINE_VIEW_SCHEMA_VERSION,
    session_generation,
    room_id,
    revision: 0,
    position,
    pagination,
    read_state: TimelineReadState {             own_read_event_id,
    unread_anchor_event_id: None,
    },
    rows: items             .iter()             .map(|item| project_timeline_item(item,
    own_user_id.as_deref()))             .collect(),
    capabilities: TimelineViewCapabilities {             mark_read: false,
    mark_unread: false,
    paginate_backward: true,
    paginate_forward: true,
    revision: u64,
) -> TimelineViewSnapshot {
    let (items, _updates) = timeline.subscribe().await;
    let rows = items
        .iter()
        .map(|item| project_timeline_item(item, own_user_id.as_deref()))
        .collect();
    view_snapshot_from_items(
        TimelineViewSnapshotInput {
            session_generation,
            room_id,
            position,
            pagination,
            own_user_id,
            revision,
        },
        timeline,
        rows,
    )
    .await
}

struct TimelineViewSnapshotInput {
    session_generation: u64,    room_id: String,
    position: TimelineViewPosition,
    pagination: TimelinePaginationState,
    own_user_id: Option<OwnedUserId>,
    revision: u64,
}

async fn view_snapshot_from_items(
    input: TimelineViewSnapshotInput,
    timeline: &Timeline,    rows: Vec<super::TimelineViewRow>,
) -> TimelineViewSnapshot {
    let unread_anchor_event_id = match &input.position {
        TimelineViewPosition::Unread { anchor_event_id } => Some(anchor_event_id.clone()),
        _ => None,
    };
    let own_read_event_id = match timeline.room().fully_read_event_id() {
        Some(event_id) => Some(event_id.to_string()),
        None => match input.own_user_id.as_ref() {
            Some(user_id) => timeline
                .latest_user_read_receipt_timeline_event_id(user_id)
                .await
                .map(|event_id| event_id.to_string()),
            None => None,
        },
    };
    TimelineViewSnapshot {
        schema_version: TIMELINE_VIEW_SCHEMA_VERSION,
        session_generation: input.session_generation,
        room_id: input.room_id,
        revision: input.revision,
        position: input.position,
        pagination: input.pagination,
        read_state: TimelineReadState {
            own_read_event_id,
            unread_anchor_event_id,
            is_marked_unread: timeline.room().is_marked_unread(),
        },
        rows,
        capabilities: TimelineViewCapabilities {
            mark_read: true,
            mark_unread: true,
            paginate_backward: true,
            paginate_forward: true,
        },
    }}

fn parse_room_id(room_id: &str) -> Result<OwnedRoomId, &'static str> {
    OwnedRoomId::try_from(room_id.trim()).map_err(|_| "d0.3-timeline-invalid-room-id")
}

fn parse_event_id(event_id: &str) -> Result<OwnedEventId, &'static str> {
    OwnedEventId::try_from(event_id.trim()).map_err(|_| "v-crypto.6-invalid-event-id")
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
    fn native_snapshot_schema_has_no_secret_or_ciphertext_fields() {
        let snapshot = NativeTimelineSnapshot {
            session_generation: 7,
            room_id: "!room:example.org".into(),
            is_encrypted: true,
            items: vec![NativeTimelineItem {
                item_id: "item-1".into(),
                event_id: "$event".into(),
                sender: "@alice:example.org".into(),
                event_type: "m.room.message".into(),
                body: "hello".into(),
                origin_server_ts: 42,
                decryption_state: None,
                reactions: vec![NativeTimelineReaction {
                    key: "✅".into(),
                    count: 1,
                    me: true,
                    senders: vec![NativeTimelineReactionSender {
                        user_id: "@alice:example.org".into(),
                        reaction_event_id: Some("$reaction".into()),
                    }],
                }],
            }],
            hit_start: false,
            utd: NativeUtdStatus {
                phase: NativeUtdPhase::Idle,
                pending_count: 0,
                unavailable_count: 0,
                recovered_count: 0,
            },
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        for forbidden in [
            "accessToken",
            "access_token",
            "refreshToken",
            "refresh_token",
            "sessionKey",
            "ciphertext",
        ] {
            assert!(!json.contains(forbidden));
        }
        assert!(json.contains("\"type\":\"m.room.message\""));
        assert!(json.contains("\"body\":\"hello\""));
        assert!(json.contains("\"isEncrypted\":true"));
        assert!(json.contains("\"reactionEventId\":\"$reaction\""));
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
    fn reaction_mutation_readback_schema_has_no_secret_fields() {
        let result = NativeReactionMutationResult {
            room_id: "!room:example.org".into(),
            target_event_id: "$event:example.org".into(),
            key: "✅".into(),
            mutation: NativeReactionMutation::AlreadyPresent,
            readback: Some(NativeTimelineReaction {
                key: "✅".into(),
                count: 2,
                me: true,
                senders: vec![NativeTimelineReactionSender {
                    user_id: "@alice:example.org".into(),
                    reaction_event_id: Some("$reaction:example.org".into()),
                }],
            }),
        };
        let json = serde_json::to_string(&result).unwrap();
        for forbidden in [
            "accessToken",
            "access_token",
            "refreshToken",
            "refresh_token",
            "sessionKey",
            "ciphertext",
            "private_key",
        ] {
            assert!(!json.contains(forbidden));
        }
        assert!(json.contains("\"mutation\":\"already_present\""));
        assert!(json.contains("\"reactionEventId\":\"$reaction:example.org\""));
        assert!(json.contains("\"me\":true"));
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
    fn focused_open_request_keeps_the_event_link_at_the_native_boundary() {
        let request: NativeTimelineOpenRequest = serde_json::from_value(serde_json::json!({
            "roomId": "!room:example.org",
            "position": { "kind": "focused", "event_id": "$event:example.org" }
        }))
        .unwrap();
        assert_eq!(request.room_id, "!room:example.org");
        assert_eq!(
            request.position,
            NativeTimelineOpenPosition::Focused {
                event_id: "$event:example.org".into()
            }
        );
    }
    #[test]
    fn unread_open_request_stays_distinct_from_live_bottom() {
        let request: NativeTimelineOpenRequest = serde_json::from_value(serde_json::json!({
            "roomId": "!room:example.org",
            "position": { "kind": "unread" }
        }))
        .unwrap();
        assert_eq!(request.position, NativeTimelineOpenPosition::Unread);
    }

    #[test]
    fn typed_open_readback_uses_the_versioned_view_boundary() {
        let readback = NativeTimelineOpenReadback {
            schema_version: NATIVE_TIMELINE_OPEN_SCHEMA_VERSION,
            stream_id: "live:!room:example.org".into(),
            position: NativeTimelineOpenPosition::LiveBottom,
            snapshot: TimelineViewSnapshot {
                schema_version: TIMELINE_VIEW_SCHEMA_VERSION,
                session_generation: 7,
                room_id: "!room:example.org".into(),
                revision: 0,
                position: TimelineViewPosition::LiveBottom,
                pagination: TimelinePaginationState {
                    backward: TimelinePageState::Available,
                    forward: TimelinePageState::Available,
                },
                read_state: TimelineReadState {
                    own_read_event_id: None,
                    unread_anchor_event_id: None,
                    is_marked_unread: false,
                },
                rows: Vec::new(),
                capabilities: TimelineViewCapabilities {
                    mark_read: false,
                    mark_unread: false,
                    paginate_backward: true,
                    paginate_forward: true,
                },
            },
        };
        let json = serde_json::to_value(readback).unwrap();
        let snapshot = &json["snapshot"];
        assert_eq!(snapshot["schemaVersion"], TIMELINE_VIEW_SCHEMA_VERSION);
        assert_eq!(snapshot["roomId"], "!room:example.org");
        assert!(snapshot.get("isEncrypted").is_none());
        assert!(snapshot.get("items").is_none());
        assert_eq!(snapshot["readState"]["isMarkedUnread"], false);
    }

    #[test]
    fn view_subscription_keys_keep_focuses_isolated_from_live_timeline() {
        assert_eq!(
            view_subscription_key("!room:example.org", &NativeTimelineOpenPosition::LiveBottom),
            "live:!room:example.org"
        );
        assert_eq!(
            view_subscription_key(
                "!room:example.org",
                &NativeTimelineOpenPosition::Focused {
                    event_id: "$one:example.org".into()
                }
            ),
            "focused:!room:example.org:$one:example.org"
        );
        assert_eq!(
            view_subscription_key("!room:example.org", &NativeTimelineOpenPosition::Unread),
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
            "action": "mark_unread"
        }))
        .unwrap();
        assert_eq!(request.stream_id, "live:!room:example.org");
        assert_eq!(request.action, NativeTimelineReadAction::MarkUnread);
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
}
