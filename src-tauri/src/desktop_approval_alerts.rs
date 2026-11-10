//! Approval alerts close themselves once the prompt is no longer pending.
//!
//! A banner's Approve/Deny buttons are stale as soon as the prompt is decided
//! on any device, settled by a Hermes edit, redacted, or expired. The renderer
//! cannot be relied on to notice: a window closed to the tray or on another
//! workspace may have its timers throttled, and its inbox drops a prompt that
//! Hermes edited to its outcome instead of reporting it decided. So the native
//! side remembers each approval alert it posted and asks Core about it on a
//! short timer until the prompt is settled or expired, then withdraws it.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use synara_core::app::agent_approvals::{
    classify_agent_approval, is_eligible_agent_approval_prompt, AGENT_APPROVAL_TTL_MS,
};
use synara_core::app::timeline::NativeTimelineItem;
use tauri::{AppHandle, Manager, Runtime};

use super::{
    active_session, dismiss_notifications_now, is_time_sensitive_agent_approval,
    log_notification_error, sanitize_dismiss_key, DesktopNotificationPayload,
};

/// More simultaneous approvals than anyone reviews; the oldest stops being
/// tracked and falls back to the daemon's own timeout.
const MAX_TRACKED_APPROVAL_ALERTS: usize = 32;
/// How often tracked alerts are re-read. Core answers from a timeline that
/// sync keeps current, so this is a cheap in-memory read per alert.
const APPROVAL_ALERT_CHECK_INTERVAL: Duration = Duration::from_secs(10);
/// One slow read must not hold up the other alerts' expiry.
const APPROVAL_ALERT_READ_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ApprovalAlertKey {
    pub room_id: String,
    pub event_id: String,
}

impl ApprovalAlertKey {
    fn dismiss_key(&self) -> Option<String> {
        sanitize_dismiss_key(&format!("event:{}", self.event_id))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TrackedApprovalAlert {
    pub session_generation: Option<u64>,
    pub expires_at_ms: u64,
}

/// What Core says about a tracked prompt right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ApprovalAlertState {
    Pending {
        expires_at_ms: u64,
    },
    /// Decided by this account anywhere, expired, edited to its outcome,
    /// redacted, or posted for a session that has ended.
    Settled,
    /// Not readable yet (offline, still decrypting); expiry still applies.
    Unknown,
}

/// The state of a prompt from Core's readback of it. Core's own classifier
/// decides; anything that is no longer an approval prompt from someone else
/// (Hermes edited it to the outcome, or it was redacted) is settled.
pub(crate) fn approval_alert_state(
    item: &NativeTimelineItem,
    own_user: &str,
    now_ms: u64,
) -> ApprovalAlertState {
    if own_user.is_empty() || item.decryption_state.is_some() {
        return ApprovalAlertState::Unknown;
    }
    if !is_eligible_agent_approval_prompt(&item.body, &item.sender, own_user) {
        return ApprovalAlertState::Settled;
    }
    match classify_agent_approval(
        &item.body,
        &item.sender,
        own_user,
        item.origin_server_ts,
        now_ms,
        item.reactions
            .iter()
            .map(|reaction| (reaction.key.as_str(), reaction.me)),
    ) {
        Ok(classification) if classification.decided || classification.expired => {
            ApprovalAlertState::Settled
        }
        Ok(classification) => ApprovalAlertState::Pending {
            expires_at_ms: classification.expires_at,
        },
        Err(_) => ApprovalAlertState::Unknown,
    }
}

#[derive(Debug, Default)]
pub(crate) struct ApprovalAlertTracker {
    alerts: HashMap<ApprovalAlertKey, TrackedApprovalAlert>,
    order: VecDeque<ApprovalAlertKey>,
    checking: bool,
}

impl ApprovalAlertTracker {
    /// Remember a posted alert. Returns true when no check loop is running,
    /// so the caller starts one.
    pub(crate) fn track(&mut self, key: ApprovalAlertKey, alert: TrackedApprovalAlert) -> bool {
        self.forget(&key);
        while self.alerts.len() >= MAX_TRACKED_APPROVAL_ALERTS {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            self.alerts.remove(&oldest);
        }
        self.alerts.insert(key.clone(), alert);
        self.order.push_back(key);
        !std::mem::replace(&mut self.checking, true)
    }

    pub(crate) fn forget(&mut self, key: &ApprovalAlertKey) {
        if self.alerts.remove(key).is_some() {
            self.order.retain(|candidate| candidate != key);
        }
    }

    pub(crate) fn tracks_event(&self, event_id: &str) -> bool {
        self.alerts.keys().any(|key| key.event_id == event_id)
    }

    /// The alerts to check, or `None` (and the loop marked stopped) when
    /// nothing is left, so a later `track` starts a fresh loop.
    pub(crate) fn due_for_check(
        &mut self,
    ) -> Option<Vec<(ApprovalAlertKey, TrackedApprovalAlert)>> {
        if self.alerts.is_empty() {
            self.checking = false;
            return None;
        }
        Some(
            self.order
                .iter()
                .filter_map(|key| self.alerts.get(key).map(|alert| (key.clone(), *alert)))
                .collect(),
        )
    }

    /// Record what Core reported. Returns true when the alert must be
    /// withdrawn now (settled, or past its expiry); it is then forgotten.
    pub(crate) fn apply(
        &mut self,
        key: &ApprovalAlertKey,
        state: ApprovalAlertState,
        now_ms: u64,
    ) -> bool {
        let Some(alert) = self.alerts.get_mut(key) else {
            return false;
        };
        let withdraw = match state {
            ApprovalAlertState::Settled => true,
            ApprovalAlertState::Pending { expires_at_ms } => {
                // The prompt's own expiry is never later than the bound set
                // when the alert was posted.
                alert.expires_at_ms = alert.expires_at_ms.min(expires_at_ms);
                now_ms >= alert.expires_at_ms
            }
            ApprovalAlertState::Unknown => now_ms >= alert.expires_at_ms,
        };
        if withdraw {
            self.forget(key);
        }
        withdraw
    }

    pub(crate) fn clear(&mut self) {
        self.alerts.clear();
        self.order.clear();
    }
}

fn tracker() -> MutexGuard<'static, ApprovalAlertTracker> {
    static TRACKER: OnceLock<Mutex<ApprovalAlertTracker>> = OnceLock::new();
    TRACKER
        .get_or_init(|| Mutex::new(ApprovalAlertTracker::default()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

/// The tracked alert for an approval banner, or `None` for anything else.
fn approval_alert(
    notification: &DesktopNotificationPayload,
    session_generation: Option<u64>,
    now_ms: u64,
) -> Option<(ApprovalAlertKey, TrackedApprovalAlert)> {
    let context = notification.action_context.as_ref()?;
    if !is_time_sensitive_agent_approval(Some(context)) {
        return None;
    }
    let key = ApprovalAlertKey {
        room_id: context.room_id.clone()?,
        event_id: context.event_id.clone()?,
    };
    key.dismiss_key()?;
    Some((
        key,
        TrackedApprovalAlert {
            session_generation,
            // The prompt was sent before it was posted, so it expires no
            // later than this; the first readback narrows it.
            expires_at_ms: now_ms.saturating_add(AGENT_APPROVAL_TTL_MS),
        },
    ))
}

/// Whether a Linux waiter that gave up should keep the banner's handle: a
/// tracked approval is closed by the check loop, not abandoned.
#[cfg(target_os = "linux")]
pub(crate) fn tracks_event(event_id: &str) -> bool {
    tracker().tracks_event(event_id)
}

/// Remember an approval banner the OS accepted and make sure a check loop
/// runs until it is settled or expired.
pub(crate) async fn track_posted<R: Runtime>(
    app: &AppHandle<R>,
    notification: &DesktopNotificationPayload,
) {
    let generation = match notification.session_generation {
        Some(generation) => Some(generation),
        None => active_session(app).await.map(|active| active.generation),
    };
    let Some((key, alert)) = approval_alert(notification, generation, now_ms()) else {
        return;
    };
    if tracker().track(key, alert) {
        let app = app.clone();
        tauri::async_runtime::spawn(async move { check_until_settled(app).await });
    }
}

/// Sign-out: every banner is withdrawn anyway; stop tracking them.
pub(crate) fn clear() {
    tracker().clear();
}

async fn read_state<R: Runtime>(
    app: &AppHandle<R>,
    key: &ApprovalAlertKey,
    alert: &TrackedApprovalAlert,
) -> ApprovalAlertState {
    let Some(active) = active_session(app).await else {
        return ApprovalAlertState::Unknown;
    };
    if alert
        .session_generation
        .is_some_and(|generation| generation != active.generation)
    {
        // Another session's banner: its buttons can no longer act.
        return ApprovalAlertState::Settled;
    }
    let Some(core) = app.try_state::<Arc<synara_core::Core>>() else {
        return ApprovalAlertState::Unknown;
    };
    let core = core.inner().clone();
    let read =
        core.timeline_event_readback(synara_core::core_api::MatrixTimelineEventReadbackRequest {
            room_id: key.room_id.clone(),
            event_id: key.event_id.clone(),
        });
    match tokio::time::timeout(APPROVAL_ALERT_READ_TIMEOUT, read).await {
        Ok(Ok(readback)) if readback.session_generation == active.generation => {
            approval_alert_state(&readback.item, &active.account, now_ms())
        }
        Ok(Ok(_)) => ApprovalAlertState::Settled,
        Ok(Err(_)) | Err(_) => ApprovalAlertState::Unknown,
    }
}

async fn check_until_settled<R: Runtime>(app: AppHandle<R>) {
    loop {
        tokio::time::sleep(APPROVAL_ALERT_CHECK_INTERVAL).await;
        let Some(due) = tracker().due_for_check() else {
            return;
        };
        for (key, alert) in due {
            let state = read_state(&app, &key, &alert).await;
            if !tracker().apply(&key, state, now_ms()) {
                continue;
            }
            if let Some(dismiss_key) = key.dismiss_key() {
                dismiss_notifications_now(&[dismiss_key]).await;
            } else {
                log_notification_error(&app, "approval alert has no dismiss key");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::DesktopNotificationActionContext;
    use super::*;
    use synara_core::app::timeline::{NativeDecryptionState, NativeTimelineReaction};

    const NOW: u64 = 1_800_000_000_000;
    const USER: &str = "@reader:example.org";
    const PROMPT: &str = "⚠️ Approval required: dangerous command\nrm example";

    fn prompt() -> NativeTimelineItem {
        NativeTimelineItem {
            item_id: "prompt".into(),
            event_id: "$prompt".into(),
            sender: "@hermes:example.org".into(),
            event_type: "m.room.message".into(),
            body: PROMPT.into(),
            origin_server_ts: NOW - 1_000,
            decryption_state: None,
            reactions: Vec::new(),
        }
    }

    fn reaction(key: &str, me: bool) -> NativeTimelineReaction {
        NativeTimelineReaction {
            key: key.into(),
            count: 1,
            me,
            senders: Vec::new(),
        }
    }

    fn key(event_id: &str) -> ApprovalAlertKey {
        ApprovalAlertKey {
            room_id: "!room:example.org".into(),
            event_id: event_id.into(),
        }
    }

    fn alert(expires_at_ms: u64) -> TrackedApprovalAlert {
        TrackedApprovalAlert {
            session_generation: Some(1),
            expires_at_ms,
        }
    }

    #[test]
    fn a_prompt_is_settled_once_decided_anywhere_edited_redacted_or_expired() {
        let pending = ApprovalAlertState::Pending {
            expires_at_ms: NOW - 1_000 + AGENT_APPROVAL_TTL_MS,
        };
        assert_eq!(approval_alert_state(&prompt(), USER, NOW), pending);

        // Hermes seeds its own reactions; those are not decisions.
        let mut item = prompt();
        item.reactions.push(reaction("✅", false));
        assert_eq!(approval_alert_state(&item, USER, NOW), pending);
        // This account reacted, on this device or on the iPhone.
        item.reactions.push(reaction("❌", true));
        assert_eq!(
            approval_alert_state(&item, USER, NOW),
            ApprovalAlertState::Settled
        );

        // Hermes edits the card to its outcome; the inbox drops such an item.
        let mut item = prompt();
        item.body = "✅ Approved: rm example".into();
        assert_eq!(
            approval_alert_state(&item, USER, NOW),
            ApprovalAlertState::Settled
        );
        item.body = "Message removed".into();
        assert_eq!(
            approval_alert_state(&item, USER, NOW),
            ApprovalAlertState::Settled
        );

        let mut item = prompt();
        item.origin_server_ts = NOW - AGENT_APPROVAL_TTL_MS;
        assert_eq!(
            approval_alert_state(&item, USER, NOW),
            ApprovalAlertState::Settled
        );
    }

    #[test]
    fn an_unreadable_prompt_is_unknown_not_settled() {
        let mut item = prompt();
        item.decryption_state = Some(NativeDecryptionState::Pending);
        item.body = "Unable to decrypt this message".into();
        assert_eq!(
            approval_alert_state(&item, USER, NOW),
            ApprovalAlertState::Unknown
        );
        // A clock skew answer is not a settlement either.
        let mut item = prompt();
        item.origin_server_ts = NOW + 120_000;
        assert_eq!(
            approval_alert_state(&item, USER, NOW),
            ApprovalAlertState::Unknown
        );
        assert_eq!(
            approval_alert_state(&prompt(), "", NOW),
            ApprovalAlertState::Unknown
        );
    }

    #[test]
    fn the_tracker_withdraws_on_settle_or_expiry_and_stops_when_empty() {
        let mut tracker = ApprovalAlertTracker::default();
        assert!(tracker.track(key("$a"), alert(NOW + 300_000)));
        // One loop serves every alert.
        assert!(!tracker.track(key("$b"), alert(NOW + 300_000)));
        assert!(tracker.tracks_event("$a"));

        // Still pending: kept, and the expiry narrows to the prompt's own.
        assert!(!tracker.apply(
            &key("$a"),
            ApprovalAlertState::Pending {
                expires_at_ms: NOW + 60_000
            },
            NOW
        ));
        assert!(!tracker.apply(&key("$a"), ApprovalAlertState::Unknown, NOW + 59_999));
        assert!(tracker.apply(&key("$a"), ApprovalAlertState::Unknown, NOW + 60_000));
        assert!(!tracker.tracks_event("$a"));
        // Withdrawn once only.
        assert!(!tracker.apply(&key("$a"), ApprovalAlertState::Settled, NOW));

        // A later readback never extends the bound set at post time.
        assert!(!tracker.apply(
            &key("$b"),
            ApprovalAlertState::Pending {
                expires_at_ms: NOW + 900_000
            },
            NOW + 300_000 - 1
        ));
        assert!(tracker.apply(&key("$b"), ApprovalAlertState::Settled, NOW));

        // Empty: the loop stops, and the next alert starts a new one.
        assert_eq!(tracker.due_for_check(), None);
        assert!(tracker.track(key("$c"), alert(NOW)));
        assert_eq!(tracker.due_for_check().map(|due| due.len()), Some(1));
        tracker.clear();
        assert!(!tracker.tracks_event("$c"));
    }

    #[test]
    fn the_tracker_is_bounded_oldest_first() {
        let mut tracker = ApprovalAlertTracker::default();
        for index in 0..=MAX_TRACKED_APPROVAL_ALERTS {
            tracker.track(key(&format!("$e{index}")), alert(NOW));
        }
        assert!(!tracker.tracks_event("$e0"));
        assert!(tracker.tracks_event(&format!("$e{MAX_TRACKED_APPROVAL_ALERTS}")));
        assert_eq!(
            tracker.due_for_check().map(|due| due.len()),
            Some(MAX_TRACKED_APPROVAL_ALERTS)
        );
    }

    #[test]
    fn only_approval_banners_with_a_room_and_event_are_tracked() {
        let mut payload = DesktopNotificationPayload {
            session_generation: Some(3),
            sound: None,
            title: "Approval".into(),
            body: None,
            route: None,
            actions: None,
            action_context: Some(DesktopNotificationActionContext {
                kind: "agent-approval".into(),
                room_id: Some("!room:example.org".into()),
                event_id: Some("$prompt".into()),
            }),
            dismiss_keys: None,
            public_read_receipt: None,
        };
        let (tracked_key, tracked) = approval_alert(&payload, Some(3), NOW).unwrap();
        assert_eq!(tracked_key, key("$prompt"));
        assert_eq!(tracked.expires_at_ms, NOW + AGENT_APPROVAL_TTL_MS);
        assert_eq!(tracked.session_generation, Some(3));
        assert_eq!(tracked_key.dismiss_key().as_deref(), Some("event:$prompt"));

        payload.action_context.as_mut().unwrap().event_id = Some("$bad id".into());
        assert!(approval_alert(&payload, Some(3), NOW).is_none());
        payload.action_context.as_mut().unwrap().event_id = None;
        assert!(approval_alert(&payload, Some(3), NOW).is_none());
        payload.action_context = Some(DesktopNotificationActionContext {
            kind: "message".into(),
            room_id: Some("!room:example.org".into()),
            event_id: Some("$prompt".into()),
        });
        assert!(approval_alert(&payload, Some(3), NOW).is_none());
    }
}
