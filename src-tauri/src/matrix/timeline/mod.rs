//! P5.1 — Timeline registry and lifecycle (harness foundation).
//!
//! Per-room (and optional thread) timeline owners stamped with session
//! generation. Snapshot/diff mapping is **P5.2**. No production Tauri commands,
//! no dual-backend, no event plaintext in errors.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p5.1-timeline-registry.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod registry;

pub use error::TimelineError;
pub use registry::{TimelineEntry, TimelineKey, TimelineLifecycle, TimelineRegistry};

/// Static marker for link / schema smoke.
pub const MATRIX_TIMELINE_MARKER: &str = "matrix-timeline-registry-p5.1";

/// Touch timeline registry paths so they remain linked in non-test builds.
pub fn matrix_timeline_markers() -> &'static str {
    let reg = TimelineRegistry::new(0);
    debug_assert!(reg.is_empty());
    debug_assert_eq!(reg.active_count(), 0);
    debug_assert_eq!(TimelineLifecycle::Live.as_str(), "live");
    debug_assert_eq!(MATRIX_TIMELINE_MARKER, "matrix-timeline-registry-p5.1");
    MATRIX_TIMELINE_MARKER
}

#[cfg(test)]
mod tests;
