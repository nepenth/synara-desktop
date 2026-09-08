//! Typed desktop notification decision owner (A9 follow-on).
//!
//! Core owns the suppress/show policy; platforms own delivery, sound
//! preference, Do-Not-Disturb, and OS presentation. This owner wraps the
//! [`NotificationIndex`] harness with a closed decision table whose inputs
//! Core resolves itself from the pinned Matrix SDK:
//!
//! - The renderer submits only the identity of an event it observed
//!   (`room_id`, `event_id`) plus privacy-filtered product strings. It sends
//!   no mode, no highlight flag, no sender comparison, and no message body.
//! - Core loads the exact event through the SDK event cache (falling back to
//!   an authenticated `/event` fetch) and reads the SDK-evaluated push
//!   actions. Those actions are the single owner of room mode, keywords,
//!   `m.mentions`, legacy display-name matching, `@room` power-level checks,
//!   the account suppress-edits override, and rule ordering. `None` push
//!   actions are recomputed once through `Room::event_push_actions`; when the
//!   room context is still unavailable the decision fails closed.
//! - `is_own_event` is Core's comparison of the event sender against the
//!   bound session. Own events never notify.
//! - focused room + `(room_id, event_id)` dedup + 128-pending cap: owned by
//!   the wrapped [`NotificationIndex`].
//!
//! Title/body are already privacy-filtered product strings (room names,
//! usernames, fixed summaries) — never raw ciphertext or event dumps. Core
//! truncates them to the desktop sanitizer caps (120/500 chars) so Core and
//! `desktop_notifications.rs` agree without rejecting legitimate long names.
//!
//! No OS posting, no tokens, no credentials, no media bytes on this path.
//! Account binding follows the `NativeHttpPusherOwner` template: the owner
//! captures the exact authenticated identity at session attach and answers
//! `owns_session` without ever serializing that identity.

use std::sync::Mutex;
use std::time::Duration;

use matrix_sdk::config::RequestConfig;
use matrix_sdk::ruma::push::Action;
use matrix_sdk::ruma::{EventId, RoomId};
use matrix_sdk::Client;
use serde::{Deserialize, Serialize};

use crate::dto::{NotificationCandidate, NotificationKind};

use super::error::NotificationError;
use super::index::NotificationIndex;

/// Desktop sanitizer caps mirrored here so Core decisions agree with
/// `desktop_notifications.rs` delivery bounds.
pub const NOTIFICATION_TITLE_MAX_CHARS: usize = 120;
pub const NOTIFICATION_BODY_MAX_CHARS: usize = 500;
/// Deep-link routes stay internal. Malformed routes degrade to no route
/// (notification still shows) rather than failing the whole decision; the
/// desktop sanitizer remains the final delivery boundary.
pub const NOTIFICATION_ROUTE_MAX_CHARS: usize = 512;
/// Bound for the authenticated `/event` fallback when the observed event is
/// not in the SDK event cache. The renderer treats a failed decision as
/// transient and resubmits on its next sync scan, so one short attempt is
/// enough; retries would only delay other pending decisions.
const EVENT_FETCH_TIMEOUT: Duration = Duration::from_secs(10);

/// Closed notification kind vocabulary for the decision table. Mirrors the
/// [`NotificationKind`] DTO without moving the DTO itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationDecisionKind {
    Message,
    Invite,
    AgentApproval,
    LaterReminder,
}

impl NotificationDecisionKind {
    pub fn parse(kind: &str) -> Result<Self, &'static str> {
        match kind.trim() {
            "message" => Ok(Self::Message),
            "invite" => Ok(Self::Invite),
            "agent_approval" => Ok(Self::AgentApproval),
            "later_reminder" => Ok(Self::LaterReminder),
            _ => Err("v-notify.invalid-kind"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Message => "message",
            Self::Invite => "invite",
            Self::AgentApproval => "agent_approval",
            Self::LaterReminder => "later_reminder",
        }
    }

    fn as_dto(self) -> NotificationKind {
        match self {
            Self::Message => NotificationKind::Message,
            Self::Invite => NotificationKind::Invite,
            Self::AgentApproval => NotificationKind::AgentApproval,
            Self::LaterReminder => NotificationKind::LaterReminder,
        }
    }
}

/// Closed suppress reason for the decision readback. Static strings only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationSuppressReason {
    OwnEvent,
    /// The SDK-evaluated push rules produced no `notify` action: muted room,
    /// mentions-only room without a mention/keyword, suppressed edit, or any
    /// other server-side rule the user configured.
    PushRulesNoNotify,
    FocusedRoom,
    DuplicateEvent,
}

impl NotificationSuppressReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OwnEvent => "own-event",
            Self::PushRulesNoNotify => "push-rules-no-notify",
            Self::FocusedRoom => "focused-room",
            Self::DuplicateEvent => "duplicate-event",
        }
    }
}

/// SDK-evaluated push outcome for one event. This is the only notification
/// policy input for timeline messages; nothing here is reconstructed from
/// message text, display names, or room-list modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPushEvaluation {
    pub notify: bool,
    pub highlight: bool,
    pub sound: bool,
}

impl NotificationPushEvaluation {
    /// Fold a Matrix push-action list into the closed evaluation.
    pub fn from_actions(actions: &[Action]) -> Self {
        Self {
            notify: actions.iter().any(Action::should_notify),
            highlight: actions.iter().any(Action::is_highlight),
            sound: actions.iter().any(|action| action.sound().is_some()),
        }
    }

    /// Kinds without a timeline event (invites, agent approvals, Later
    /// reminders) are explicit user commitments and always surface, subject
    /// to focus and dedup.
    pub const fn surface() -> Self {
        Self {
            notify: true,
            highlight: false,
            sound: false,
        }
    }
}

/// Core-resolved facts for one decision. Title/body are pre-filtered product
/// strings; `push` and `is_own_event` come from the SDK, never the renderer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDecisionInput {
    pub room_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    pub kind: NotificationDecisionKind,
    pub title: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    #[serde(default = "default_suppress_if_focused")]
    pub suppress_if_focused_room: bool,
    #[serde(default)]
    pub is_encrypted: bool,
    #[serde(default)]
    pub push: NotificationPushEvaluation,
    #[serde(default)]
    pub is_own_event: bool,
}

fn default_suppress_if_focused() -> bool {
    true
}

/// React/Tauri wire request for `matrix_notification_focus_set`.
///
/// The renderer sends the currently focused room (or null when no room has
/// focus). This is a platform observation Core decides over; unknown keys are
/// rejected so the focus route cannot grow identity or session fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeNotificationFocusSetRequest {
    #[serde(default)]
    pub room_id: Option<String>,
}

/// React/Tauri wire request for `matrix_notification_decide`.
///
/// Identity plus presentation only. Title/body are pre-filtered product
/// strings (room names, usernames, fixed summaries) — never raw ciphertext or
/// event dumps. There is deliberately no `roomMode`, `highlight`,
/// `isEncrypted`, or `isOwnEvent` field: Core resolves all of those from the
/// SDK for the named event. Unknown keys are rejected so the decide route
/// cannot grow policy, credential, token, path, or byte fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeNotificationDecideRequest {
    pub room_id: String,
    #[serde(default)]
    pub event_id: Option<String>,
    pub kind: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub route: Option<String>,
    #[serde(default = "default_suppress_if_focused")]
    pub suppress_if_focused_room: bool,
}

/// React/Tauri wire request for `matrix_notification_dismiss`.
///
/// `outcome` is the platform's delivery receipt for the shown candidate. It
/// is optional so a plain acknowledgement (user dismissed, nothing
/// attempted) keeps working; when present it must use the closed
/// [`NotificationDeliveryOutcome`] vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeNotificationDismissRequest {
    pub candidate_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<NotificationDeliveryOutcome>,
}

/// Closed delivery-receipt vocabulary reported by the platform after it
/// handed a shown candidate to the OS. `failed` means the OS call returned
/// an error; Core records it and releases the candidate without retrying.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NotificationDeliveryOutcome {
    Delivered,
    Failed,
}

impl NotificationDeliveryOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Delivered => "delivered",
            Self::Failed => "failed",
        }
    }
}

/// Bounded, identifier-free delivery receipt ledger for the bound session.
/// Counts advance only when an acknowledgement releases a pending candidate,
/// so repeated acks for one candidate cannot inflate them. The ledger resets
/// with the session generation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDeliveryLedger {
    pub delivered: u64,
    pub failed: u64,
    /// Acknowledged without a receipt (user dismissal, delivery not
    /// attempted, or a platform that does not report outcomes).
    pub unreported: u64,
}

impl NotificationDeliveryLedger {
    fn record(&mut self, outcome: Option<NotificationDeliveryOutcome>) {
        let counter = match outcome {
            Some(NotificationDeliveryOutcome::Delivered) => &mut self.delivered,
            Some(NotificationDeliveryOutcome::Failed) => &mut self.failed,
            None => &mut self.unreported,
        };
        *counter = counter.saturating_add(1);
    }
}

/// Exact readback for `matrix_notification_decide`. `decision` is the closed
/// `show` / `suppress` vocabulary; `reason` is set only on suppress;
/// `candidate` is set only on show. `highlight` and `sound` echo the
/// SDK-evaluated push tweaks for a shown message so presentation can follow
/// the server-side rules without re-deriving them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDecisionReadback {
    pub decision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate: Option<NotificationCandidate>,
    #[serde(default)]
    pub highlight: bool,
    #[serde(default)]
    pub sound: bool,
}

/// Account-bound decision owner. Binds the exact authenticated session at
/// attach (identity never serialized); the wrapped index owns focus, dedup,
/// and the pending cap. The bound client is used only to load the observed
/// event and its SDK push evaluation.
pub struct NativeNotificationDecisionOwner {
    session_generation: u64,
    user_id: String,
    device_id: String,
    homeserver_url: String,
    /// `None` only for table-only unit tests; production owners are always
    /// bound and fail closed on message decisions without a client.
    client: Option<Client>,
    index: Mutex<NotificationIndex>,
    delivery: Mutex<NotificationDeliveryLedger>,
}

/// Facts Core resolved from the SDK for one observed timeline event.
struct ObservedEvent {
    is_own_event: bool,
    is_encrypted: bool,
    push: NotificationPushEvaluation,
}

impl NativeNotificationDecisionOwner {
    pub fn new(client: &Client, session_generation: u64) -> Result<Self, &'static str> {
        let user_id = client
            .user_id()
            .ok_or("v-notify.no-session")?
            .as_str()
            .to_owned();
        let device_id = client.device_id().ok_or("v-notify.no-session")?.to_string();
        let homeserver_url = client.homeserver().as_str().to_owned();
        Ok(Self {
            session_generation,
            user_id,
            device_id,
            homeserver_url,
            client: Some(client.clone()),
            index: Mutex::new(NotificationIndex::new(session_generation)),
            delivery: Mutex::new(NotificationDeliveryLedger::default()),
        })
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    /// Test-only owner without a Matrix client. Production shells use
    /// [`Self::new`], which binds the exact authenticated session. Message
    /// decisions through this owner fail closed (`v-notify.no-client`).
    #[cfg(test)]
    pub fn for_tests(session_generation: u64) -> Self {
        Self {
            session_generation,
            user_id: "@test:example.org".into(),
            device_id: "TESTDEVICE".into(),
            homeserver_url: "https://example.org".into(),
            client: None,
            index: Mutex::new(NotificationIndex::new(session_generation)),
            delivery: Mutex::new(NotificationDeliveryLedger::default()),
        }
    }

    /// Whether this owner is bound to the exact shell session asking for a
    /// decision. Compared inside Core; identity is never returned or echoed
    /// in errors.
    pub fn owns_session(&self, user_id: &str, device_id: &str, homeserver_url: &str) -> bool {
        self.user_id == user_id
            && self.device_id == device_id
            && self.homeserver_url.trim_end_matches('/')
                == homeserver_url.trim().trim_end_matches('/')
    }

    /// Record the platform-observed focused room. `None` clears focus.
    /// Fails closed on malformed room ids; the previous focus is retained.
    pub fn set_focused_room(&self, room_id: Option<&str>) -> Result<(), NotificationError> {
        let normalized = match room_id {
            None => None,
            Some(raw) => {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    None
                } else if !trimmed.starts_with('!') {
                    return Err(NotificationError::Invalid {
                        diagnostic_id: "v-notify.invalid-room-id",
                    });
                } else {
                    Some(trimmed.to_owned())
                }
            }
        };
        let mut index = self.index.lock().map_err(|_| NotificationError::Invalid {
            diagnostic_id: "v-notify.owner-poisoned",
        })?;
        index.set_focused_room(normalized);
        Ok(())
    }

    pub fn focused_room(&self) -> Option<String> {
        self.index
            .lock()
            .ok()
            .and_then(|index| index.focused_room().map(str::to_owned))
    }

    /// Resolve one renderer observation through the SDK and decide it.
    ///
    /// Message kinds require an event id and a bound client: Core loads the
    /// event, compares its sender with the session, and reads the
    /// SDK-evaluated push actions. Kinds without a timeline event surface
    /// through the same focus/dedup index without push evaluation. No lock
    /// is held across the SDK awaits.
    pub async fn decide_observed(
        &self,
        request: NativeNotificationDecideRequest,
    ) -> Result<NotificationDecisionReadback, NotificationError> {
        let kind = NotificationDecisionKind::parse(&request.kind)
            .map_err(|diagnostic_id| NotificationError::Invalid { diagnostic_id })?;
        let observed = match kind {
            NotificationDecisionKind::Message => {
                self.observe_message_event(&request.room_id, request.event_id.as_deref())
                    .await?
            }
            NotificationDecisionKind::Invite
            | NotificationDecisionKind::AgentApproval
            | NotificationDecisionKind::LaterReminder => ObservedEvent {
                is_own_event: false,
                is_encrypted: false,
                push: NotificationPushEvaluation::surface(),
            },
        };
        self.decide(NotificationDecisionInput {
            room_id: request.room_id,
            event_id: request.event_id,
            kind,
            title: request.title,
            body: request.body,
            route: request.route,
            suppress_if_focused_room: request.suppress_if_focused_room,
            is_encrypted: observed.is_encrypted,
            push: observed.push,
            is_own_event: observed.is_own_event,
        })
    }

    async fn observe_message_event(
        &self,
        room_id: &str,
        event_id: Option<&str>,
    ) -> Result<ObservedEvent, NotificationError> {
        let client = self.client.as_ref().ok_or(NotificationError::Invalid {
            diagnostic_id: "v-notify.no-client",
        })?;
        let event_id = event_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or(NotificationError::Invalid {
                diagnostic_id: "v-notify.event-id-required",
            })?;
        let room_id = RoomId::parse(room_id.trim()).map_err(|_| NotificationError::Invalid {
            diagnostic_id: "v-notify.invalid-room-id",
        })?;
        let event_id = EventId::parse(event_id).map_err(|_| NotificationError::Invalid {
            diagnostic_id: "v-notify.invalid-event-id",
        })?;
        let room = client
            .get_room(&room_id)
            .ok_or(NotificationError::Invalid {
                diagnostic_id: "v-notify.room-unknown",
            })?;
        // Synced events are served from the SDK event cache. The `/event`
        // fallback covers an observation that raced ahead of the cache and is
        // bounded to one short attempt; the renderer resubmits on failure.
        let event = room
            .load_or_fetch_event(
                &event_id,
                Some(
                    RequestConfig::new()
                        .timeout(EVENT_FETCH_TIMEOUT)
                        .disable_retry(),
                ),
            )
            .await
            .map_err(|_| NotificationError::Invalid {
                diagnostic_id: "v-notify.event-unavailable",
            })?;
        let is_own_event = event
            .sender()
            .is_some_and(|sender| sender.as_str() == self.user_id);
        // Sync stores computed actions as `Some` (possibly empty). `None`
        // means they were never computed for this event (for example a
        // `/event` fetch before room state settled), so recompute once with
        // the current room context instead of guessing.
        let actions = match event.push_actions() {
            Some(actions) => actions.to_vec(),
            None => room
                .event_push_actions(event.raw())
                .await
                .ok()
                .flatten()
                .ok_or(NotificationError::Invalid {
                    diagnostic_id: "v-notify.push-context-unavailable",
                })?,
        };
        Ok(ObservedEvent {
            is_own_event,
            is_encrypted: room.encryption_state().is_encrypted(),
            push: NotificationPushEvaluation::from_actions(&actions),
        })
    }

    /// Apply the closed policy table and, on show, enqueue through the
    /// dedup/focus/cap index. Returns the exact readback the bridge emits.
    pub fn decide(
        &self,
        input: NotificationDecisionInput,
    ) -> Result<NotificationDecisionReadback, NotificationError> {
        if input.is_own_event {
            return Ok(suppressed(NotificationSuppressReason::OwnEvent));
        }
        if !input.push.notify {
            return Ok(suppressed(NotificationSuppressReason::PushRulesNoNotify));
        }

        let push = input.push;
        let event_id_for_dedup = input.event_id.clone();
        let candidate = NotificationCandidate {
            candidate_id: String::new(),
            room_id: input.room_id,
            event_id: input.event_id,
            kind: input.kind.as_dto(),
            title: truncate_chars(&input.title, NOTIFICATION_TITLE_MAX_CHARS),
            body: truncate_chars(&input.body, NOTIFICATION_BODY_MAX_CHARS),
            route: input.route.and_then(sanitize_route),
            suppress_if_focused_room: input.suppress_if_focused_room,
            is_encrypted: input.is_encrypted,
        };

        let mut index = self.index.lock().map_err(|_| NotificationError::Invalid {
            diagnostic_id: "v-notify.owner-poisoned",
        })?;
        // Classify the suppression exactly before enqueue: duplicates are
        // retained across dismiss within the recent-event bound; focus is
        // transient. `enqueue` checks focus first, so pre-read both signals
        // without leaking identifiers.
        let duplicate = event_id_for_dedup
            .as_deref()
            .is_some_and(|event_id| index.is_duplicate(&candidate.room_id, event_id));
        let focused = candidate.suppress_if_focused_room
            && index
                .focused_room()
                .is_some_and(|focused| focused == candidate.room_id);
        match index.enqueue(candidate) {
            Ok(Some(id)) => {
                let stored = index.get(&id).cloned().ok_or(NotificationError::Invalid {
                    diagnostic_id: "v-notify.decision-readback-missing",
                })?;
                Ok(NotificationDecisionReadback {
                    decision: "show".to_owned(),
                    reason: None,
                    candidate: Some(stored),
                    highlight: push.highlight,
                    sound: push.sound,
                })
            }
            Ok(None) => Ok(suppressed(if duplicate {
                NotificationSuppressReason::DuplicateEvent
            } else if focused {
                NotificationSuppressReason::FocusedRoom
            } else {
                // Focus raced or the cap path suppressed; report focus as the
                // transient reason rather than inventing a new code.
                NotificationSuppressReason::FocusedRoom
            })),
            Err(error) => Err(error),
        }
    }

    pub fn list_pending(&self) -> Result<Vec<NotificationCandidate>, NotificationError> {
        let index = self.index.lock().map_err(|_| NotificationError::Invalid {
            diagnostic_id: "v-notify.owner-poisoned",
        })?;
        Ok(index.list_pending().into_iter().cloned().collect())
    }

    /// Release a pending candidate and record the platform's delivery
    /// receipt. Returns whether the candidate was still pending; the ledger
    /// advances only in that case. A `failed` receipt does not re-arm the
    /// event: `(room_id, event_id)` dedup is retained and no retry is
    /// scheduled, so a flapping OS cannot re-notify the same message.
    pub fn dismiss(
        &self,
        candidate_id: &str,
        outcome: Option<NotificationDeliveryOutcome>,
    ) -> Result<bool, NotificationError> {
        if candidate_id.trim().is_empty() {
            return Err(NotificationError::Invalid {
                diagnostic_id: "v-notify.invalid-candidate-id",
            });
        }
        let mut index = self.index.lock().map_err(|_| NotificationError::Invalid {
            diagnostic_id: "v-notify.owner-poisoned",
        })?;
        let dismissed = index.dismiss(candidate_id);
        if dismissed {
            let mut ledger = self
                .delivery
                .lock()
                .map_err(|_| NotificationError::Invalid {
                    diagnostic_id: "v-notify.owner-poisoned",
                })?;
            ledger.record(outcome);
        }
        Ok(dismissed)
    }

    /// Identifier-free delivery receipt counts for the bound session.
    pub fn delivery_ledger(&self) -> Result<NotificationDeliveryLedger, NotificationError> {
        let ledger = self
            .delivery
            .lock()
            .map_err(|_| NotificationError::Invalid {
                diagnostic_id: "v-notify.owner-poisoned",
            })?;
        Ok(*ledger)
    }

    pub fn pending_count(&self) -> Result<usize, NotificationError> {
        let index = self.index.lock().map_err(|_| NotificationError::Invalid {
            diagnostic_id: "v-notify.owner-poisoned",
        })?;
        Ok(index.len())
    }

    /// Wipe pending state on logout / account switch. Generation advances;
    /// focus clears with the queue.
    pub fn retire_generation(&self, new_generation: u64) {
        if let Ok(mut index) = self.index.lock() {
            index.retire_generation(new_generation);
        }
        if let Ok(mut ledger) = self.delivery.lock() {
            *ledger = NotificationDeliveryLedger::default();
        }
    }
}

fn suppressed(reason: NotificationSuppressReason) -> NotificationDecisionReadback {
    NotificationDecisionReadback {
        decision: "suppress".to_owned(),
        reason: Some(reason.as_str().to_owned()),
        candidate: None,
        highlight: false,
        sound: false,
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_owned();
    }
    trimmed.chars().take(max_chars).collect()
}

/// Internal deep-link routes only (`/` or `#` prefix, no control or
/// whitespace characters). Malformed routes degrade to no route so the
/// notification still delivers; delivery sanitization stays authoritative.
fn sanitize_route(route: String) -> Option<String> {
    let trimmed = route.trim();
    if trimmed.is_empty() || trimmed.chars().count() > NOTIFICATION_ROUTE_MAX_CHARS {
        return None;
    }
    let internal = trimmed.starts_with('/') || trimmed.starts_with('#');
    let clean = !trimmed
        .chars()
        .any(|ch| ch.is_control() || ch.is_whitespace());
    (internal && clean).then(|| trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_sdk::ruma::push::Tweak;

    const NOTIFY: NotificationPushEvaluation = NotificationPushEvaluation {
        notify: true,
        highlight: false,
        sound: false,
    };
    const NO_NOTIFY: NotificationPushEvaluation = NotificationPushEvaluation {
        notify: false,
        highlight: false,
        sound: false,
    };
    const HIGHLIGHT: NotificationPushEvaluation = NotificationPushEvaluation {
        notify: true,
        highlight: true,
        sound: true,
    };

    fn input(
        room: &str,
        event: Option<&str>,
        kind: NotificationDecisionKind,
        push: NotificationPushEvaluation,
        own: bool,
    ) -> NotificationDecisionInput {
        NotificationDecisionInput {
            room_id: room.into(),
            event_id: event.map(Into::into),
            kind,
            title: "Room".into(),
            body: "New message".into(),
            route: Some("/home/room/!r:example.org".into()),
            suppress_if_focused_room: true,
            is_encrypted: false,
            push,
            is_own_event: own,
        }
    }

    fn owner() -> NativeNotificationDecisionOwner {
        NativeNotificationDecisionOwner {
            session_generation: 7,
            user_id: "@u:example.org".into(),
            device_id: "DEV".into(),
            homeserver_url: "https://example.org".into(),
            client: None,
            index: Mutex::new(NotificationIndex::new(7)),
            delivery: Mutex::new(NotificationDeliveryLedger::default()),
        }
    }

    #[test]
    fn kind_vocabulary_is_closed() {
        assert_eq!(
            NotificationDecisionKind::parse("agent_approval").unwrap(),
            NotificationDecisionKind::AgentApproval
        );
        assert_eq!(
            NotificationDecisionKind::parse("nope").unwrap_err(),
            "v-notify.invalid-kind"
        );
    }

    #[test]
    fn push_evaluation_folds_sdk_actions() {
        assert_eq!(
            NotificationPushEvaluation::from_actions(&[]),
            NO_NOTIFY,
            "no actions means the rules decided not to notify"
        );
        assert_eq!(
            NotificationPushEvaluation::from_actions(&[Action::Notify]),
            NOTIFY
        );
        assert_eq!(
            NotificationPushEvaluation::from_actions(&[
                Action::Notify,
                Action::SetTweak(Tweak::Highlight(true.into())),
                Action::SetTweak(Tweak::Sound("default".into())),
            ]),
            HIGHLIGHT
        );
        assert_eq!(
            NotificationPushEvaluation::from_actions(&[Action::SetTweak(Tweak::Highlight(
                true.into()
            ))]),
            NotificationPushEvaluation {
                notify: false,
                highlight: true,
                sound: false,
            },
            "a highlight tweak without notify still does not notify"
        );
        assert_eq!(
            NotificationPushEvaluation::from_actions(&[
                Action::Notify,
                Action::SetTweak(Tweak::Highlight(false.into())),
            ]),
            NOTIFY,
            "an explicit highlight=false tweak is not a highlight"
        );
    }

    #[test]
    fn own_events_never_notify() {
        let owner = owner();
        let readback = owner
            .decide(input(
                "!r:example.org",
                Some("$e1"),
                NotificationDecisionKind::Message,
                HIGHLIGHT,
                true,
            ))
            .unwrap();
        assert_eq!(readback.decision, "suppress");
        assert_eq!(readback.reason.as_deref(), Some("own-event"));
        assert!(readback.candidate.is_none());
        assert!(!readback.highlight && !readback.sound);
    }

    #[test]
    fn push_rules_without_notify_suppress_messages() {
        let owner = owner();
        let message = owner
            .decide(input(
                "!r:example.org",
                Some("$m1"),
                NotificationDecisionKind::Message,
                NO_NOTIFY,
                false,
            ))
            .unwrap();
        assert_eq!(message.decision, "suppress");
        assert_eq!(message.reason.as_deref(), Some("push-rules-no-notify"));
        assert!(message.candidate.is_none());
    }

    #[test]
    fn shown_messages_echo_sdk_highlight_and_sound() {
        let owner = owner();
        let plain = owner
            .decide(input(
                "!r:example.org",
                Some("$p1"),
                NotificationDecisionKind::Message,
                NOTIFY,
                false,
            ))
            .unwrap();
        assert_eq!(plain.decision, "show");
        assert!(!plain.highlight);
        assert!(!plain.sound);

        let highlighted = owner
            .decide(input(
                "!r:example.org",
                Some("$p2"),
                NotificationDecisionKind::Message,
                HIGHLIGHT,
                false,
            ))
            .unwrap();
        assert_eq!(highlighted.decision, "show");
        assert!(highlighted.highlight);
        assert!(highlighted.sound);
    }

    #[test]
    fn focused_room_suppresses_and_dedup_holds() {
        let owner = owner();
        owner.set_focused_room(Some("!r:example.org")).unwrap();
        let suppressed = owner
            .decide(input(
                "!r:example.org",
                Some("$e1"),
                NotificationDecisionKind::Message,
                NOTIFY,
                false,
            ))
            .unwrap();
        assert_eq!(suppressed.decision, "suppress");
        assert_eq!(suppressed.reason.as_deref(), Some("focused-room"));

        owner.set_focused_room(None).unwrap();
        let first = owner
            .decide(input(
                "!r:example.org",
                Some("$e2"),
                NotificationDecisionKind::Message,
                NOTIFY,
                false,
            ))
            .unwrap();
        assert_eq!(first.decision, "show");
        let candidate_id = first.candidate.clone().unwrap().candidate_id;
        assert!(candidate_id.starts_with("notif-"));

        // Same (room, event) never notifies twice, even across dismiss.
        let second = owner
            .decide(input(
                "!r:example.org",
                Some("$e2"),
                NotificationDecisionKind::Message,
                NOTIFY,
                false,
            ))
            .unwrap();
        assert_eq!(second.decision, "suppress");
        assert_eq!(second.reason.as_deref(), Some("duplicate-event"));
        assert!(owner.dismiss(&candidate_id, None).unwrap());
        assert_eq!(owner.pending_count().unwrap(), 0);
    }

    #[test]
    fn delivery_receipts_count_once_per_released_candidate() {
        let owner = owner();
        let mut candidates = Vec::new();
        for event in ["$d1", "$d2", "$d3"] {
            let shown = owner
                .decide(input(
                    "!r:example.org",
                    Some(event),
                    NotificationDecisionKind::Message,
                    NOTIFY,
                    false,
                ))
                .unwrap();
            candidates.push(shown.candidate.unwrap().candidate_id);
        }
        assert_eq!(
            owner.delivery_ledger().unwrap(),
            NotificationDeliveryLedger::default()
        );

        assert!(owner
            .dismiss(&candidates[0], Some(NotificationDeliveryOutcome::Delivered))
            .unwrap());
        assert!(owner
            .dismiss(&candidates[1], Some(NotificationDeliveryOutcome::Failed))
            .unwrap());
        assert!(owner.dismiss(&candidates[2], None).unwrap());
        assert_eq!(
            owner.delivery_ledger().unwrap(),
            NotificationDeliveryLedger {
                delivered: 1,
                failed: 1,
                unreported: 1,
            }
        );

        // A repeated or unknown acknowledgement releases nothing and cannot
        // inflate the ledger.
        assert!(!owner
            .dismiss(&candidates[1], Some(NotificationDeliveryOutcome::Failed))
            .unwrap());
        assert!(!owner
            .dismiss(
                "notif-unknown",
                Some(NotificationDeliveryOutcome::Delivered)
            )
            .unwrap());
        assert_eq!(owner.delivery_ledger().unwrap().failed, 1);
        assert_eq!(owner.delivery_ledger().unwrap().delivered, 1);

        // A failed OS delivery does not re-arm the event: dedup holds and no
        // retry is scheduled, so the same message never notifies twice.
        let again = owner
            .decide(input(
                "!r:example.org",
                Some("$d2"),
                NotificationDecisionKind::Message,
                NOTIFY,
                false,
            ))
            .unwrap();
        assert_eq!(again.reason.as_deref(), Some("duplicate-event"));
        assert_eq!(owner.pending_count().unwrap(), 0);

        owner.retire_generation(8);
        assert_eq!(
            owner.delivery_ledger().unwrap(),
            NotificationDeliveryLedger::default()
        );
    }

    #[test]
    fn dismiss_wire_outcome_is_closed_and_optional() {
        let plain: NativeNotificationDismissRequest =
            serde_json::from_value(serde_json::json!({ "candidateId": "notif-1" })).unwrap();
        assert_eq!(plain.outcome, None);
        let failed: NativeNotificationDismissRequest = serde_json::from_value(
            serde_json::json!({ "candidateId": "notif-1", "outcome": "failed" }),
        )
        .unwrap();
        assert_eq!(failed.outcome, Some(NotificationDeliveryOutcome::Failed));
        assert_eq!(
            serde_json::to_value(&failed).unwrap(),
            serde_json::json!({ "candidateId": "notif-1", "outcome": "failed" })
        );
        assert!(serde_json::from_value::<NativeNotificationDismissRequest>(
            serde_json::json!({ "candidateId": "notif-1", "outcome": "retry" })
        )
        .is_err());
        assert!(serde_json::from_value::<NativeNotificationDismissRequest>(
            serde_json::json!({ "candidateId": "notif-1", "delivered": true })
        )
        .is_err());
        assert_eq!(NotificationDeliveryOutcome::Delivered.as_str(), "delivered");
    }

    #[test]
    fn malformed_focus_is_fail_closed_and_titles_truncate() {
        let owner = owner();
        assert!(owner.set_focused_room(Some("not-a-room")).is_err());
        assert!(owner.focused_room().is_none());

        let long = "t".repeat(NOTIFICATION_TITLE_MAX_CHARS + 50);
        let mut entry = input(
            "!r:example.org",
            Some("$e9"),
            NotificationDecisionKind::Message,
            NOTIFY,
            false,
        );
        entry.title = long;
        let readback = owner.decide(entry).unwrap();
        assert_eq!(readback.decision, "show");
        assert_eq!(
            readback.candidate.unwrap().title.chars().count(),
            NOTIFICATION_TITLE_MAX_CHARS
        );
    }

    #[test]
    fn session_binding_compares_exact_identity() {
        let owner = owner();
        assert!(owner.owns_session("@u:example.org", "DEV", "https://example.org/"));
        assert!(!owner.owns_session("@u:example.org", "OTHER", "https://example.org"));
        assert!(!owner.owns_session("@other:example.org", "DEV", "https://example.org"));
    }

    #[test]
    fn malformed_routes_degrade_to_no_route() {
        assert_eq!(
            sanitize_route("https://evil.example.com".into()),
            None,
            "external URLs never become deep links"
        );
        assert_eq!(sanitize_route("room/abc".into()), None);
        assert_eq!(sanitize_route("   ".into()), None);
        assert_eq!(
            sanitize_route("/home/room/!r:example.org".into()),
            Some("/home/room/!r:example.org".into())
        );
        assert_eq!(
            sanitize_route("#/room/abc".into()),
            Some("#/room/abc".into())
        );

        // A malformed route still delivers the notification without a link.
        let owner = owner();
        let mut entry = input(
            "!r:example.org",
            Some("$e-route"),
            NotificationDecisionKind::Message,
            NOTIFY,
            false,
        );
        entry.route = Some("https://evil.example.com".into());
        let readback = owner.decide(entry).unwrap();
        assert_eq!(readback.decision, "show");
        assert_eq!(readback.candidate.unwrap().route, None);
    }

    #[test]
    fn wire_request_rejects_renderer_supplied_policy_fields() {
        for legacy in ["roomMode", "highlight", "isOwnEvent", "isEncrypted"] {
            let payload = serde_json::json!({
                "roomId": "!r:example.org",
                "eventId": "$e1",
                "kind": "message",
                "title": "Room",
                "body": "New message",
                legacy: true,
            });
            assert!(
                serde_json::from_value::<NativeNotificationDecideRequest>(payload).is_err(),
                "renderer must not be able to supply `{legacy}`"
            );
        }
        let minimal =
            serde_json::from_value::<NativeNotificationDecideRequest>(serde_json::json!({
                "roomId": "!r:example.org",
                "eventId": "$e1",
                "kind": "message",
                "title": "Room",
                "body": "New message",
            }))
            .unwrap();
        assert!(minimal.suppress_if_focused_room);
        assert_eq!(minimal.route, None);
    }

    #[tokio::test]
    async fn observed_messages_fail_closed_without_a_bound_client() {
        let owner = owner();
        let error = owner
            .decide_observed(NativeNotificationDecideRequest {
                room_id: "!r:example.org".into(),
                event_id: Some("$e1".into()),
                kind: "message".into(),
                title: "Room".into(),
                body: "New message".into(),
                route: None,
                suppress_if_focused_room: true,
            })
            .await
            .unwrap_err();
        assert_eq!(error.diagnostic_id(), "v-notify.no-client");

        // Kinds without a timeline event surface without SDK evaluation.
        let invite = owner
            .decide_observed(NativeNotificationDecideRequest {
                room_id: "!r:example.org".into(),
                event_id: Some("$i1".into()),
                kind: "invite".into(),
                title: "Invitation".into(),
                body: "You have 1 new invitation request.".into(),
                route: None,
                suppress_if_focused_room: true,
            })
            .await
            .unwrap();
        assert_eq!(invite.decision, "show");
        assert!(!invite.highlight && !invite.sound);

        let bad_kind = owner
            .decide_observed(NativeNotificationDecideRequest {
                room_id: "!r:example.org".into(),
                event_id: None,
                kind: "loud".into(),
                title: "Room".into(),
                body: "New message".into(),
                route: None,
                suppress_if_focused_room: true,
            })
            .await
            .unwrap_err();
        assert_eq!(bad_kind.diagnostic_id(), "v-notify.invalid-kind");
    }
}
