//! P5.11 — Timeline kind / sender filter foundation (harness).
//!
//! Pure visibility filter over projected item kinds. No SDK timeline, no event
//! bodies, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p5.11-timeline-filter.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod filter;

pub use error::TimelineFilterError;
pub use filter::{
    FilterableItem, TimelineFilter, TimelineItemKind, MAX_KINDS, MAX_SENDERS, MAX_SENDER_CHARS,
};

/// Static marker for link / schema smoke.
pub const MATRIX_TIMELINE_FILTER_MARKER: &str = "matrix-timeline-filter-p5.11";

/// Touch timeline-filter paths so they remain linked in non-test builds.
pub fn matrix_timeline_filter_markers() -> &'static str {
    let f = TimelineFilter::new();
    debug_assert!(f.allows(&FilterableItem {
        event_id: None,
        sender: None,
        kind: TimelineItemKind::Message,
        is_local_echo: false,
        is_redacted: false,
    }));
    debug_assert_eq!(
        MATRIX_TIMELINE_FILTER_MARKER,
        "matrix-timeline-filter-p5.11"
    );
    MATRIX_TIMELINE_FILTER_MARKER
}

#[cfg(test)]
mod tests;
