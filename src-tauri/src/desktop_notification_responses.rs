//! Notification clicks and actions that arrive before Synara can act on them.
//!
//! macOS delivers a Notification Center click to a freshly launched process
//! before the renderer has restored the session, and a banner can outlive the
//! session (or the process) that posted it. A response is delivered at once
//! only when the renderer is listening for the installed session and the
//! banner belongs to that session or account. Otherwise it waits here
//! (bounded, with a TTL) until the renderer drains it, or it is refused and
//! the window is shown so the click is never silently lost.

use std::collections::VecDeque;
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::DesktopNotificationActionContext;

/// Clicks kept while the session or renderer is not ready.
const MAX_PENDING_RESPONSES: usize = 16;
/// An approval expires after 5 minutes; a click older than this is stale.
const PENDING_RESPONSE_TTL: Duration = Duration::from_secs(10 * 60);

/// Identifies this process, so a generation number reused by a later launch
/// (generations restart at 1) is never mistaken for the original session.
pub(super) fn process_nonce() -> u64 {
    static NONCE: OnceLock<u64> = OnceLock::new();
    *NONCE.get_or_init(|| {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        // Never zero, so a decoded default can't match it.
        (nanos ^ (u64::from(std::process::id()) << 32)) | 1
    })
}

/// The session a notification was posted for. macOS persists it with the
/// request so it survives a relaunch.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NotificationBinding {
    #[serde(default)]
    pub session_generation: Option<u64>,
    /// The signed-in account's user id when the banner was posted.
    #[serde(default)]
    pub account: Option<String>,
    #[serde(default)]
    pub process: Option<u64>,
}

/// What the user asked for from a banner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum NotificationResponse {
    /// Show the window at this route (a click, or Review).
    Open { route: String },
    /// A renderer-handled action button (Approve once, Deny).
    Action {
        action_id: String,
        context: Option<DesktopNotificationActionContext>,
    },
}

/// Review and approve-always never send anything from a banner: they open the
/// exact prompt, natively, so the window comes forward even when hidden.
pub(super) fn response_for_action(
    action_id: &str,
    route: Option<&str>,
    context: Option<DesktopNotificationActionContext>,
) -> NotificationResponse {
    let opens_prompt = matches!(
        action_id,
        "agent-approval.review" | "agent-approval.approve-always"
    );
    match route {
        Some(route) if opens_prompt => NotificationResponse::Open {
            route: route.to_owned(),
        },
        _ => NotificationResponse::Action {
            action_id: action_id.to_owned(),
            context,
        },
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ActiveSession {
    pub generation: u64,
    pub account: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Admission {
    /// Deliver now, bound to this generation (`None`: an unbound notice).
    Deliver { generation: Option<u64> },
    /// Keep until the renderer drains.
    Queue,
    /// Posted for another account (or an unverifiable approval): drop it and
    /// show the window.
    Reject,
}

/// Decide what to do with one response. `critical` marks approval banners,
/// which are never delivered without a matching session or account.
pub(super) fn admit(
    binding: &NotificationBinding,
    critical: bool,
    active: Option<&ActiveSession>,
    renderer_ready: Option<u64>,
    process: u64,
) -> Admission {
    let Some(active) = active else {
        return Admission::Queue;
    };
    let same_session =
        binding.process == Some(process) && binding.session_generation == Some(active.generation);
    let same_account = binding.account.as_deref() == Some(active.account.as_str());
    let unbound_notice =
        !critical && binding.session_generation.is_none() && binding.account.is_none();
    if !(same_session || same_account || unbound_notice) {
        return Admission::Reject;
    }
    if renderer_ready != Some(active.generation) {
        return Admission::Queue;
    }
    Admission::Deliver {
        generation: (!unbound_notice || same_session || same_account).then_some(active.generation),
    }
}

struct PendingResponse {
    binding: NotificationBinding,
    critical: bool,
    response: NotificationResponse,
    queued_at: Instant,
}

/// Responses waiting for the renderer, and the generation it last drained for.
#[derive(Default)]
pub(super) struct PendingResponses {
    entries: VecDeque<PendingResponse>,
    ready: Option<u64>,
}

impl PendingResponses {
    pub fn renderer_ready(&self) -> Option<u64> {
        self.ready
    }

    pub fn push(
        &mut self,
        binding: NotificationBinding,
        critical: bool,
        response: NotificationResponse,
        now: Instant,
    ) {
        self.entries
            .retain(|entry| now.saturating_duration_since(entry.queued_at) < PENDING_RESPONSE_TTL);
        while self.entries.len() >= MAX_PENDING_RESPONSES {
            self.entries.pop_front();
        }
        self.entries.push_back(PendingResponse {
            binding,
            critical,
            response,
            queued_at: now,
        });
    }

    /// The renderer listens for `active`: from now on responses for it are
    /// delivered directly. Returns the queued ones it may act on, oldest
    /// first, each with the generation to deliver under; drops expired and
    /// refused ones.
    pub fn drain_for(
        &mut self,
        active: &ActiveSession,
        process: u64,
        now: Instant,
    ) -> Vec<(NotificationResponse, Option<u64>)> {
        self.ready = Some(active.generation);
        self.entries
            .drain(..)
            .filter(|entry| now.saturating_duration_since(entry.queued_at) < PENDING_RESPONSE_TTL)
            .filter_map(|entry| {
                match admit(
                    &entry.binding,
                    entry.critical,
                    Some(active),
                    Some(active.generation),
                    process,
                ) {
                    Admission::Deliver { generation } => Some((entry.response, generation)),
                    Admission::Queue | Admission::Reject => None,
                }
            })
            .collect()
    }

    /// Signed out: nothing is listening until the next drain.
    pub fn reset(&mut self) {
        self.entries.clear();
        self.ready = None;
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROCESS: u64 = 41;

    fn active(generation: u64, account: &str) -> ActiveSession {
        ActiveSession {
            generation,
            account: account.to_owned(),
        }
    }

    fn bound(generation: u64, account: &str, process: u64) -> NotificationBinding {
        NotificationBinding {
            session_generation: Some(generation),
            account: Some(account.to_owned()),
            process: Some(process),
        }
    }

    fn open(route: &str) -> NotificationResponse {
        NotificationResponse::Open {
            route: route.to_owned(),
        }
    }

    #[test]
    fn a_click_before_restore_waits_and_replays_for_the_same_account() {
        let mut pending = PendingResponses::default();
        let now = Instant::now();
        // Cold launch: the banner came from the previous process.
        let binding = bound(1, "@alice:example.org", PROCESS - 1);
        assert_eq!(admit(&binding, true, None, None, PROCESS), Admission::Queue);
        pending.push(
            binding,
            true,
            NotificationResponse::Action {
                action_id: "agent-approval.approve-once".to_owned(),
                context: None,
            },
            now,
        );
        // Session restored, renderer not listening yet: still waiting.
        let alice = active(1, "@alice:example.org");
        assert_eq!(
            admit(
                &bound(1, "@alice:example.org", PROCESS - 1),
                true,
                Some(&alice),
                None,
                PROCESS
            ),
            Admission::Queue
        );
        let drained = pending.drain_for(&alice, PROCESS, now + Duration::from_secs(3));
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].1, Some(1));
        assert_eq!(pending.renderer_ready(), Some(1));
        assert_eq!(pending.len(), 0);
    }

    #[test]
    fn a_reused_generation_from_another_process_needs_the_same_account() {
        let bob = active(1, "@bob:example.org");
        // Generation 1 == 1 by coincidence, but another account posted it.
        assert_eq!(
            admit(
                &bound(1, "@alice:example.org", PROCESS - 1),
                true,
                Some(&bob),
                Some(1),
                PROCESS
            ),
            Admission::Reject
        );
        // Same process and generation always matches.
        assert_eq!(
            admit(
                &bound(1, "@alice:example.org", PROCESS),
                true,
                Some(&bob),
                Some(1),
                PROCESS
            ),
            Admission::Deliver {
                generation: Some(1)
            }
        );
        // After sign-out and sign-in as the same account, old banners work.
        let alice = active(4, "@alice:example.org");
        assert_eq!(
            admit(
                &bound(2, "@alice:example.org", PROCESS),
                true,
                Some(&alice),
                Some(4),
                PROCESS
            ),
            Admission::Deliver {
                generation: Some(4)
            }
        );
    }

    #[test]
    fn unbound_notices_open_for_anyone_but_approvals_never_do() {
        let alice = active(2, "@alice:example.org");
        let unbound = NotificationBinding::default();
        assert_eq!(
            admit(&unbound, false, Some(&alice), Some(2), PROCESS),
            Admission::Deliver { generation: None }
        );
        assert_eq!(
            admit(&unbound, true, Some(&alice), Some(2), PROCESS),
            Admission::Reject
        );
    }

    #[test]
    fn the_queue_is_bounded_and_drops_stale_clicks() {
        let mut pending = PendingResponses::default();
        let start = Instant::now();
        for index in 0..(MAX_PENDING_RESPONSES + 4) {
            pending.push(
                NotificationBinding::default(),
                false,
                open(&format!("/inbox/{index}")),
                start,
            );
        }
        assert_eq!(pending.len(), MAX_PENDING_RESPONSES);
        let alice = active(1, "@alice:example.org");
        let drained = pending.drain_for(&alice, PROCESS, start + Duration::from_secs(1));
        assert_eq!(drained.len(), MAX_PENDING_RESPONSES);
        assert_eq!(drained[0].0, open("/inbox/4"));

        pending.push(
            NotificationBinding::default(),
            false,
            open("/inbox/old"),
            start,
        );
        let late = start + PENDING_RESPONSE_TTL + Duration::from_secs(1);
        assert!(pending.drain_for(&alice, PROCESS, late).is_empty());
    }

    #[test]
    fn drain_drops_banners_from_another_account() {
        let mut pending = PendingResponses::default();
        let now = Instant::now();
        pending.push(
            bound(1, "@bob:example.org", PROCESS - 1),
            false,
            open("/a"),
            now,
        );
        pending.push(
            bound(1, "@alice:example.org", PROCESS - 1),
            false,
            open("/b"),
            now,
        );
        let drained = pending.drain_for(&active(1, "@alice:example.org"), PROCESS, now);
        assert_eq!(drained, vec![(open("/b"), Some(1))]);
        pending.reset();
        assert_eq!(pending.renderer_ready(), None);
    }

    #[test]
    fn review_opens_the_prompt_natively_instead_of_asking_the_renderer() {
        let route = Some("/direct/!agents:example.org/$prompt");
        assert_eq!(
            response_for_action("agent-approval.review", route, None),
            open("/direct/!agents:example.org/$prompt")
        );
        assert_eq!(
            response_for_action("agent-approval.approve-always", route, None),
            open("/direct/!agents:example.org/$prompt")
        );
        assert!(matches!(
            response_for_action("agent-approval.approve-once", route, None),
            NotificationResponse::Action { .. }
        ));
        // Without a route the renderer still opens the room.
        assert!(matches!(
            response_for_action("agent-approval.review", None, None),
            NotificationResponse::Action { .. }
        ));
    }

    #[test]
    fn process_nonce_is_stable_and_nonzero() {
        assert_eq!(process_nonce(), process_nonce());
        assert_ne!(process_nonce(), 0);
    }
}
