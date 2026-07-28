//! P9.3 — App-focus suppression and badge semantics foundation (harness).
//!
//! Pure policy and count state only. No window hooks, OS notification or badge
//! APIs, production Tauri commands, event content, or dual-backend behavior.
//!
//! Authoritative design note:
//! `docs/matrix-rust-sdk/p9.3-focus-suppression.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod badge;
mod focus;

pub use badge::BadgeCounter;
pub use focus::{FocusState, SuppressionPolicy};

/// Static marker for link / schema smoke.
pub const MATRIX_NOTIFICATION_POLICY_MARKER: &str = "matrix-notification-policy-p9.3";

/// Touch notification-policy paths so they remain linked in non-test builds.
pub fn matrix_notification_policy_markers() -> &'static str {
    let policy = SuppressionPolicy::default();
    let badges = BadgeCounter::new(0);
    debug_assert_eq!(policy.focus_state(), FocusState::Background);
    debug_assert_eq!(badges.total(), 0);
    debug_assert_eq!(
        MATRIX_NOTIFICATION_POLICY_MARKER,
        "matrix-notification-policy-p9.3"
    );
    MATRIX_NOTIFICATION_POLICY_MARKER
}

#[cfg(test)]
mod tests;
