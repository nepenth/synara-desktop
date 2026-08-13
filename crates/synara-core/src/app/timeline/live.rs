//! D0.3 live Matrix SDK timeline ownership and privacy-safe projection.
//!
//! SDK timeline objects stay inside the Rust session. The webview receives a
//! product snapshot containing only stable identifiers, sender IDs,
//! event types, timestamps, and safe display text.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use eyeball_im::VectorDiff;
use futures_util::{stream, StreamExt};
use matrix_sdk::{
    event_cache::PaginationStatus,
    ruma::{
        api::client::receipt::create_receipt::v3::ReceiptType,
        events::{reaction::ReactionEventContent, relation::Annotation},
        OwnedEventId, OwnedRoomId, OwnedUserId, UserId,
    },
    Client, Room,
};
use matrix_sdk_crypto::types::events::UtdCause;
use matrix_sdk_ui::timeline::{
    EncryptedMessage, ReactionStatus, Timeline, TimelineBuilder, TimelineEventFocusThreadMode,
    TimelineEventItemId, TimelineFocus, TimelineItem as SdkTimelineItem,
    TimelineItemContent as SdkTimelineItemContent,
};
use serde::{Deserialize, Serialize};
use tokio::{sync::Mutex as AsyncMutex, task::JoinHandle};

use crate::app::utd_recovery::{UtdRecoveryCoordinator, UtdRecoveryKind, MAX_EVENT_IDS_PER_BATCH};
use crate::dto::TimelineEncryptedUnavailableItem;

use super::{
    project_timeline_diffs_with_media, project_timeline_item_with_media, NativeDecryptionState,
    NativeReactionMutation, NativeReactionMutationResult, NativeTimelineCloseRequest,
    NativeTimelineDirection, NativeTimelineEventReadback, NativeTimelineItem,
    NativeTimelineJumpLatestRequest, NativeTimelineOpenPosition, NativeTimelineOpenReadback,
    NativeTimelineOpenRequest, NativeTimelineReaction, NativeTimelineReactionSender,
    NativeTimelineReadAction, NativeTimelineReadStateReadback, NativeTimelineReadStateRequest,
    NativeTimelineSnapshot, NativeTimelineViewPaginationRequest, NativeTimelineViewportHint,
    NativeUtdPhase, NativeUtdStatus, TimelineMediaRegistry, TimelineMediaSource, TimelinePageState,
    TimelinePaginationState, TimelineReadState, TimelineViewCapabilities, TimelineViewDeltaBatch,
    TimelineViewPosition, TimelineViewSnapshot, TimelineViewUpdateEmit, UtdIndex, UtdPhase,
    UtdReasonCode, ViewDeltaEmitter, NATIVE_TIMELINE_OPEN_SCHEMA_VERSION,
    NATIVE_TIMELINE_VIEWPORT_RESTORE_TTL_MS, TIMELINE_VIEW_SCHEMA_VERSION,
};

const PAGINATION_BATCH_SIZE: u16 = 30;
const REDACTED_PLACEHOLDER: &str = "Message removed";
const UTD_PLACEHOLDER: &str = "Unable to decrypt this message";
const UNSUPPORTED_PLACEHOLDER: &str = "Unsupported event";
const MAX_FOCUSED_EVENT_READBACKS: usize = 256;
const FOCUSED_CONTEXT_EVENT_COUNT: u16 = 25;

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
    registry: tokio::sync::Mutex<NativeTimelineRegistry>,
}

impl NativeTimelineOwner {
    pub fn new(client: &Client, session_generation: u64) -> Self {
        Self {
            client: client.clone(),
            registry: tokio::sync::Mutex::new(NativeTimelineRegistry::new(session_generation)),
        }
    }

    pub async fn lock(&self) -> tokio::sync::MutexGuard<'_, NativeTimelineRegistry> {
        self.registry.lock().await
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
                let now_ms = unix_time_ms();
                let selected_position = resolve_normal_open_position(
                    has_unread,
                    room.fully_read_event_id()
                        .map(|event_id| event_id.to_string()),
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
                    }
                    | TimelineViewPosition::Restored {
                        anchor_event_id: Some(ref anchor_event_id),
                    } => {
                        let event_id = parse_event_id(anchor_event_id)
                            .map_err(|_| "v-timeline-normal-anchor-invalid")?;
                        let key = (room_id_string.clone(), event_id.to_string());
                        if !self.focused_entries.contains_key(&key) {
                            if self.focused_entries.len() >= MAX_FOCUSED_EVENT_READBACKS {
                                if let Some(oldest_key) =
                                    self.focused_entries.keys().next().cloned()
                                {
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
                                .map_err(|_| "v-timeline-normal-open-failed")?;
                            self.focused_entries.insert(key.clone(), Arc::new(timeline));
                        }
                        let timeline = self
                            .focused_entries
                            .get(&key)
                            .expect("normal focused timeline present")
                            .clone();
                        (
                            timeline,
                            selected_position,
                            TimelinePaginationState {
                                backward: TimelinePageState::Available,
                                forward: TimelinePageState::Available,
                            },
                        )
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
        let rows = {
            let mut registry = media.lock().await;
            items
                .iter()
                .map(|item| {
                    project_timeline_item_with_media(item, own_user_id.as_deref(), &mut registry)
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
        // A supported room can be unread without an `m.fully_read` marker, for
        // example a channel this device has never opened or read. With no
        // authoritative native frontier there is no unread anchor to place, so
        // fall back to the live bottom instead of failing the whole room open.
        // The explicit `Unread` position kind remains strict in `open_at`.
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
    } = input;
    tokio::spawn(async move {
        let emitter = ViewDeltaEmitter::new(emit, session_generation, stream_id, room_id, revision);
        let mut last_read_state =
            project_live_read_state(&timeline, &position, own_user_id.as_deref()).await;
        let mut last_pagination =
            pagination_state_from_hit_start(hit_start.load(Ordering::Acquire));
        let mut last_pinned_event_ids = project_pinned_event_ids(timeline.room());

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

        futures_util::pin_mut!(updates);

        loop {
            tokio::select! {
                Some(diffs) = updates.next() => {
                    apply_item_id_diffs(&mut item_ids, &diffs);
                    let ops = {
                        let mut registry = media.lock().await;
                        registry.retain_items(item_ids.iter().map(String::as_str));
                        project_timeline_diffs_with_media(
                            &diffs,
                            own_user_id.as_deref(),
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
                    let read_changed = read_state != last_read_state;
                    let pins_changed = pinned_event_ids != last_pinned_event_ids;
                    if !read_changed && !pins_changed {
                        continue;
                    }
                    if read_changed {
                        last_read_state = read_state.clone();
                    }
                    if pins_changed {
                        last_pinned_event_ids = pinned_event_ids.clone();
                    }
                    emitter.emit(
                        Vec::new(),
                        read_changed.then_some(read_state),
                        None,
                        pins_changed.then_some(pinned_event_ids),
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
    let rows = {
        let mut registry = media.lock().await;
        registry.retain_items(items.iter().map(|item| item.unique_id().0.as_str()));
        items
            .iter()
            .map(|item| {
                project_timeline_item_with_media(item, input.own_user_id.as_deref(), &mut registry)
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
