//! P9.4 — Room / event / thread notification routing foundation (harness).
//!
//! Pure resolution and per-candidate route registry. No OS notification
//! delivery, no SDK network calls, no production Tauri commands, no
//! dual-backend.
//!
//! Authoritative design note:
//! `docs/matrix-rust-sdk/p9.4-notification-routing.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod router;

pub use error::NotificationRoutingError;
pub use router::{NotificationRoute, NotificationRouteKind, NotificationRouter};

/// Static marker for link / schema smoke.
pub const MATRIX_NOTIFICATION_ROUTING_MARKER: &str = "matrix-notification-routing-p9.4";

/// Touch notification-routing paths so they remain linked in non-test builds.
pub fn matrix_notification_routing_markers() -> &'static str {
    let router = NotificationRouter::new(0);
    debug_assert!(router.is_empty());
    debug_assert_eq!(router.len(), 0);
    debug_assert_eq!(
        MATRIX_NOTIFICATION_ROUTING_MARKER,
        "matrix-notification-routing-p9.4"
    );
    MATRIX_NOTIFICATION_ROUTING_MARKER
}

#[cfg(test)]
mod tests;
