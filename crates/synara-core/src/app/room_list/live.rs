//! Live D0.2 room-list snapshot projection.
//!
//! SDK room objects and vector diffs stop here. The Tauri boundary receives
//! only ordered room IDs and product-owned, privacy-safe summaries.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use eyeball_im::{Vector, VectorDiff};
use futures_util::StreamExt;
use matrix_sdk::event_handler::EventHandlerHandle;
use matrix_sdk::notification_settings::{
    IsEncrypted, IsOneToOne, NotificationSettings, RoomNotificationMode,
};
use matrix_sdk::ruma::events::{
    AnyGlobalAccountDataEvent, AnyRoomAccountDataEvent, MessageLikeEventContent,
};
use matrix_sdk::ruma::serde::Raw;
use matrix_sdk::ruma::{OwnedRoomId, RoomId};
use matrix_sdk::sync::RoomUpdates;
use matrix_sdk::{Client, EncryptionState, Room, RoomState};
use matrix_sdk_ui::room_list_service::{filters, RoomListItem};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

use crate::app::room_list::counts::{room_unread_presentation, RoomUnreadMembership};
use crate::app::room_list::dm_avatar::dm_avatar_source;
use crate::app::room_list::last_message::{
    last_message_event_is_agent_approval, last_message_event_is_agent_approval_str,
    last_message_preview_from_event_json, last_message_preview_from_event_json_str,
    last_message_preview_from_invite,
};
use crate::app::sync::SyncServiceOwner;
use crate::dto::{Membership, NotificationMode, RoomEncryptionStatus, RoomSummary};

/// Privacy-safe room-list wake-up. No room ids, names, tokens, or password.
/// Consumers re-fetch via the existing snapshot command. `revision` grows by
/// one per coalesced change for this owner; it is not a wall-clock value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeRoomListUpdateSignal {
    pub session_generation: u64,
    pub revision: u64,
}

pub type RoomListUpdateEmit = Arc<dyn Fn(NativeRoomListUpdateSignal) + Send + Sync>;

/// Changes inside this window collapse into one wake-up. The first change
/// emits after at most this delay, so a burst of sync updates costs one
/// snapshot instead of one per room.
pub const ROOM_LIST_UPDATE_DEBOUNCE: Duration = Duration::from_millis(150);

/// Safety net for a change no SDK stream reports. Room-info writes (unread
/// counts, receipts, latest events, names, tags), invites, push rules and
/// account data already wake the owner.
pub const ROOM_LIST_SAFETY_PULSE: Duration = Duration::from_secs(60);

/// Owns one joined-room entries stream for an attached SyncService.
///
/// Wakes consumers only when something they render can have changed:
/// room-list diffs (the SDK turns every room-info notable update into a
/// `Set` diff), invite/leave/knock sync updates, push-rule changes and
/// account data. It also keeps the sliding-sync room subscriptions equal to
/// the joined set, so encrypted rooms keep receiving events without a
/// renderer poll.
pub struct NativeRoomListOwner {
    task: JoinHandle<()>,
    client: Client,
    handlers: Vec<EventHandlerHandle>,
    revision: Arc<AtomicU64>,
}

impl NativeRoomListOwner {
    pub fn start(owner: &Arc<SyncServiceOwner>, emit: RoomListUpdateEmit) -> Self {
        let service = owner.room_list_service();
        let client = service.client().clone();
        let sync_owner = Arc::clone(owner);
        let session_generation = owner.session_generation();
        let revision = Arc::new(AtomicU64::new(0));
        let (wake, wakes) = mpsc::unbounded_channel::<()>();
        let handlers = vec![
            client.add_event_handler({
                let wake = wake.clone();
                move |_: Raw<AnyGlobalAccountDataEvent>| {
                    let _ = wake.send(());
                    async {}
                }
            }),
            client.add_event_handler({
                let wake = wake.clone();
                move |_: Raw<AnyRoomAccountDataEvent>| {
                    let _ = wake.send(());
                    async {}
                }
            }),
        ];
        let room_updates = client.subscribe_to_all_room_updates();
        let task_revision = Arc::clone(&revision);
        let task_client = client.clone();
        let task = tokio::spawn(async move {
            let Ok(list) = service.all_rooms().await else {
                return;
            };
            let (entries, controller) = list.entries_with_dynamic_adapters(usize::MAX);
            if !controller.set_filter(Box::new(filters::new_filter_joined())) {
                return;
            }
            let settings = task_client.notification_settings().await;
            run_room_list_updates(
                entries,
                RoomListWakeSources {
                    wakes,
                    push_rules: settings.subscribe_to_changes(),
                    room_updates,
                },
                |room_ids| {
                    let sync_owner = Arc::clone(&sync_owner);
                    async move { sync_owner.subscribe_to_room_list(&room_ids).await }
                },
                move || {
                    let revision = task_revision.fetch_add(1, Ordering::AcqRel) + 1;
                    emit(NativeRoomListUpdateSignal {
                        session_generation,
                        revision,
                    });
                },
            )
            .await;
            drop(settings);
            drop(controller);
        });
        Self {
            task,
            client,
            handlers,
            revision,
        }
    }

    /// Coalesced changes emitted so far. Zero until the first change.
    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }
}

impl Drop for NativeRoomListOwner {
    fn drop(&mut self) {
        self.task.abort();
        for handle in self.handlers.drain(..) {
            self.client.remove_event_handler(handle);
        }
    }
}

struct RoomListWakeSources {
    wakes: mpsc::UnboundedReceiver<()>,
    push_rules: broadcast::Receiver<()>,
    room_updates: broadcast::Receiver<RoomUpdates>,
}

/// True when a sync update touches rooms outside the joined filter. Joined
/// rooms already arrive as entry diffs.
fn room_updates_change_non_joined(update: &RoomUpdates) -> bool {
    !update.invited.is_empty() || !update.left.is_empty() || !update.knocked.is_empty()
}

/// Apply entry diffs, keep subscriptions equal to the joined set, and emit
/// one debounced wake-up per burst of changes. Returns when the entries
/// stream ends.
async fn run_room_list_updates<S, Subscribe, SubscribeFuture, Emit>(
    entries: S,
    mut sources: RoomListWakeSources,
    subscribe: Subscribe,
    emit: Emit,
) where
    S: futures_util::Stream<Item = Vec<VectorDiff<RoomListItem>>>,
    Subscribe: Fn(Vec<OwnedRoomId>) -> SubscribeFuture,
    SubscribeFuture: std::future::Future<Output = ()>,
    Emit: Fn(),
{
    futures_util::pin_mut!(entries);
    let mut rooms: Vector<RoomListItem> = Vector::new();
    let mut subscribed: HashSet<OwnedRoomId> = HashSet::new();
    let mut debounce = RoomListDebounce::default();
    let mut wakes_open = true;
    let mut push_rules_open = true;
    let mut room_updates_open = true;
    let mut pulse = tokio::time::interval_at(
        tokio::time::Instant::now() + ROOM_LIST_SAFETY_PULSE,
        ROOM_LIST_SAFETY_PULSE,
    );
    pulse.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            diffs = entries.next() => {
                let Some(diffs) = diffs else { break };
                if diffs.is_empty() {
                    continue;
                }
                for diff in diffs {
                    diff.apply(&mut rooms);
                }
                let joined: HashSet<OwnedRoomId> =
                    rooms.iter().map(|room| room.room_id().to_owned()).collect();
                if joined != subscribed {
                    let room_ids = rooms.iter().map(|room| room.room_id().to_owned()).collect();
                    subscribe(room_ids).await;
                    subscribed = joined;
                }
                debounce.mark();
            }
            wake = sources.wakes.recv(), if wakes_open => match wake {
                Some(()) => debounce.mark(),
                None => wakes_open = false,
            },
            changed = sources.push_rules.recv(), if push_rules_open => match changed {
                Ok(()) | Err(broadcast::error::RecvError::Lagged(_)) => debounce.mark(),
                Err(broadcast::error::RecvError::Closed) => push_rules_open = false,
            },
            update = sources.room_updates.recv(), if room_updates_open => match update {
                Ok(update) if room_updates_change_non_joined(&update) => debounce.mark(),
                Ok(_) => {}
                Err(broadcast::error::RecvError::Lagged(_)) => debounce.mark(),
                Err(broadcast::error::RecvError::Closed) => room_updates_open = false,
            },
            () = debounce.elapsed(), if debounce.is_pending() => {
                debounce.clear();
                emit();
            }
            _ = pulse.tick() => debounce.mark(),
        }
    }
}

/// Leading-edge debounce: the first change starts the window and later ones
/// in that window join it.
#[derive(Default)]
struct RoomListDebounce {
    deadline: Option<tokio::time::Instant>,
}

impl RoomListDebounce {
    fn mark(&mut self) {
        if self.deadline.is_none() {
            self.deadline = Some(tokio::time::Instant::now() + ROOM_LIST_UPDATE_DEBOUNCE);
        }
    }

    fn is_pending(&self) -> bool {
        self.deadline.is_some()
    }

    fn clear(&mut self) {
        self.deadline = None;
    }

    async fn elapsed(&self) {
        match self.deadline {
            Some(deadline) => tokio::time::sleep_until(deadline).await,
            None => std::future::pending().await,
        }
    }
}

/// Retry window for a room whose encryption state could not be learned.
/// `latest_encryption_state` sends `GET /state/m.room.encryption` whenever the
/// state is unknown, so without a throttle every snapshot repeated that
/// request for each such room.
const ENCRYPTION_PROBE_RETRY: Duration = Duration::from_secs(300);
const ENCRYPTION_PROBE_MAX_TRACKED: usize = 4096;

fn encryption_probes() -> &'static Mutex<HashMap<OwnedRoomId, Instant>> {
    static PROBES: OnceLock<Mutex<HashMap<OwnedRoomId, Instant>>> = OnceLock::new();
    PROBES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn encryption_probe_allowed(room_id: &RoomId, now: Instant) -> bool {
    let Ok(probes) = encryption_probes().lock() else {
        return false;
    };
    probes
        .get(room_id)
        .is_none_or(|last| now.saturating_duration_since(*last) >= ENCRYPTION_PROBE_RETRY)
}

fn record_encryption_probe(room_id: &RoomId, resolved: bool, now: Instant) {
    let Ok(mut probes) = encryption_probes().lock() else {
        return;
    };
    if resolved {
        probes.remove(room_id);
        return;
    }
    if probes.len() >= ENCRYPTION_PROBE_MAX_TRACKED {
        probes.retain(|_, last| now.saturating_duration_since(*last) < ENCRYPTION_PROBE_RETRY);
        if probes.len() >= ENCRYPTION_PROBE_MAX_TRACKED {
            probes.clear();
        }
    }
    probes.insert(room_id.to_owned(), now);
}

/// Cached encryption state, probing the homeserver at most once per
/// [`ENCRYPTION_PROBE_RETRY`] while it stays unknown.
async fn room_encryption_state(room: &Room) -> matrix_sdk::Result<EncryptionState> {
    let cached = room.encryption_state();
    if !cached.is_unknown() {
        return Ok(cached);
    }
    let now = Instant::now();
    if !encryption_probe_allowed(room.room_id(), now) {
        return Ok(cached);
    }
    let result = room.latest_encryption_state().await;
    let resolved = result.as_ref().is_ok_and(|state| !state.is_unknown());
    record_encryption_probe(room.room_id(), resolved, now);
    result
}

const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomListSnapshot {
    pub session_generation: u64,
    pub ordered_room_ids: Vec<String>,
    pub rooms: Vec<RoomSummary>,
}

pub async fn snapshot_from_sync_owner(
    owner: &SyncServiceOwner,
) -> Result<NativeRoomListSnapshot, &'static str> {
    let service = owner.room_list_service();
    let list = service
        .all_rooms()
        .await
        .map_err(|_| "d0.2-room-list-open-failed")?;
    let (entries, controller) = list.entries_with_dynamic_adapters(usize::MAX);
    if !controller.set_filter(Box::new(filters::new_filter_joined())) {
        return Err("d0.2-room-list-filter-failed");
    }

    futures_util::pin_mut!(entries);
    let diffs = tokio::time::timeout(SNAPSHOT_TIMEOUT, entries.next())
        .await
        .map_err(|_| "d0.2-room-list-snapshot-timeout")?
        .ok_or("d0.2-room-list-stream-ended")?;
    let values = diffs
        .into_iter()
        .find_map(|diff| match diff {
            VectorDiff::Reset { values } => Some(values),
            _ => None,
        })
        .ok_or("d0.2-room-list-reset-missing")?;

    // Sliding-sync `set_room_subscriptions` makes the subscription set
    // exactly these rooms and is how encrypted rooms receive events for
    // client-side unread math. A 20-row viewport left every other room at
    // server notification_count=0, which is typically zero for E2EE —
    // Element (full /sync) still showed badges.
    let subscribed_room_ids = values
        .iter()
        .map(|room| room.room_id().to_owned())
        .collect::<Vec<_>>();
    owner.subscribe_to_room_list(&subscribed_room_ids).await;

    // One push-rule read per snapshot. `Room::notification_mode` builds a new
    // `NotificationSettings` (a store read plus an event handler) per room.
    let settings = service.client().notification_settings().await;
    let mut ordered_room_ids = Vec::with_capacity(values.len());
    let mut rooms = Vec::with_capacity(values.len());
    for item in values {
        ordered_room_ids.push(item.room_id().to_string());
        rooms.push(project_room(&item, &settings).await);
    }

    Ok(NativeRoomListSnapshot {
        session_generation: owner.session_generation(),
        ordered_room_ids,
        rooms,
    })
}

async fn project_room(room: &Room, settings: &NotificationSettings) -> RoomSummary {
    let counts = room.unread_notification_counts();
    // `recency_stamp` is an opaque ordering value, not wall-clock time. Only
    // expose a timestamp when the SDK has an actual latest-event timestamp.
    let last_activity_ts = room
        .latest_event_timestamp()
        .map(|timestamp| timestamp.get().into());
    // Same answer as `Room::notification_mode`, without its per-room HTTP
    // encryption probe or push-rule reload.
    let latest_encryption = room_encryption_state(room).await;
    let notification_mode = room_notification_mode(room, settings, &latest_encryption)
        .await
        .map(map_notification_mode);
    let membership = membership(room.state());
    let last_message_preview = last_message_preview(room);
    // Classification uses the raw latest event, not the sanitized preview.
    let last_message_is_approval = last_message_is_agent_approval(room);
    // Approval prompts only promote unread/highlight while the SDK still
    // reports unread. Reading the room does not change the latest body, so
    // boosting from the last message alone would recreate badges after every
    // receipt that already zeroed the counters. Marked-unread is unread.
    let has_unread = room_has_unread(
        room.num_unread_messages(),
        room.num_unread_notifications(),
        room.num_unread_mentions(),
        counts.notification_count,
        counts.highlight_count,
        room.is_marked_unread(),
    );
    let pending_approval = pending_approval_unread_boost(has_unread, last_message_is_approval);
    let mention_count = room
        .num_unread_mentions()
        .max(counts.highlight_count)
        .max(u64::from(pending_approval));
    let unread = room_unread_presentation(
        match membership {
            Membership::Invite => RoomUnreadMembership::Invited,
            _ => RoomUnreadMembership::Joined,
        },
        room.num_unread_messages(),
        room.num_unread_notifications()
            .max(counts.notification_count)
            .max(u64::from(pending_approval)),
        mention_count,
        room.is_marked_unread(),
    );
    // Room derefs to `BaseRoom`: `is_favourite`/`is_low_priority` read cached
    // `notable_tags` derived from the room's m.tag account data.
    let state_encrypted = latest_encryption
        .as_ref()
        .is_ok_and(|state| state.is_state_encrypted());
    let encryption_status = project_encryption_status(latest_encryption);
    let is_direct = room.is_direct().await.unwrap_or(false);
    let avatar_url = {
        let room_avatar = room.avatar_url();
        if is_direct && room_avatar.is_none() {
            dm_avatar_source(room, room.own_user_id(), true)
                .await
                .map(|uri| uri.to_string())
        } else {
            room_avatar.map(|uri| uri.to_string())
        }
    };
    let (has_active_call, active_call_participant_count) = project_active_call(room);
    RoomSummary {
        room_id: room.room_id().to_string(),
        name: room.cached_display_name().map(|name| name.to_string()),
        canonical_alias: room.canonical_alias().map(|alias| alias.to_string()),
        avatar_url,
        membership,
        is_direct,
        direct_user_id: project_direct_user_id(room, is_direct),
        is_space: room.is_space(),
        is_call: room.is_call(),
        has_active_call,
        active_call_participant_count,
        is_favorite: room.is_favourite(),
        is_low_priority: room.is_low_priority(),
        folder_id: None,
        encryption_status,
        state_encrypted,
        join_rule: None,
        unread_count: bounded_count(unread.unread_count),
        highlight_count: bounded_count(mention_count),
        marked_unread: room.is_marked_unread(),
        notification_mode,
        last_activity_ts,
        last_message_preview,
        last_message_is_agent_approval: last_message_is_approval,
        heroes: None,
        tombstone_successor_room_id: None,
    }
}

async fn room_notification_mode(
    room: &Room,
    settings: &NotificationSettings,
    encryption: &matrix_sdk::Result<EncryptionState>,
) -> Option<RoomNotificationMode> {
    if !matches!(room.state(), RoomState::Joined) {
        return None;
    }
    if let Some(mode) = room.cached_user_defined_notification_mode() {
        return Some(mode);
    }
    if let Some(mode) = settings
        .get_user_defined_room_notification_mode(room.room_id())
        .await
    {
        return Some(mode);
    }
    let Ok(state) = encryption else {
        return None;
    };
    // From the point of view of notification settings, a one-to-one room
    // involves exactly two people.
    let is_one_to_one = IsOneToOne::from(room.active_members_count() == 2);
    Some(
        settings
            .get_default_room_notification_mode(
                IsEncrypted::from(state.is_encrypted()),
                is_one_to_one,
            )
            .await,
    )
}

const MAX_ACTIVE_CALL_PARTICIPANTS: u32 = 99;

fn project_direct_user_id(room: &Room, is_direct: bool) -> Option<String> {
    if !is_direct {
        return None;
    }
    let client = room.client();
    let own = client.user_id();
    let mut peer: Option<String> = None;
    for target in room.direct_targets() {
        let Some(user_id) = target.as_user_id() else {
            continue;
        };
        if own.is_some_and(|own| own == user_id) {
            continue;
        }
        let next = user_id.to_string();
        if peer.as_ref().is_some_and(|existing| existing != &next) {
            return None;
        }
        peer = Some(next);
    }
    peer
}

fn project_active_call(room: &Room) -> (bool, u32) {
    if !room.has_active_room_call() {
        return (false, 0);
    }
    let mut unique = std::collections::HashSet::new();
    for participant in room.active_room_call_participants() {
        unique.insert(participant);
    }
    (
        true,
        unique.len().min(MAX_ACTIVE_CALL_PARTICIPANTS as usize) as u32,
    )
}

fn project_encryption_status<E>(result: Result<EncryptionState, E>) -> RoomEncryptionStatus {
    match result {
        Ok(state) if state.is_unknown() => RoomEncryptionStatus::Unknown,
        Ok(state) if state.is_encrypted() => RoomEncryptionStatus::Encrypted,
        Ok(_) => RoomEncryptionStatus::NotEncrypted,
        Err(_) => RoomEncryptionStatus::Unknown,
    }
}

fn last_message_preview(room: &Room) -> Option<String> {
    use matrix_sdk::latest_events::LatestEventValue;
    match room.latest_event() {
        LatestEventValue::None => None,
        LatestEventValue::RemoteInvite { inviter, .. } => {
            last_message_preview_from_invite(inviter.as_ref().map(|id| id.as_str()))
        }
        LatestEventValue::Remote(event) => {
            last_message_preview_from_event_json_str(event.raw().json().get())
        }
        LatestEventValue::LocalIsSending(local)
        | LatestEventValue::LocalHasBeenSent { value: local, .. }
        | LatestEventValue::LocalCannotBeSent(local) => {
            let Ok(content) = local.content.deserialize() else {
                return None;
            };
            last_message_preview_from_event_json(&serde_json::json!({
                "type": content.event_type().to_string(),
                "content": content,
            }))
        }
    }
}

fn room_has_unread(
    unread_messages: u64,
    unread_notifications: u64,
    unread_mentions: u64,
    notification_count: u64,
    highlight_count: u64,
    is_marked_unread: bool,
) -> bool {
    unread_messages > 0
        || unread_notifications > 0
        || unread_mentions > 0
        || notification_count > 0
        || highlight_count > 0
        || is_marked_unread
}

/// Last-message approval must not manufacture unread after receipts clear.
fn pending_approval_unread_boost(has_unread: bool, last_message_is_approval: bool) -> bool {
    has_unread && last_message_is_approval
}

fn last_message_is_agent_approval(room: &Room) -> bool {
    use matrix_sdk::latest_events::LatestEventValue;
    match room.latest_event() {
        LatestEventValue::Remote(event) => {
            last_message_event_is_agent_approval_str(event.raw().json().get())
        }
        LatestEventValue::LocalIsSending(local)
        | LatestEventValue::LocalHasBeenSent { value: local, .. }
        | LatestEventValue::LocalCannotBeSent(local) => {
            let Ok(content) = local.content.deserialize() else {
                return false;
            };
            last_message_event_is_agent_approval(&serde_json::json!({
                "type": content.event_type().to_string(),
                "content": content,
            }))
        }
        LatestEventValue::None | LatestEventValue::RemoteInvite { .. } => false,
    }
}

fn map_notification_mode(mode: RoomNotificationMode) -> NotificationMode {
    match mode {
        RoomNotificationMode::AllMessages => NotificationMode::All,
        RoomNotificationMode::MentionsAndKeywordsOnly => NotificationMode::Mentions,
        RoomNotificationMode::Mute => NotificationMode::Mute,
    }
}

fn membership(state: RoomState) -> Membership {
    match state {
        RoomState::Invited => Membership::Invite,
        RoomState::Joined => Membership::Join,
        RoomState::Knocked => Membership::Knock,
        RoomState::Left => Membership::Leave,
        RoomState::Banned => Membership::Ban,
    }
}

fn bounded_count(value: u64) -> u32 {
    value.min(u32::MAX.into()) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_states_map_to_product_memberships() {
        assert_eq!(membership(RoomState::Joined), Membership::Join);
        assert_eq!(membership(RoomState::Invited), Membership::Invite);
        assert_eq!(membership(RoomState::Knocked), Membership::Knock);
        assert_eq!(membership(RoomState::Left), Membership::Leave);
        assert_eq!(membership(RoomState::Banned), Membership::Ban);
    }

    #[test]
    fn notification_modes_map_to_product_dto() {
        assert_eq!(
            map_notification_mode(RoomNotificationMode::AllMessages),
            NotificationMode::All
        );
        assert_eq!(
            map_notification_mode(RoomNotificationMode::MentionsAndKeywordsOnly),
            NotificationMode::Mentions
        );
        assert_eq!(
            map_notification_mode(RoomNotificationMode::Mute),
            NotificationMode::Mute
        );
    }

    #[test]
    fn unread_counts_are_bounded_for_ipc() {
        assert_eq!(bounded_count(7), 7);
        assert_eq!(bounded_count(u64::MAX), u32::MAX);
    }

    #[test]
    fn pending_approval_does_not_restore_unread_after_receipts_clear_counts() {
        // Latest event can still be the approval prompt after a read receipt
        // zeros every SDK counter; the boost must stay off so badges clear.
        assert!(!pending_approval_unread_boost(false, true));
        assert!(pending_approval_unread_boost(true, true));
        assert!(!pending_approval_unread_boost(true, false));
        assert!(!pending_approval_unread_boost(false, false));
    }

    #[test]
    fn marked_unread_counts_as_unread_for_approval_boost() {
        assert!(!room_has_unread(0, 0, 0, 0, 0, false));
        assert!(room_has_unread(0, 0, 0, 0, 0, true));
        assert!(room_has_unread(1, 0, 0, 0, 0, false));
        assert!(pending_approval_unread_boost(
            room_has_unread(0, 0, 0, 0, 0, true),
            true
        ));
        assert!(!pending_approval_unread_boost(
            room_has_unread(0, 0, 0, 0, 0, false),
            true
        ));
    }

    #[test]
    fn last_message_preview_matches_local_send_queue_states() {
        let source = include_str!("live.rs");
        assert!(source.contains("LatestEventValue::LocalIsSending"));
        assert!(source.contains("LatestEventValue::LocalHasBeenSent"));
        assert!(source.contains("LatestEventValue::LocalCannotBeSent"));
        assert!(source.contains("LatestEventValue::Remote("));
    }

    #[test]
    fn encryption_projection_preserves_unknown_and_errors_fail_closed() {
        assert_eq!(
            project_encryption_status::<()>(Ok(EncryptionState::Encrypted)),
            RoomEncryptionStatus::Encrypted
        );
        assert_eq!(
            project_encryption_status::<()>(Ok(EncryptionState::NotEncrypted)),
            RoomEncryptionStatus::NotEncrypted
        );
        assert_eq!(
            project_encryption_status::<()>(Ok(EncryptionState::Unknown)),
            RoomEncryptionStatus::Unknown
        );
        assert_eq!(
            project_encryption_status::<()>(Err(())),
            RoomEncryptionStatus::Unknown
        );
        assert_eq!(
            project_encryption_status::<()>(Ok(EncryptionState::StateEncrypted)),
            RoomEncryptionStatus::Encrypted
        );
        assert!(EncryptionState::StateEncrypted.is_encrypted());
        assert!(EncryptionState::StateEncrypted.is_state_encrypted());
        assert!(!EncryptionState::Encrypted.is_state_encrypted());
    }

    #[test]
    fn encryption_probe_is_throttled_until_the_state_resolves() {
        let room_id: OwnedRoomId = "!probe-throttle:example.org".try_into().unwrap();
        let now = Instant::now();
        assert!(encryption_probe_allowed(&room_id, now));
        record_encryption_probe(&room_id, false, now);
        assert!(!encryption_probe_allowed(&room_id, now));
        assert!(!encryption_probe_allowed(
            &room_id,
            now + ENCRYPTION_PROBE_RETRY - Duration::from_secs(1)
        ));
        assert!(encryption_probe_allowed(
            &room_id,
            now + ENCRYPTION_PROBE_RETRY
        ));
        record_encryption_probe(&room_id, true, now);
        assert!(encryption_probe_allowed(&room_id, now));
    }

    #[test]
    fn only_non_joined_sync_updates_wake_the_owner() {
        let mut update = RoomUpdates::default();
        assert!(!room_updates_change_non_joined(&update));
        update.joined.insert(
            "!joined:example.org".try_into().unwrap(),
            Default::default(),
        );
        assert!(
            !room_updates_change_non_joined(&update),
            "joined rooms arrive as entry diffs"
        );
        update.invited.insert(
            "!invite:example.org".try_into().unwrap(),
            Default::default(),
        );
        assert!(room_updates_change_non_joined(&update));
    }

    #[tokio::test]
    async fn debounce_collapses_a_burst_into_one_emit() {
        let mut debounce = RoomListDebounce::default();
        assert!(!debounce.is_pending());
        let started = tokio::time::Instant::now();
        debounce.mark();
        let first_deadline = debounce.deadline;
        tokio::time::sleep(Duration::from_millis(50)).await;
        debounce.mark();
        assert_eq!(
            debounce.deadline, first_deadline,
            "later marks join the window"
        );
        debounce.elapsed().await;
        let waited = started.elapsed();
        assert!(waited >= ROOM_LIST_UPDATE_DEBOUNCE);
        assert!(waited < ROOM_LIST_UPDATE_DEBOUNCE + Duration::from_millis(100));
        debounce.clear();
        assert!(!debounce.is_pending());
    }

    #[test]
    fn room_list_subscriptions_cover_the_full_joined_snapshot() {
        let source = include_str!("live.rs");
        assert!(source.contains("subscribed_room_ids"));
        assert!(source.contains("values.iter().map(|room| room.room_id().to_owned())"));
        assert!(source.contains("MissedTickBehavior::Skip"));
        assert!(source.contains("last_message_is_agent_approval"));
        assert!(source.contains("last_message_is_agent_approval: last_message_is_approval"));
        assert!(source.contains("pending_approval_unread_boost"));
        assert!(source.contains("room_has_unread("));
        assert!(source.contains("room.is_marked_unread()"));
        assert!(source.contains("dm_avatar_source"));
        assert!(source.contains("is_direct && room_avatar.is_none()"));
        let truncated_viewport = concat!("ROOM_LIST_SUBSCRIPTION", "_LIMIT");
        assert_eq!(
            source.matches(truncated_viewport).count(),
            0,
            "encrypted rooms only get client-side unreads after set_room_subscriptions"
        );
    }
}

#[cfg(test)]
mod live_owner_tests {
    use super::*;
    use crate::app::sync::{build_sync_service, SyncServiceConfig};
    use matrix_sdk::test_utils::mocks::MatrixMockServer;
    use wiremock::matchers::{body_partial_json, method, path, path_regex};
    use wiremock::{Mock, ResponseTemplate};

    const SLIDING_SYNC: &str = "/_matrix/client/unstable/org.matrix.simplified_msc3575/sync";
    const ROOM: &str = "!live-owner:example.org";

    fn room_list_response(pos: &str, name: &str) -> serde_json::Value {
        serde_json::json!({
            "pos": pos,
            "lists": { "all_rooms": { "count": 1 } },
            "rooms": { ROOM: { "name": name, "initial": true, "timeline": [] } },
            "extensions": {}
        })
    }

    async fn mount_room_list_response(server: &MatrixMockServer, pos: &str, name: &str) {
        Mock::given(method("POST"))
            .and(path(SLIDING_SYNC))
            .and(body_partial_json(
                serde_json::json!({"conn_id": "room-list"}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(room_list_response(pos, name)))
            .up_to_n_times(1)
            .with_priority(1)
            .mount(server.server())
            .await;
    }

    async fn requests_matching(
        server: &MatrixMockServer,
        matches: impl Fn(&wiremock::Request) -> bool,
    ) -> usize {
        server
            .server()
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .filter(|request| matches(request))
            .count()
    }

    fn subscribes_room(request: &wiremock::Request) -> bool {
        request.url.path() == SLIDING_SYNC
            && request
                .body_json::<serde_json::Value>()
                .ok()
                .and_then(|body| body.get("room_subscriptions").cloned())
                .is_some_and(|subscriptions| subscriptions.get(ROOM).is_some())
    }

    #[tokio::test]
    async fn owner_emits_on_room_changes_and_stays_quiet_while_idle() {
        let server = MatrixMockServer::new().await;
        let client = server.client_builder().build().await;
        Mock::given(method("GET"))
            .and(path("/_matrix/client/versions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "versions": ["v1.12"],
                "unstable_features": { "org.matrix.simplified_msc3575": true }
            })))
            .mount(server.server())
            .await;
        server.mock_upload_keys().ok().mount().await;
        Mock::given(method("GET"))
            .and(path_regex(r"/state/m\.room\.encryption"))
            .respond_with(ResponseTemplate::new(500))
            .mount(server.server())
            .await;
        mount_room_list_response(&server, "1", "Room A").await;
        // Idle long-poll: nothing about the room changes.
        Mock::given(method("POST"))
            .and(path(SLIDING_SYNC))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({
                        "pos": "idle", "lists": {}, "rooms": {}, "extensions": {}
                    }))
                    .set_delay(Duration::from_millis(100)),
            )
            .mount(server.server())
            .await;

        let owner = Arc::new(
            build_sync_service(&client, 4, SyncServiceConfig::default())
                .await
                .expect("sync service"),
        );
        let signals = Arc::new(Mutex::new(Vec::<NativeRoomListUpdateSignal>::new()));
        let sink = Arc::clone(&signals);
        let live = NativeRoomListOwner::start(
            &owner,
            Arc::new(move |signal| sink.lock().unwrap().push(signal)),
        );
        owner.start().await.expect("sync starts");

        let wait_for = |count: usize| {
            let signals = Arc::clone(&signals);
            async move {
                tokio::time::timeout(Duration::from_secs(5), async {
                    while signals.lock().unwrap().len() < count {
                        tokio::time::sleep(Duration::from_millis(20)).await;
                    }
                })
                .await
                .is_ok()
            }
        };
        assert!(wait_for(1).await, "the joined room wakes the owner");
        // The owner keeps sliding-sync subscriptions equal to the joined set
        // without a snapshot poll.
        tokio::time::timeout(Duration::from_secs(5), async {
            while requests_matching(&server, subscribes_room).await == 0 {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("the owner subscribes the joined room");

        // Two snapshots with an unknown encryption state probe it once.
        snapshot_from_sync_owner(&owner).await.expect("snapshot");
        snapshot_from_sync_owner(&owner).await.expect("snapshot");
        let probes = requests_matching(&server, |request| {
            request.url.path().contains("/state/m.room.encryption")
        })
        .await;
        assert!(probes <= 1, "encryption probes are throttled, saw {probes}");

        // Let the initial burst settle, then idle across what used to be the
        // unconditional 2 s pulse.
        tokio::time::sleep(Duration::from_millis(500)).await;
        let settled = signals.lock().unwrap().len();
        tokio::time::sleep(Duration::from_millis(2500)).await;
        assert_eq!(
            signals.lock().unwrap().len(),
            settled,
            "an idle room list must not wake consumers"
        );

        mount_room_list_response(&server, "2", "Room A renamed").await;
        assert!(wait_for(settled + 1).await, "a room change wakes the owner");
        let signals = signals.lock().unwrap().clone();
        assert!(signals.iter().all(|signal| signal.session_generation == 4));
        assert!(
            signals
                .windows(2)
                .all(|pair| pair[1].revision == pair[0].revision + 1),
            "revisions are consecutive"
        );
        assert_eq!(live.revision(), signals.last().unwrap().revision);
        drop(live);
        owner.stop().await.expect("sync stops");
    }
}
