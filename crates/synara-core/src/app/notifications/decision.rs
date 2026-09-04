//! Typed desktop notification decision owner (A9 follow-on).
//!
//! Core owns the suppress/show policy; platforms own delivery, sound
//! preference, Do-Not-Disturb, and OS presentation. This owner wraps the
//! [`NotificationIndex`] harness with a closed decision table fed by explicit
//! platform observations:
//!
//! - `room_mode`: the shell-resolved per-room mode vocabulary
//!   (`all` / `mentions` / `mute` / `default`). The shell resolves it through
//!   the existing `matrix_room_notification_snapshot` / push-rules snapshots
//!   and passes the closed string; Core never accepts a TS-computed boolean.
//! - `highlight`: whether the event is a mention/keyword highlight. The shell
//!   passes the Matrix highlight signal it observed; Core owns what that
//!   signal means for each mode.
//! - `is_own_event`: whether the sender is the local user. Own events never
//!   notify, regardless of mode.
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

/// Closed per-room mode vocabulary for the decision table. Resolved by the
/// shell through the existing push-rule/room-notification snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationRoomMode {
    All,
    Mentions,
    Mute,
    Default,
}

impl NotificationRoomMode {
    pub fn parse(mode: &str) -> Result<Self, &'static str> {
        match mode.trim() {
            "all" => Ok(Self::All),
            "mentions" => Ok(Self::Mentions),
            "mute" => Ok(Self::Mute),
            "default" => Ok(Self::Default),
            _ => Err("v-notify.invalid-room-mode"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Mentions => "mentions",
            Self::Mute => "mute",
            Self::Default => "default",
        }
    }
}

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
    MutedRoom,
    MentionsOnlyWithoutHighlight,
    FocusedRoom,
    DuplicateEvent,
}

impl NotificationSuppressReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OwnEvent => "own-event",
            Self::MutedRoom => "muted-room",
            Self::MentionsOnlyWithoutHighlight => "mentions-only-without-highlight",
            Self::FocusedRoom => "focused-room",
            Self::DuplicateEvent => "duplicate-event",
        }
    }
}

/// Platform-observed facts for one decision. Title/body are pre-filtered
/// product strings; `room_mode`/`highlight`/`is_own_event` are the closed
/// observations Core decides over.
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
    pub room_mode: NotificationRoomMode,
    #[serde(default)]
    pub highlight: bool,
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
/// Title/body are pre-filtered product strings (room names, usernames, fixed
/// summaries) — never raw ciphertext or event dumps. `room_mode` is the closed
/// `all` / `mentions` / `mute` / `default` vocabulary resolved by the shell
/// through the existing push-rule/room-notification snapshots; `highlight`
/// carries the observed mention signal; `is_own_event` carries the observed
/// sender-is-self fact. Unknown keys are rejected so the decide route cannot
/// grow credential, token, path, or byte fields.
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
    #[serde(default)]
    pub is_encrypted: bool,
    pub room_mode: String,
    #[serde(default)]
    pub highlight: bool,
    #[serde(default)]
    pub is_own_event: bool,
}

/// React/Tauri wire request for `matrix_notification_dismiss`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeNotificationDismissRequest {
    pub candidate_id: String,
}

/// Exact readback for `matrix_notification_decide`. `decision` is the closed
/// `show` / `suppress` vocabulary; `reason` is set only on suppress;
/// `candidate` is set only on show.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDecisionReadback {
    pub decision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate: Option<NotificationCandidate>,
}

/// Account-bound decision owner. Binds the exact authenticated session at
/// attach (identity never serialized); the wrapped index owns focus, dedup,
/// and the pending cap.
pub struct NativeNotificationDecisionOwner {
    session_generation: u64,
    user_id: String,
    device_id: String,
    homeserver_url: String,
    index: Mutex<NotificationIndex>,
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
            index: Mutex::new(NotificationIndex::new(session_generation)),
        })
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    /// Test-only owner without a Matrix client. Production shells use
    /// [`Self::new`], which binds the exact authenticated session.
    #[cfg(test)]
    pub fn for_tests(session_generation: u64) -> Self {
        Self {
            session_generation,
            user_id: "@test:example.org".into(),
            device_id: "TESTDEVICE".into(),
            homeserver_url: "https://example.org".into(),
            index: Mutex::new(NotificationIndex::new(session_generation)),
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

    /// Apply the closed policy table and, on show, enqueue through the
    /// dedup/focus/cap index. Returns the exact readback the bridge emits.
    pub fn decide(
        &self,
        input: NotificationDecisionInput,
    ) -> Result<NotificationDecisionReadback, NotificationError> {
        if input.is_own_event {
            return Ok(suppressed(NotificationSuppressReason::OwnEvent));
        }
        // Mute suppresses timeline messages. Invites, agent approvals, and
        // Later reminders are explicit user commitments outside the muted
        // room's message policy and still surface.
        if input.room_mode == NotificationRoomMode::Mute
            && input.kind == NotificationDecisionKind::Message
        {
            return Ok(suppressed(NotificationSuppressReason::MutedRoom));
        }
        if input.room_mode == NotificationRoomMode::Mentions
            && input.kind == NotificationDecisionKind::Message
            && !input.highlight
        {
            return Ok(suppressed(
                NotificationSuppressReason::MentionsOnlyWithoutHighlight,
            ));
        }

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
        // permanent (survive dismiss), focus is transient. `enqueue` checks
        // focus first, so pre-read both signals without leaking identifiers.
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

    pub fn dismiss(&self, candidate_id: &str) -> Result<bool, NotificationError> {
        if candidate_id.trim().is_empty() {
            return Err(NotificationError::Invalid {
                diagnostic_id: "v-notify.invalid-candidate-id",
            });
        }
        let mut index = self.index.lock().map_err(|_| NotificationError::Invalid {
            diagnostic_id: "v-notify.owner-poisoned",
        })?;
        Ok(index.dismiss(candidate_id))
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
    }
}

fn suppressed(reason: NotificationSuppressReason) -> NotificationDecisionReadback {
    NotificationDecisionReadback {
        decision: "suppress".to_owned(),
        reason: Some(reason.as_str().to_owned()),
        candidate: None,
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

    fn input(
        room: &str,
        event: Option<&str>,
        kind: NotificationDecisionKind,
        mode: NotificationRoomMode,
        highlight: bool,
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
            room_mode: mode,
            highlight,
            is_own_event: own,
        }
    }

    fn owner() -> NativeNotificationDecisionOwner {
        NativeNotificationDecisionOwner {
            session_generation: 7,
            user_id: "@u:example.org".into(),
            device_id: "DEV".into(),
            homeserver_url: "https://example.org".into(),
            index: Mutex::new(NotificationIndex::new(7)),
        }
    }

    #[test]
    fn mode_vocabulary_is_closed() {
        assert_eq!(
            NotificationRoomMode::parse("all").unwrap(),
            NotificationRoomMode::All
        );
        assert_eq!(
            NotificationRoomMode::parse("mentions").unwrap(),
            NotificationRoomMode::Mentions
        );
        assert_eq!(
            NotificationRoomMode::parse("mute").unwrap(),
            NotificationRoomMode::Mute
        );
        assert_eq!(
            NotificationRoomMode::parse("default").unwrap(),
            NotificationRoomMode::Default
        );
        assert_eq!(
            NotificationRoomMode::parse("loud").unwrap_err(),
            "v-notify.invalid-room-mode"
        );
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
    fn own_events_never_notify() {
        let owner = owner();
        let readback = owner
            .decide(input(
                "!r:example.org",
                Some("$e1"),
                NotificationDecisionKind::Message,
                NotificationRoomMode::All,
                true,
                true,
            ))
            .unwrap();
        assert_eq!(readback.decision, "suppress");
        assert_eq!(readback.reason.as_deref(), Some("own-event"));
        assert!(readback.candidate.is_none());
    }

    #[test]
    fn mute_suppresses_messages_but_not_invites_or_approvals() {
        let owner = owner();
        let message = owner
            .decide(input(
                "!r:example.org",
                Some("$m1"),
                NotificationDecisionKind::Message,
                NotificationRoomMode::Mute,
                true,
                false,
            ))
            .unwrap();
        assert_eq!(message.decision, "suppress");
        assert_eq!(message.reason.as_deref(), Some("muted-room"));

        let invite = owner
            .decide(input(
                "!r:example.org",
                Some("$i1"),
                NotificationDecisionKind::Invite,
                NotificationRoomMode::Mute,
                false,
                false,
            ))
            .unwrap();
        assert_eq!(invite.decision, "show");

        let approval = owner
            .decide(input(
                "!r:example.org",
                Some("$a1"),
                NotificationDecisionKind::AgentApproval,
                NotificationRoomMode::Mute,
                false,
                false,
            ))
            .unwrap();
        assert_eq!(approval.decision, "show");
    }

    #[test]
    fn mentions_only_requires_highlight_for_messages() {
        let owner = owner();
        let plain = owner
            .decide(input(
                "!r:example.org",
                Some("$p1"),
                NotificationDecisionKind::Message,
                NotificationRoomMode::Mentions,
                false,
                false,
            ))
            .unwrap();
        assert_eq!(plain.decision, "suppress");
        assert_eq!(
            plain.reason.as_deref(),
            Some("mentions-only-without-highlight")
        );

        let highlighted = owner
            .decide(input(
                "!r:example.org",
                Some("$p2"),
                NotificationDecisionKind::Message,
                NotificationRoomMode::Mentions,
                true,
                false,
            ))
            .unwrap();
        assert_eq!(highlighted.decision, "show");
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
                NotificationRoomMode::All,
                false,
                false,
            ))
            .unwrap();
        assert_eq!(suppressed.decision, "suppress");

        owner.set_focused_room(None).unwrap();
        let first = owner
            .decide(input(
                "!r:example.org",
                Some("$e2"),
                NotificationDecisionKind::Message,
                NotificationRoomMode::All,
                false,
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
                NotificationRoomMode::All,
                false,
                false,
            ))
            .unwrap();
        assert_eq!(second.decision, "suppress");
        assert!(owner.dismiss(&candidate_id).unwrap());
        assert_eq!(owner.pending_count().unwrap(), 0);
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
            NotificationRoomMode::All,
            false,
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
            NotificationRoomMode::All,
            false,
            false,
        );
        entry.route = Some("https://evil.example.com".into());
        let readback = owner.decide(entry).unwrap();
        assert_eq!(readback.decision, "show");
        assert_eq!(readback.candidate.unwrap().route, None);
    }
}
