//! P5.1 / P5.2 — Timeline registry + snapshot/diff projection (harness foundation).
//!
//! Per-room (and optional thread) timeline owners stamped with session
//! generation, plus pure ordered-diff projection over Synara [`TimelineItem`]
//! DTOs. No SDK `Timeline` attach yet, no production Tauri commands, no
//! dual-backend, no event plaintext in errors.
//!
//! Authoritative design notes:
//! - `docs/matrix-rust-sdk/p5.1-timeline-registry.md`
//! - `docs/matrix-rust-sdk/p5.2-timeline-diffs.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod delta;
mod error;
mod projection;
mod registry;

pub use delta::{TimelineDeltaBatch, TimelineDeltaOp, TimelineSnapshot};
pub use error::TimelineError;
pub use projection::{reconstruct, TimelineProjection};
pub use registry::{TimelineEntry, TimelineKey, TimelineLifecycle, TimelineRegistry};

/// Static marker for link / schema smoke.
pub const MATRIX_TIMELINE_MARKER: &str = "matrix-timeline-registry-p5.1+diffs-p5.2";

/// Touch timeline registry + projection paths so they remain linked in non-test builds.
pub fn matrix_timeline_markers() -> &'static str {
    let reg = TimelineRegistry::new(0);
    debug_assert!(reg.is_empty());
    debug_assert_eq!(reg.active_count(), 0);
    debug_assert_eq!(TimelineLifecycle::Live.as_str(), "live");
    let proj = TimelineProjection::new(0);
    debug_assert!(proj.is_empty());
    debug_assert_eq!(TimelineDeltaOp::Clear.op_name(), "clear");
    debug_assert_eq!(
        MATRIX_TIMELINE_MARKER,
        "matrix-timeline-registry-p5.1+diffs-p5.2"
    );
    MATRIX_TIMELINE_MARKER
}

#[cfg(test)]
mod tests;
