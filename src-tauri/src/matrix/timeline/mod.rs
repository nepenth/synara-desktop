//! P5.1–P5.3 — Timeline registry, diffs, and pagination (harness foundation).
//!
//! Per-room (and optional thread) timeline owners stamped with session
//! generation, pure ordered-diff projection over Synara [`TimelineItem`] DTOs,
//! and a pagination state machine. No SDK `Timeline` attach yet, no production
//! Tauri commands, no dual-backend, no event plaintext in errors.
//!
//! Authoritative design notes:
//! - `docs/matrix-rust-sdk/p5.1-timeline-registry.md`
//! - `docs/matrix-rust-sdk/p5.2-timeline-diffs.md`
//! - `docs/matrix-rust-sdk/p5.3-timeline-pagination.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod delta;
mod error;
mod pagination;
mod projection;
mod registry;

pub use delta::{TimelineDeltaBatch, TimelineDeltaOp, TimelineSnapshot};
pub use error::TimelineError;
pub use pagination::{
    DirectionStatus, PaginationDirection, PaginationOutcome, PaginationPhase, PaginationRequest,
    TimelinePagination,
};
pub use projection::{reconstruct, TimelineProjection};
pub use registry::{TimelineEntry, TimelineKey, TimelineLifecycle, TimelineRegistry};

/// Static marker for link / schema smoke.
pub const MATRIX_TIMELINE_MARKER: &str = "matrix-timeline-registry-p5.1+diffs-p5.2+pagination-p5.3";

/// Touch timeline registry + projection + pagination paths so they remain linked.
pub fn matrix_timeline_markers() -> &'static str {
    let reg = TimelineRegistry::new(0);
    debug_assert!(reg.is_empty());
    debug_assert_eq!(reg.active_count(), 0);
    debug_assert_eq!(TimelineLifecycle::Live.as_str(), "live");
    let proj = TimelineProjection::new(0);
    debug_assert!(proj.is_empty());
    debug_assert_eq!(TimelineDeltaOp::Clear.op_name(), "clear");
    debug_assert_eq!(PaginationDirection::Backwards.as_str(), "backwards");
    debug_assert_eq!(PaginationPhase::Idle.as_str(), "idle");
    debug_assert_eq!(
        MATRIX_TIMELINE_MARKER,
        "matrix-timeline-registry-p5.1+diffs-p5.2+pagination-p5.3"
    );
    MATRIX_TIMELINE_MARKER
}

#[cfg(test)]
mod tests;
