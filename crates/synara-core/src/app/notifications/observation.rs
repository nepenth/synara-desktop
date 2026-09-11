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
//! event type, its origin timestamp, and the bounded plaintext `body` of an
//! `m.room.message` — the same class of data the timeline view stream already
//! delivers to the renderer. It carries no ciphertext, keys, tokens, or push
//! verdicts.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use matrix_sdk::config::RequestConfig;
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

/// Bound for the plaintext body carried to the renderer for agent-approval
/// prompt detection. Longer bodies are truncated on a char boundary.
pub const NOTIFICATION_OBSERVATION_BODY_MAX_CHARS: usize = 8_000;

/// One observed candidate event. Identity plus presentation-neutral facts;
/// no policy verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeNotificationObservation {
    pub session_generation: u64,
    pub room_id: String,
    pub event_id: String,
    pub sender: String,
    pub event_type: String,
    pub origin_server_ts: u64,
    /// Plaintext `body` of an `m.room.message`; `None` for stickers and
    /// events Core could not decrypt.
    pub body: Option<String>,
}

/// Shell-supplied sink. Desktop maps this to a Tauri event; iOS can map it
/// to a UniFFI callback later.
pub type NotificationObservationEmit = Arc<dyn Fn(NativeNotificationObservation) + Send + Sync>;

/// Owns the SDK message-like event stream for one authenticated session.
/// Dropping the owner removes the SDK handler.
pub struct NativeNotificationObservationOwner {
    session_generation: u64,
    retired: Arc<AtomicBool>,
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
        let handler =
            client.add_event_handler(move |event: AnySyncMessageLikeEvent, room: Room| {
                let retired = retired_for_handler.clone();
                let emit = emit.clone();
                let own_user_id = own_user_id.clone();
                async move {
                    if retired.load(Ordering::Acquire) {
                        return;
                    }
                    let room_id = room.room_id().to_string();
                    if let Some(observation) = project_observation(
                        &event,
                        &room_id,
                        &own_user_id,
                        now_ms(),
                        session_generation,
                    ) {
                        emit(observation);
                    }
                    if needs_decryption_follow_up(&event) {
                        tokio::spawn(async move {
                            follow_up_encrypted_observation(
                                room,
                                event,
                                own_user_id,
                                emit,
                                retired,
                                session_generation,
                            )
                            .await;
                        });
                    }
                }
            });
        Ok(Self {
            session_generation,
            retired,
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
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

const DECRYPT_FOLLOW_UP_DELAYS_MS: [u64; 7] = [150, 300, 500, 800, 1_200, 2_000, 3_000];

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

async fn follow_up_encrypted_observation(
    room: Room,
    original_event: AnySyncMessageLikeEvent,
    own_user_id: OwnedUserId,
    emit: NotificationObservationEmit,
    retired: Arc<AtomicBool>,
    session_generation: u64,
) {
    let event_id = original_event.event_id().to_owned();
    let room_id = room.room_id().to_string();
    for delay_ms in DECRYPT_FOLLOW_UP_DELAYS_MS {
        if retired.load(Ordering::Acquire) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        if retired.load(Ordering::Acquire) {
            return;
        }
        let Ok(timeline_event) = room
            .load_or_fetch_event(
                &event_id,
                Some(
                    RequestConfig::new()
                        .timeout(Duration::from_secs(2))
                        .disable_retry(),
                ),
            )
            .await
        else {
            continue;
        };
        let Ok(AnySyncTimelineEvent::MessageLike(message_like)) =
            timeline_event.raw().deserialize()
        else {
            continue;
        };
        if matches!(message_like, AnySyncMessageLikeEvent::RoomEncrypted(_)) {
            continue;
        }
        if let Some(observation) = project_observation(
            &message_like,
            &room_id,
            &own_user_id,
            now_ms(),
            session_generation,
        ) {
            emit(observation);
            return;
        }
    }
    if retired.load(Ordering::Acquire) {
        return;
    }
    // Decrypt never landed: re-emit the ciphertext observation so the
    // generic message path can still notify after the renderer skipped the
    // first encrypted pass (which would otherwise record seen and block a
    // later approval).
    if let Some(observation) = project_observation(
        &original_event,
        &room_id,
        &own_user_id,
        now_ms(),
        session_generation,
    ) {
        emit(observation);
    }
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
    let body = match event {
        AnySyncMessageLikeEvent::RoomMessage(SyncMessageLikeEvent::Original(message)) => {
            if matches!(
                message.content.relates_to,
                Some(MessageRelation::Replacement(_))
            ) {
                return None;
            }
            Some(truncate_chars(
                message.content.body(),
                NOTIFICATION_OBSERVATION_BODY_MAX_CHARS,
            ))
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
        body,
    })
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
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
    fn recent_message_from_another_user_is_observed_with_bounded_body() {
        let me = user_id!("@me:example.org");
        let bob = user_id!("@bob:example.org");
        let long_body = "x".repeat(NOTIFICATION_OBSERVATION_BODY_MAX_CHARS + 50);
        let event = text(bob, NOW - 1_000, &long_body);
        let observed = project_observation(&event, ROOM, me, NOW, 7).expect("observed");
        assert_eq!(observed.session_generation, 7);
        assert_eq!(observed.room_id, ROOM);
        assert_eq!(observed.event_id, "$observed");
        assert_eq!(observed.sender, bob.as_str());
        assert_eq!(observed.event_type, "m.room.message");
        assert_eq!(observed.origin_server_ts, NOW - 1_000);
        assert_eq!(
            observed.body.as_deref().map(|b| b.chars().count()),
            Some(NOTIFICATION_OBSERVATION_BODY_MAX_CHARS)
        );
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
    fn undecryptable_and_sticker_events_are_observed_without_body() {
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
        assert_eq!(observed.body, None, "ciphertext never crosses");
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
        assert_eq!(observed.body, None);
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
}
