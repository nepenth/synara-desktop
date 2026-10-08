//! Core→renderer notification observation stream (A9 follow-on).
//!
//! The renderer used to discover notifiable events by scanning js-sdk style
//! room timelines on every sync tick. The native client facade never emits
//! `Room.timeline`, never reports `SYNCING`, and stubs live timelines to `[]`,
//! so that pump was dead in the shipped product. This owner replaces it: Core
//! observes every message-like timeline event the SDK sync delivers and pushes
//! one bounded observation per candidate event to the shell sink. The renderer
//! submits that identity back through `matrix_notification_decide`, so the
//! policy owner is unchanged — the SDK push rules inside
//! [`super::NativeNotificationDecisionOwner`] still decide.
//!
//! Filtering here is observation hygiene, not policy:
//! - only `m.room.message`, `m.sticker`, and undecryptable `m.room.encrypted`
//!   events (the same set the renderer already treated as notifiable);
//! - redacted events and `m.replace` edits are skipped (Core's decide would
//!   also suppress them through the account's suppress-edits override, but a
//!   skipped observation saves an IPC round trip);
//! - own-sender events are skipped (Core's decide also refuses them);
//! - events older than the recency window are skipped so a fresh login's
//!   initial sync does not replay history into the tray.
//!
//! The observation carries identity (`room_id`, `event_id`, `sender`), the
//! event type, its origin timestamp, and Core approval classification. Raw
//! prompt bodies remain inside Core. It carries no ciphertext, keys, tokens, or push
//! verdicts.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures_util::{Stream, StreamExt};
use matrix_sdk::config::RequestConfig;
use matrix_sdk::event_cache::RedecryptorReport;
use matrix_sdk::event_handler::EventHandlerDropGuard;
use matrix_sdk::ruma::events::room::encrypted::Relation as EncryptedRelation;
use matrix_sdk::ruma::events::room::message::Relation as MessageRelation;
use matrix_sdk::ruma::events::{
    AnySyncMessageLikeEvent, AnySyncTimelineEvent, MessageLikeEventType, SyncMessageLikeEvent,
};
use matrix_sdk::ruma::{OwnedUserId, UserId};
use matrix_sdk::{Client, Room};
use serde::{Deserialize, Serialize};

/// Tauri event / UniFFI callback name carrying [`NativeNotificationObservation`].
pub const NOTIFICATION_OBSERVED_EVENT: &str = "matrix-notification-observed";

/// Only events this recent are observed; older ones are history replayed by
/// an initial or catch-up sync and never notify.
pub const NOTIFICATION_OBSERVATION_WINDOW_MS: u64 = 5 * 60 * 1000;

/// One observed candidate event: identity/facts and Core approval classification.
/// Push delivery verdicts remain private to the decision owner. Approval
/// classification does not assert terminal reaction state; the action owner
/// revalidates current SDK reaction aggregation before sending any reaction.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeNotificationObservation {
    pub session_generation: u64,
    pub room_id: String,
    pub event_id: String,
    pub sender: String,
    pub event_type: String,
    pub origin_server_ts: u64,
    /// Classification from the complete SDK plaintext body, never truncated renderer input.
    pub agent_approval: Option<NativeAgentApprovalObservation>,
}

/// Core classification; delayed consumers must revalidate through decide_observed.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeAgentApprovalObservation {
    pub expires_at: u64,
    pub expired: bool,
}

/// Shell-supplied sink. Desktop maps this to a Tauri event; iOS can map it
/// to a UniFFI callback later.
pub type NotificationObservationEmit = Arc<dyn Fn(NativeNotificationObservation) + Send + Sync>;

/// Owns the SDK message-like event stream for one authenticated session.
/// Dropping the owner removes the SDK handler.
pub struct NativeNotificationObservationOwner {
    session_generation: u64,
    retired: Arc<AtomicBool>,
    follow_ups: Arc<Mutex<Vec<tokio::task::AbortHandle>>>,
    /// Undecryptable candidates waiting for late room keys.
    pending_utds: Arc<PendingUtds<PendingCandidate>>,
    /// Consumes the event cache's decryption reports; aborted on retire.
    reports_task: Option<tokio::task::AbortHandle>,
    _handler: EventHandlerDropGuard,
}

impl NativeNotificationObservationOwner {
    pub fn start(
        client: &Client,
        emit: NotificationObservationEmit,
        session_generation: u64,
    ) -> Result<Self, &'static str> {
        let own_user_id = client
            .user_id()
            .ok_or("v-notify.observation-no-session")?
            .to_owned();
        let retired = Arc::new(AtomicBool::new(false));
        let retired_for_handler = retired.clone();
        let follow_ups = Arc::new(Mutex::new(Vec::new()));
        let follow_ups_for_handler = follow_ups.clone();
        let pending_utds = Arc::new(PendingUtds::default());
        let pending_for_handler = pending_utds.clone();
        let reports_task = tokio::runtime::Handle::try_current().ok().map(|runtime| {
            let client = client.clone();
            let context = LateResolutionContext {
                pending: pending_utds.clone(),
                own_user_id: own_user_id.clone(),
                emit: emit.clone(),
                retired: retired.clone(),
                session_generation,
            };
            runtime
                .spawn(async move {
                    // The stream borrows the event cache, so it lives with
                    // the client clone inside this task.
                    let reports = client.event_cache().subscribe_to_decryption_reports();
                    run_decryption_reports(reports, context).await;
                })
                .abort_handle()
        });
        let handler =
            client.add_event_handler(move |event: AnySyncMessageLikeEvent, room: Room| {
                let retired = retired_for_handler.clone();
                let follow_ups = follow_ups_for_handler.clone();
                let pending = pending_for_handler.clone();
                let emit = emit.clone();
                let own_user_id = own_user_id.clone();
                async move {
                    if retired.load(Ordering::Acquire) {
                        return;
                    }
                    let room_id = room.room_id().to_string();
                    let observation = project_observation(
                        &event,
                        &room_id,
                        &own_user_id,
                        now_ms(),
                        session_generation,
                    );
                    if needs_decryption_follow_up(&event) {
                        // Ciphertext cannot establish approval classification. Do
                        // not consume a notification/dedup slot before resolving it.
                        if observation.is_none() {
                            return;
                        }
                        let retired_for_task = retired.clone();
                        let task = tokio::spawn(async move {
                            follow_up_encrypted_observation(
                                room,
                                event,
                                own_user_id,
                                emit,
                                retired_for_task,
                                pending,
                                session_generation,
                            )
                            .await;
                        });
                        track_follow_up(&follow_ups, &retired, task.abort_handle());
                    } else if let Some(observation) = observation {
                        emit(observation);
                    }
                }
            });
        Ok(Self {
            session_generation,
            retired,
            follow_ups,
            pending_utds,
            reports_task,
            _handler: client.event_handler_drop_guard(handler),
        })
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    /// Stop emitting before the SDK handler is dropped (logout / account
    /// switch). Observations for a retired generation are never delivered.
    pub fn retire(&self) {
        self.retired.store(true, Ordering::Release);
        if let Some(task) = &self.reports_task {
            task.abort();
        }
        self.pending_utds.clear();
        for task in self
            .follow_ups
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .drain(..)
        {
            task.abort();
        }
    }
}

impl Drop for NativeNotificationObservationOwner {
    fn drop(&mut self) {
        self.retire();
    }
}

fn track_follow_up(
    tasks: &Mutex<Vec<tokio::task::AbortHandle>>,
    retired: &AtomicBool,
    task: tokio::task::AbortHandle,
) {
    let mut tasks = tasks.lock().unwrap_or_else(|p| p.into_inner());
    if retired.load(Ordering::Acquire) {
        task.abort();
        return;
    }
    tasks.retain(|task| !task.is_finished());
    tasks.push(task);
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Quick retries for keys that arrive with or right after the event. Keys that
/// arrive later are handled by the event cache's decryption reports.
const DECRYPT_FOLLOW_UP_DELAYS_MS: [u64; 5] = [150, 300, 500, 800, 1_200];

/// Undecryptable candidates kept for late room keys. Anything older than the
/// observation window would be rejected by [`project_observation`] anyway.
const PENDING_UTD_LIMIT: usize = 128;
const PENDING_UTD_TTL: Duration = Duration::from_millis(NOTIFICATION_OBSERVATION_WINDOW_MS);

/// One undecryptable event waiting for its key.
#[derive(Clone)]
struct PendingCandidate {
    room: Room,
    original_event: AnySyncMessageLikeEvent,
}

struct PendingUtd<T> {
    room_id: String,
    event_id: String,
    registered: Instant,
    value: T,
}

/// Bounded, time-limited set of undecryptable candidates keyed by room and
/// event id. Oldest entries are dropped first when the limit is reached.
pub(crate) struct PendingUtds<T> {
    entries: Mutex<VecDeque<PendingUtd<T>>>,
}

impl<T> Default for PendingUtds<T> {
    fn default() -> Self {
        Self {
            entries: Mutex::new(VecDeque::new()),
        }
    }
}

impl<T> PendingUtds<T> {
    fn lock(&self) -> std::sync::MutexGuard<'_, VecDeque<PendingUtd<T>>> {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn purge(entries: &mut VecDeque<PendingUtd<T>>, now: Instant) {
        entries.retain(|entry| now.saturating_duration_since(entry.registered) <= PENDING_UTD_TTL);
    }

    /// Register a candidate. `registered` is when it was first seen, so a
    /// re-registered candidate keeps its original deadline.
    pub(crate) fn insert(
        &self,
        room_id: &str,
        event_id: &str,
        registered: Instant,
        value: T,
        now: Instant,
    ) {
        let mut entries = self.lock();
        Self::purge(&mut entries, now);
        entries.retain(|entry| !(entry.room_id == room_id && entry.event_id == event_id));
        if now.saturating_duration_since(registered) > PENDING_UTD_TTL {
            return;
        }
        while entries.len() >= PENDING_UTD_LIMIT {
            entries.pop_front();
        }
        entries.push_back(PendingUtd {
            room_id: room_id.to_owned(),
            event_id: event_id.to_owned(),
            registered,
            value,
        });
    }

    /// Remove and return the candidates in `room_id` whose event id matches.
    pub(crate) fn take_matching(
        &self,
        room_id: &str,
        resolved: impl Fn(&str) -> bool,
        now: Instant,
    ) -> Vec<(Instant, T)> {
        let mut entries = self.lock();
        Self::purge(&mut entries, now);
        let (taken, kept): (VecDeque<_>, VecDeque<_>) = entries
            .drain(..)
            .partition(|entry| entry.room_id == room_id && resolved(&entry.event_id));
        *entries = kept;
        taken
            .into_iter()
            .map(|entry| (entry.registered, entry.value))
            .collect()
    }

    /// Remove and return every live candidate (the SDK may have missed keys).
    pub(crate) fn take_all(&self, now: Instant) -> Vec<(Instant, T)> {
        let mut entries = self.lock();
        Self::purge(&mut entries, now);
        entries
            .drain(..)
            .map(|entry| (entry.registered, entry.value))
            .collect()
    }

    pub(crate) fn clear(&self) {
        self.lock().clear();
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.lock().len()
    }
}

/// What the event cache reported, reduced to what this owner acts on.
#[derive(Debug, Clone, PartialEq, Eq)]
enum DecryptionReport {
    /// These events of one room now decrypt.
    Resolved {
        room_id: String,
        event_ids: Vec<String>,
    },
    /// Keys may have been missed, or a backup became available: re-check all.
    Recheck,
}

impl DecryptionReport {
    fn from_sdk<E>(report: Result<RedecryptorReport, E>) -> Self {
        match report {
            Ok(RedecryptorReport::ResolvedUtds { room_id, events }) => Self::Resolved {
                room_id: room_id.to_string(),
                event_ids: events.iter().map(ToString::to_string).collect(),
            },
            Ok(RedecryptorReport::Lagging | RedecryptorReport::BackupAvailable) | Err(_) => {
                Self::Recheck
            }
        }
    }
}

struct LateResolutionContext {
    pending: Arc<PendingUtds<PendingCandidate>>,
    own_user_id: OwnedUserId,
    emit: NotificationObservationEmit,
    retired: Arc<AtomicBool>,
    session_generation: u64,
}

/// Outcome of re-reading one pending candidate after a decryption report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LateOutcome {
    /// Plaintext was observed (and emitted if it is still a candidate).
    Resolved,
    /// Still ciphertext or not loadable: keep waiting until the TTL.
    StillEncrypted,
}

/// Project a late-decrypted event. A resolved event that is not a candidate
/// (an edit, a redaction, an old event) is still resolved and never emitted.
fn resolve_late_event(
    event: Option<AnySyncTimelineEvent>,
    room_id: &str,
    own_user_id: &UserId,
    emit: &NotificationObservationEmit,
    session_generation: u64,
) -> LateOutcome {
    let Some(event) = event else {
        return LateOutcome::StillEncrypted;
    };
    match event {
        AnySyncTimelineEvent::MessageLike(ref message_like)
            if needs_decryption_follow_up(message_like) =>
        {
            LateOutcome::StillEncrypted
        }
        AnySyncTimelineEvent::MessageLike(message_like) => {
            if let Some(observation) = project_observation(
                &message_like,
                room_id,
                own_user_id,
                now_ms(),
                session_generation,
            ) {
                emit(observation);
            }
            LateOutcome::Resolved
        }
        AnySyncTimelineEvent::State(_) => LateOutcome::Resolved,
    }
}

async fn run_decryption_reports<S, E>(reports: S, context: LateResolutionContext)
where
    S: Stream<Item = Result<RedecryptorReport, E>>,
{
    futures_util::pin_mut!(reports);
    while let Some(report) = reports.next().await {
        if context.retired.load(Ordering::Acquire) {
            return;
        }
        let now = Instant::now();
        let candidates = match DecryptionReport::from_sdk(report) {
            DecryptionReport::Resolved { room_id, event_ids } => context.pending.take_matching(
                &room_id,
                |event_id| event_ids.iter().any(|id| id == event_id),
                now,
            ),
            DecryptionReport::Recheck => context.pending.take_all(now),
        };
        for (registered, candidate) in candidates {
            if context.retired.load(Ordering::Acquire) {
                return;
            }
            let room_id = candidate.room.room_id().to_string();
            let event_id = candidate.original_event.event_id().to_owned();
            let loaded = load_follow_up_event(&candidate.room, &event_id).await;
            if context.retired.load(Ordering::Acquire) {
                return;
            }
            if resolve_late_event(
                loaded,
                &room_id,
                &context.own_user_id,
                &context.emit,
                context.session_generation,
            ) == LateOutcome::StillEncrypted
            {
                context.pending.insert(
                    &room_id,
                    event_id.as_str(),
                    registered,
                    candidate,
                    Instant::now(),
                );
            }
        }
    }
}

async fn load_follow_up_event(
    room: &Room,
    event_id: &matrix_sdk::ruma::EventId,
) -> Option<AnySyncTimelineEvent> {
    room.load_or_fetch_event(
        event_id,
        Some(
            RequestConfig::new()
                .timeout(Duration::from_secs(2))
                .disable_retry(),
        ),
    )
    .await
    .ok()?
    .raw()
    .deserialize()
    .ok()
}

fn needs_decryption_follow_up(event: &AnySyncMessageLikeEvent) -> bool {
    matches!(
        event,
        AnySyncMessageLikeEvent::RoomEncrypted(SyncMessageLikeEvent::Original(encrypted))
            if !matches!(
                encrypted.content.relates_to,
                Some(EncryptedRelation::Replacement(_))
            )
    )
}

#[allow(clippy::too_many_arguments)]
async fn follow_up_encrypted_observation(
    room: Room,
    original_event: AnySyncMessageLikeEvent,
    own_user_id: OwnedUserId,
    emit: NotificationObservationEmit,
    retired: Arc<AtomicBool>,
    pending: Arc<PendingUtds<PendingCandidate>>,
    session_generation: u64,
) {
    let registered = Instant::now();
    let event_id = original_event.event_id().to_owned();
    let room_id = room.room_id().to_string();
    let resolution = resolve_encrypted_observation(
        EncryptedObservationContext {
            original_event: &original_event,
            room_id: &room_id,
            own_user_id: &own_user_id,
            emit: &emit,
            retired: &retired,
            session_generation,
        },
        &DECRYPT_FOLLOW_UP_DELAYS_MS,
        || load_follow_up_event(&room, &event_id),
    )
    .await;
    if resolution == EncryptedResolution::Unresolved && !retired.load(Ordering::Acquire) {
        // Keep it for a late key; the decryption-report task re-reads it.
        pending.insert(
            &room_id,
            event_id.as_str(),
            registered,
            PendingCandidate {
                room,
                original_event,
            },
            Instant::now(),
        );
    }
}

/// Result of the quick follow-up for one encrypted candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EncryptedResolution {
    /// Plaintext was observed, or the follow-up was retired.
    Resolved,
    /// Still ciphertext after every quick retry.
    Unresolved,
}

// Both the SDK loader and raw-event regressions run this bounded async path.
// A resolved non-candidate is terminal: an edit/redaction cannot become the
// original ciphertext fallback after projection hygiene has rejected it.
struct EncryptedObservationContext<'a> {
    original_event: &'a AnySyncMessageLikeEvent,
    room_id: &'a str,
    own_user_id: &'a UserId,
    emit: &'a NotificationObservationEmit,
    retired: &'a AtomicBool,
    session_generation: u64,
}

async fn resolve_encrypted_observation<F, Fut>(
    context: EncryptedObservationContext<'_>,
    delays_ms: &[u64],
    mut load: F,
) -> EncryptedResolution
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Option<AnySyncTimelineEvent>>,
{
    let EncryptedObservationContext {
        original_event,
        room_id,
        own_user_id,
        emit,
        retired,
        session_generation,
    } = context;
    for delay_ms in delays_ms {
        if retired.load(Ordering::Acquire) {
            return EncryptedResolution::Resolved;
        }
        tokio::time::sleep(Duration::from_millis(*delay_ms)).await;
        if retired.load(Ordering::Acquire) {
            return EncryptedResolution::Resolved;
        }
        let Some(event) = load().await else {
            continue;
        };
        if retired.load(Ordering::Acquire) {
            return EncryptedResolution::Resolved;
        }
        if matches!(
            event,
            AnySyncTimelineEvent::MessageLike(ref message_like)
                if needs_decryption_follow_up(message_like)
        ) {
            // Only unresolved original ciphertext may consume more budget.
            // Redacted ciphertext and encrypted replacements are terminal
            // filtered events, just like decrypted redactions and edits.
            continue;
        }
        if let AnySyncTimelineEvent::MessageLike(message_like) = event {
            if let Some(observation) = project_observation(
                &message_like,
                room_id,
                own_user_id,
                now_ms(),
                session_generation,
            ) {
                emit(observation);
            }
        }
        return EncryptedResolution::Resolved;
    }
    if retired.load(Ordering::Acquire) {
        return EncryptedResolution::Resolved;
    }
    // Every retry remained encrypted or unavailable. Offer one bounded opaque
    // observation; decide still refuses ciphertext without recording dedup.
    // This intentionally suppresses the former generic platform alert while
    // plaintext/classification is unavailable. If lookup now resolves plaintext,
    // decide reclassifies it and rechecks approval expiry before delivery.
    if let Some(observation) = project_observation(
        original_event,
        room_id,
        own_user_id,
        now_ms(),
        session_generation,
    ) {
        emit(observation);
    }
    EncryptedResolution::Unresolved
}

/// Pure projection of one synced message-like event onto an observation.
/// `None` means the event is not a notification candidate.
pub fn project_observation(
    event: &AnySyncMessageLikeEvent,
    room_id: &str,
    own_user_id: &UserId,
    now_ms: u64,
    session_generation: u64,
) -> Option<NativeNotificationObservation> {
    if event.is_redacted() {
        return None;
    }
    if event.sender() == own_user_id {
        return None;
    }
    let origin_server_ts: u64 = event.origin_server_ts().0.into();
    if now_ms.saturating_sub(origin_server_ts) > NOTIFICATION_OBSERVATION_WINDOW_MS {
        return None;
    }
    let agent_approval = match event {
        AnySyncMessageLikeEvent::RoomMessage(SyncMessageLikeEvent::Original(message)) => {
            if matches!(
                message.content.relates_to,
                Some(MessageRelation::Replacement(_))
            ) {
                return None;
            }
            crate::app::agent_approvals::classify_agent_approval(
                message.content.body(),
                event.sender().as_str(),
                own_user_id.as_str(),
                origin_server_ts,
                now_ms,
                std::iter::empty(),
            )
            .ok()
            .map(|classification| NativeAgentApprovalObservation {
                expires_at: classification.expires_at,
                expired: classification.expired,
            })
        }
        AnySyncMessageLikeEvent::RoomEncrypted(SyncMessageLikeEvent::Original(encrypted)) => {
            if matches!(
                encrypted.content.relates_to,
                Some(EncryptedRelation::Replacement(_))
            ) {
                return None;
            }
            None
        }
        AnySyncMessageLikeEvent::Sticker(SyncMessageLikeEvent::Original(_)) => None,
        _ => return None,
    };
    let event_type = match event.event_type() {
        MessageLikeEventType::RoomMessage => "m.room.message",
        MessageLikeEventType::RoomEncrypted => "m.room.encrypted",
        MessageLikeEventType::Sticker => "m.sticker",
        _ => return None,
    };
    Some(NativeNotificationObservation {
        session_generation,
        room_id: room_id.to_owned(),
        event_id: event.event_id().to_string(),
        sender: event.sender().to_string(),
        event_type: event_type.to_owned(),
        origin_server_ts,
        agent_approval,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_sdk::ruma::user_id;

    const ROOM: &str = "!room:example.org";
    const NOW: u64 = 1_700_000_000_000;

    fn message(
        sender: &UserId,
        at_ms: u64,
        event_type: &str,
        content: serde_json::Value,
    ) -> AnySyncMessageLikeEvent {
        serde_json::from_value(serde_json::json!({
            "type": event_type,
            "event_id": "$observed",
            "sender": sender,
            "origin_server_ts": at_ms,
            "content": content,
        }))
        .expect("valid sync event")
    }

    fn text(sender: &UserId, at_ms: u64, body: &str) -> AnySyncMessageLikeEvent {
        message(
            sender,
            at_ms,
            "m.room.message",
            serde_json::json!({ "msgtype": "m.text", "body": body }),
        )
    }

    #[test]
    fn approval_classification_uses_complete_body_and_core_expiry() {
        let me = user_id!("@me:example.org");
        let bot = user_id!("@bot:example.org");
        let body = format!(
            "Approval Required: Dangerous Command\n{}",
            "x".repeat(100_001)
        );
        assert!(
            project_observation(&text(bot, NOW, &body), ROOM, me, NOW, 7)
                .unwrap()
                .agent_approval
                .is_none()
        );
        let body = "Approval Required: Dangerous Command\necho hello";
        let observation =
            project_observation(&text(bot, NOW - 1_000, body), ROOM, me, NOW, 7).unwrap();
        assert_eq!(
            observation.agent_approval,
            Some(NativeAgentApprovalObservation {
                expires_at: NOW - 1_000 + crate::app::agent_approvals::AGENT_APPROVAL_TTL_MS,
                expired: false
            })
        );
        let wire = serde_json::to_value(observation).unwrap();
        assert!(wire.get("body").is_none());
        assert!(wire.get("agentApproval").is_some());
    }

    #[test]
    fn recent_message_carries_no_raw_prompt_body() {
        let me = user_id!("@me:example.org");
        let bob = user_id!("@bob:example.org");
        let long_body = "x".repeat(8_050);
        let event = text(bob, NOW - 1_000, &long_body);
        let observed = project_observation(&event, ROOM, me, NOW, 7).expect("observed");
        assert_eq!(observed.session_generation, 7);
        assert_eq!(observed.room_id, ROOM);
        assert_eq!(observed.event_id, "$observed");
        assert_eq!(observed.sender, bob.as_str());
        assert_eq!(observed.event_type, "m.room.message");
        assert_eq!(observed.origin_server_ts, NOW - 1_000);
        assert!(observed.agent_approval.is_none());
        let wire = serde_json::to_value(&observed).unwrap();
        assert!(wire.get("sessionGeneration").is_some());
        assert!(wire.get("originServerTs").is_some());
        assert!(wire.get("highlight").is_none(), "no policy verdict crosses");
    }

    #[test]
    fn own_events_old_events_and_edits_are_not_observed() {
        let me = user_id!("@me:example.org");
        let bob = user_id!("@bob:example.org");
        assert!(project_observation(&text(me, NOW, "mine"), ROOM, me, NOW, 7).is_none());
        assert!(project_observation(
            &text(bob, NOW - NOTIFICATION_OBSERVATION_WINDOW_MS - 1, "old"),
            ROOM,
            me,
            NOW,
            7
        )
        .is_none());
        let edit = message(
            bob,
            NOW,
            "m.room.message",
            serde_json::json!({
                "msgtype": "m.text",
                "body": "* edited",
                "m.new_content": { "msgtype": "m.text", "body": "edited" },
                "m.relates_to": { "rel_type": "m.replace", "event_id": "$original" },
            }),
        );
        assert!(project_observation(&edit, ROOM, me, NOW, 7).is_none());
        let encrypted_edit = message(
            bob,
            NOW,
            "m.room.encrypted",
            serde_json::json!({
                "algorithm": "m.megolm.v1.aes-sha2",
                "ciphertext": "AwgAE...",
                "sender_key": "abc",
                "session_id": "def",
                "device_id": "DEV",
                "m.relates_to": { "rel_type": "m.replace", "event_id": "$original" },
            }),
        );
        assert!(project_observation(&encrypted_edit, ROOM, me, NOW, 7).is_none());
        let redacted = serde_json::from_value::<AnySyncMessageLikeEvent>(serde_json::json!({
            "type": "m.room.message",
            "event_id": "$observed",
            "sender": bob,
            "origin_server_ts": NOW,
            "content": {},
            "unsigned": { "redacted_because": {
                "type": "m.room.redaction",
                "event_id": "$redaction",
                "sender": bob,
                "origin_server_ts": NOW,
                "content": { "redacts": "$observed" },
                "redacts": "$observed",
            } },
        }))
        .expect("redacted event");
        assert!(project_observation(&redacted, ROOM, me, NOW, 7).is_none());
    }

    #[test]
    fn undecryptable_and_sticker_events_have_no_approval_classification() {
        let me = user_id!("@me:example.org");
        let bob = user_id!("@bob:example.org");
        let encrypted = message(
            bob,
            NOW,
            "m.room.encrypted",
            serde_json::json!({
                "algorithm": "m.megolm.v1.aes-sha2",
                "ciphertext": "AwgAE...",
                "sender_key": "abc",
                "session_id": "def",
                "device_id": "DEV",
            }),
        );
        let observed = project_observation(&encrypted, ROOM, me, NOW, 7).expect("observed");
        assert_eq!(observed.event_type, "m.room.encrypted");
        assert_eq!(observed.agent_approval, None, "ciphertext never crosses");
        let sticker = message(
            bob,
            NOW,
            "m.sticker",
            serde_json::json!({
                "body": "a sticker",
                "info": { "w": 1, "h": 1 },
                "url": "mxc://example.org/sticker",
            }),
        );
        let observed = project_observation(&sticker, ROOM, me, NOW, 7).expect("observed");
        assert_eq!(observed.event_type, "m.sticker");
        assert_eq!(observed.agent_approval, None);
        let reaction = message(
            bob,
            NOW,
            "m.reaction",
            serde_json::json!({
                "m.relates_to": { "rel_type": "m.annotation", "event_id": "$x", "key": "👍" },
            }),
        );
        assert!(project_observation(&reaction, ROOM, me, NOW, 7).is_none());
    }

    #[test]
    fn future_timestamps_are_still_observed() {
        // Clock skew between homeserver and device must not swallow live
        // messages; only the past is bounded.
        let me = user_id!("@me:example.org");
        let bob = user_id!("@bob:example.org");
        assert!(project_observation(&text(bob, NOW + 60_000, "hi"), ROOM, me, NOW, 7).is_some());
    }

    #[test]
    fn encrypted_messages_retry_after_decrypt_but_encrypted_edits_do_not() {
        let bob = user_id!("@bob:example.org");
        let encrypted = message(
            bob,
            NOW,
            "m.room.encrypted",
            serde_json::json!({
                "algorithm": "m.megolm.v1.aes-sha2",
                "ciphertext": "AwgAE...",
                "sender_key": "abc",
                "session_id": "def",
                "device_id": "DEV",
            }),
        );
        assert!(needs_decryption_follow_up(&encrypted));
        let encrypted_edit = message(
            bob,
            NOW,
            "m.room.encrypted",
            serde_json::json!({
                "algorithm": "m.megolm.v1.aes-sha2",
                "ciphertext": "AwgAE...",
                "sender_key": "abc",
                "session_id": "def",
                "device_id": "DEV",
                "m.relates_to": { "rel_type": "m.replace", "event_id": "$original" },
            }),
        );
        assert!(!needs_decryption_follow_up(&encrypted_edit));
        assert!(!needs_decryption_follow_up(&text(bob, NOW, "plain")));
    }

    #[test]
    fn encrypted_follow_up_derives_room_id_from_the_room() {
        let source = include_str!("observation.rs");
        assert!(source.contains("let room_id = room.room_id().to_string();"));
    }
    fn ciphertext(at_ms: u64) -> AnySyncMessageLikeEvent {
        message(
            user_id!("@bob:example.org"),
            at_ms,
            "m.room.encrypted",
            serde_json::json!({
                "algorithm": "m.megolm.v1.aes-sha2", "ciphertext": "AwgAE...",
                "sender_key": "abc", "session_id": "def", "device_id": "DEV"
            }),
        )
    }

    async fn resolve_sequence(
        events: Vec<Option<AnySyncTimelineEvent>>,
        original: AnySyncMessageLikeEvent,
    ) -> (Vec<NativeNotificationObservation>, usize) {
        use std::collections::VecDeque;
        use std::sync::Mutex;
        let output = Arc::new(Mutex::new(Vec::new()));
        let recorded = output.clone();
        let emit: NotificationObservationEmit =
            Arc::new(move |event| recorded.lock().unwrap().push(event));
        let mut pending = VecDeque::from(events);
        let attempts = pending.len();
        resolve_encrypted_observation(
            EncryptedObservationContext {
                original_event: &original,
                room_id: ROOM,
                own_user_id: user_id!("@me:example.org"),
                emit: &emit,
                retired: &AtomicBool::new(false),
                session_generation: 7,
            },
            &vec![0; attempts],
            || {
                // No encrypted observation can be delivered while retries run.
                assert!(output.lock().unwrap().is_empty());
                let next = pending.pop_front().unwrap();
                async move {
                    tokio::task::yield_now().await;
                    next
                }
            },
        )
        .await;
        let delivered = output.lock().unwrap().clone();
        (delivered, attempts - pending.len())
    }

    #[tokio::test]
    async fn delayed_plaintext_approval_is_the_only_observation() {
        let at = now_ms();
        let original = ciphertext(at);
        let approval = text(
            user_id!("@bob:example.org"),
            at,
            "Approval Required: Dangerous Command\necho hello",
        );
        let (events, attempts) = resolve_sequence(
            vec![
                Some(AnySyncTimelineEvent::MessageLike(original.clone())),
                Some(AnySyncTimelineEvent::MessageLike(approval)),
            ],
            original,
        )
        .await;
        assert_eq!(attempts, 2);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "m.room.message");
        assert!(events[0]
            .agent_approval
            .as_ref()
            .is_some_and(|approval| !approval.expired));
        assert!(serde_json::to_value(&events[0])
            .unwrap()
            .get("body")
            .is_none());
    }

    #[tokio::test]
    async fn decrypted_edits_redactions_and_non_candidates_never_fall_back_to_ciphertext() {
        let at = now_ms();
        let edit = message(
            user_id!("@bob:example.org"),
            at,
            "m.room.message",
            serde_json::json!({
                "msgtype": "m.text", "body": "* edit",
                "m.new_content": { "msgtype": "m.text", "body": "edit" },
                "m.relates_to": { "rel_type": "m.replace", "event_id": "$original" }
            }),
        );
        let redacted_raw = serde_json::json!({
            "type": "m.room.message", "event_id": "$observed", "sender": "@bob:example.org",
            "origin_server_ts": at, "content": {}, "unsigned": { "redacted_because": {
                "type": "m.room.redaction", "event_id": "$redaction", "sender": "@bob:example.org",
                "origin_server_ts": at, "content": { "redacts": "$observed" }, "redacts": "$observed"
            } }
        });
        let redacted: AnySyncMessageLikeEvent =
            serde_json::from_value(redacted_raw.clone()).unwrap();
        let mut encrypted_redacted = redacted_raw;
        encrypted_redacted["type"] = serde_json::json!("m.room.encrypted");
        let encrypted_redacted: AnySyncMessageLikeEvent =
            serde_json::from_value(encrypted_redacted).unwrap();
        let encrypted_edit = message(
            user_id!("@bob:example.org"),
            at,
            "m.room.encrypted",
            serde_json::json!({ "algorithm": "m.megolm.v1.aes-sha2", "ciphertext": "AwgAE...",
                "sender_key": "abc", "session_id": "def", "device_id": "DEV",
                "m.relates_to": { "rel_type": "m.replace", "event_id": "$original" } }),
        );
        for resolved in [
            edit,
            redacted,
            encrypted_redacted,
            encrypted_edit,
            text(user_id!("@me:example.org"), at, "own"),
        ] {
            let original = ciphertext(at);
            let (events, attempts) = resolve_sequence(
                vec![
                    Some(AnySyncTimelineEvent::MessageLike(original.clone())),
                    Some(AnySyncTimelineEvent::MessageLike(resolved)),
                    None,
                ],
                original,
            )
            .await;
            assert_eq!(attempts, 2, "resolved non-candidates terminate retries");
            assert!(
                events.is_empty(),
                "filtered plaintext cannot become ciphertext fallback"
            );
        }
    }

    #[tokio::test]
    async fn unresolved_ciphertext_is_offered_once_only_after_the_entire_budget() {
        let original = ciphertext(now_ms());
        let (events, attempts) = resolve_sequence(
            vec![
                None,
                Some(AnySyncTimelineEvent::MessageLike(original.clone())),
                None,
            ],
            original,
        )
        .await;
        assert_eq!(attempts, 3);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "m.room.encrypted");
        assert!(events[0].agent_approval.is_none());
    }

    #[tokio::test]
    async fn retirement_while_sdk_lookup_is_in_flight_emits_nothing() {
        use std::sync::Mutex;
        let output = Arc::new(Mutex::new(Vec::new()));
        let recorded = output.clone();
        let emit: NotificationObservationEmit =
            Arc::new(move |event| recorded.lock().unwrap().push(event));
        let retired = AtomicBool::new(false);
        let original = ciphertext(now_ms());
        resolve_encrypted_observation(
            EncryptedObservationContext {
                original_event: &original,
                room_id: ROOM,
                own_user_id: user_id!("@me:example.org"),
                emit: &emit,
                retired: &retired,
                session_generation: 7,
            },
            &[0],
            || async {
                tokio::task::yield_now().await;
                retired.store(true, Ordering::Release);
                Some(AnySyncTimelineEvent::MessageLike(text(
                    user_id!("@bob:example.org"),
                    now_ms(),
                    "hello",
                )))
            },
        )
        .await;
        assert!(output.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn owner_drop_without_explicit_retire_cancels_inflight_follow_up() {
        use matrix_sdk::test_utils::mocks::MatrixMockServer;
        let server = MatrixMockServer::new().await;
        let client = server.client_builder().build().await;
        let output = Arc::new(Mutex::new(Vec::new()));
        let recorded = output.clone();
        let emit: NotificationObservationEmit =
            Arc::new(move |event| recorded.lock().unwrap().push(event));
        let owner = NativeNotificationObservationOwner::start(&client, emit.clone(), 7).unwrap();
        let retired = owner.retired.clone();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (_resolve_tx, resolve_rx) = tokio::sync::oneshot::channel::<()>();
        let task_retired = retired.clone();
        let task = tokio::spawn(async move {
            let original = ciphertext(now_ms());
            let mut started = Some(started_tx);
            let mut resolve = Some(resolve_rx);
            resolve_encrypted_observation(
                EncryptedObservationContext {
                    original_event: &original,
                    room_id: ROOM,
                    own_user_id: user_id!("@me:example.org"),
                    emit: &emit,
                    retired: &task_retired,
                    session_generation: 7,
                },
                &[0],
                || {
                    started.take().unwrap().send(()).unwrap();
                    let resolve = resolve.take().unwrap();
                    async move {
                        let _ = resolve.await;
                        Some(AnySyncTimelineEvent::MessageLike(text(
                            user_id!("@bob:example.org"),
                            now_ms(),
                            "plain",
                        )))
                    }
                },
            )
            .await;
        });
        track_follow_up(&owner.follow_ups, &owner.retired, task.abort_handle());
        started_rx.await.unwrap();
        drop(owner);
        assert!(retired.load(Ordering::Acquire));
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(output.lock().unwrap().is_empty());
        // A handler racing shutdown cannot register new detached work.
        let late = tokio::spawn(std::future::pending::<()>());
        track_follow_up(&Mutex::new(Vec::new()), &retired, late.abort_handle());
        assert!(late.await.unwrap_err().is_cancelled());
    }

    #[test]
    fn pending_utds_are_bounded_and_drop_the_oldest() {
        let pending = PendingUtds::<u32>::default();
        let now = Instant::now();
        for index in 0..(PENDING_UTD_LIMIT as u32 + 5) {
            pending.insert(ROOM, &format!("$event{index}"), now, index, now);
        }
        assert_eq!(pending.len(), PENDING_UTD_LIMIT);
        let all = pending.take_all(now);
        assert_eq!(all.first().map(|(_, value)| *value), Some(5));
        assert_eq!(pending.len(), 0);
    }

    #[test]
    fn pending_utds_expire_after_the_observation_window() {
        let pending = PendingUtds::<u32>::default();
        let start = Instant::now();
        pending.insert(ROOM, "$old", start, 1, start);
        pending.insert(
            ROOM,
            "$new",
            start + Duration::from_secs(60),
            2,
            start + Duration::from_secs(60),
        );
        let later = start + PENDING_UTD_TTL + Duration::from_secs(1);
        let live: Vec<u32> = pending
            .take_all(later)
            .into_iter()
            .map(|(_, value)| value)
            .collect();
        assert_eq!(live, vec![2]);
        // A candidate first seen before the window cannot be re-registered.
        pending.insert(ROOM, "$old", start, 1, later);
        assert_eq!(pending.len(), 0);
    }

    #[test]
    fn resolved_report_takes_only_matching_room_and_events() {
        let pending = PendingUtds::<&str>::default();
        let now = Instant::now();
        pending.insert(ROOM, "$a", now, "a", now);
        pending.insert(ROOM, "$b", now, "b", now);
        pending.insert("!other:example.org", "$a", now, "other", now);
        let taken: Vec<&str> = pending
            .take_matching(ROOM, |event_id| event_id == "$a", now)
            .into_iter()
            .map(|(_, value)| value)
            .collect();
        assert_eq!(taken, vec!["a"]);
        assert_eq!(pending.len(), 2);
        // Re-registering the same event replaces it rather than duplicating.
        pending.insert(ROOM, "$b", now, "b2", now);
        assert_eq!(pending.len(), 2);
    }

    #[test]
    fn sdk_reports_reduce_to_resolved_or_recheck() {
        use matrix_sdk::ruma::{owned_event_id, owned_room_id};
        let report = RedecryptorReport::ResolvedUtds {
            room_id: owned_room_id!("!room:example.org"),
            events: [owned_event_id!("$a")].into_iter().collect(),
        };
        assert_eq!(
            DecryptionReport::from_sdk::<()>(Ok(report)),
            DecryptionReport::Resolved {
                room_id: ROOM.to_owned(),
                event_ids: vec!["$a".to_owned()],
            }
        );
        assert_eq!(
            DecryptionReport::from_sdk::<()>(Ok(RedecryptorReport::Lagging)),
            DecryptionReport::Recheck
        );
        assert_eq!(
            DecryptionReport::from_sdk::<()>(Ok(RedecryptorReport::BackupAvailable)),
            DecryptionReport::Recheck
        );
        assert_eq!(
            DecryptionReport::from_sdk::<&str>(Err("lagged")),
            DecryptionReport::Recheck,
            "a lagged broadcast receiver may have missed a report"
        );
    }

    fn recording_emit() -> (
        NotificationObservationEmit,
        Arc<Mutex<Vec<NativeNotificationObservation>>>,
    ) {
        let output = Arc::new(Mutex::new(Vec::new()));
        let recorded = output.clone();
        let emit: NotificationObservationEmit =
            Arc::new(move |event| recorded.lock().unwrap().push(event));
        (emit, output)
    }

    #[test]
    fn a_key_that_arrives_late_still_produces_the_notification() {
        let (emit, output) = recording_emit();
        let approval = text(
            user_id!("@bob:example.org"),
            now_ms(),
            "Approval Required: Dangerous Command\necho hello",
        );
        assert_eq!(
            resolve_late_event(
                Some(AnySyncTimelineEvent::MessageLike(approval)),
                ROOM,
                user_id!("@me:example.org"),
                &emit,
                7,
            ),
            LateOutcome::Resolved
        );
        let events = output.lock().unwrap().clone();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "m.room.message");
        assert!(events[0].agent_approval.is_some());
    }

    #[test]
    fn late_reports_keep_ciphertext_waiting_and_never_emit_filtered_plaintext() {
        let (emit, output) = recording_emit();
        let at = now_ms();
        assert_eq!(
            resolve_late_event(
                Some(AnySyncTimelineEvent::MessageLike(ciphertext(at))),
                ROOM,
                user_id!("@me:example.org"),
                &emit,
                7,
            ),
            LateOutcome::StillEncrypted
        );
        assert_eq!(
            resolve_late_event(None, ROOM, user_id!("@me:example.org"), &emit, 7),
            LateOutcome::StillEncrypted
        );
        let own = text(user_id!("@me:example.org"), at, "own");
        assert_eq!(
            resolve_late_event(
                Some(AnySyncTimelineEvent::MessageLike(own)),
                ROOM,
                user_id!("@me:example.org"),
                &emit,
                7,
            ),
            LateOutcome::Resolved
        );
        assert!(output.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn quick_follow_up_reports_unresolved_ciphertext_for_late_keys() {
        let (emit, _output) = recording_emit();
        let original = ciphertext(now_ms());
        let resolution = resolve_encrypted_observation(
            EncryptedObservationContext {
                original_event: &original,
                room_id: ROOM,
                own_user_id: user_id!("@me:example.org"),
                emit: &emit,
                retired: &AtomicBool::new(false),
                session_generation: 7,
            },
            &[0, 0],
            || async { None },
        )
        .await;
        assert_eq!(resolution, EncryptedResolution::Unresolved);
    }

    #[test]
    fn quick_follow_up_budget_is_short_because_late_keys_use_reports() {
        let total: u64 = DECRYPT_FOLLOW_UP_DELAYS_MS.iter().sum();
        assert!(total <= 3_000, "quick follow-up budget is {total} ms");
        let source = include_str!("observation.rs");
        assert!(source.contains(".subscribe_to_decryption_reports()"));
    }
}
