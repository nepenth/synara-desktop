//! Timeline create/subscribe/pagination compile-only API-shape probes.
//!
//! Compile-only API-shape probes; do not prove runtime/network semantics.

use matrix_sdk_ui::Timeline;
use matrix_sdk_ui::timeline::{TimelineBuilder, TimelineFocus};

/// P0.3b-timeline-subscribe — `Timeline::subscribe`.
///
/// Source: `crates/matrix-sdk-ui/src/timeline/mod.rs` (`pub async fn subscribe`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_timeline_subscribe() {
    async fn _shape(timeline: &Timeline) {
        let _ = timeline.subscribe().await;
    }
    let _ = _shape;
}

/// P0.3b-timeline-paginate-backwards — `Timeline::paginate_backwards`.
///
/// Source: `crates/matrix-sdk-ui/src/timeline/pagination.rs`
/// (`pub async fn paginate_backwards`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_timeline_paginate_backwards() {
    async fn _shape(timeline: &Timeline, n: u16) -> Result<bool, matrix_sdk_ui::timeline::Error> {
        timeline.paginate_backwards(n).await
    }
    let _ = _shape;
}

/// P0.3b-timeline-paginate-forwards — `Timeline::paginate_forwards`.
///
/// Source: `crates/matrix-sdk-ui/src/timeline/pagination.rs`
/// (`pub async fn paginate_forwards`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_timeline_paginate_forwards() {
    async fn _shape(timeline: &Timeline, n: u16) -> Result<bool, matrix_sdk_ui::timeline::Error> {
        timeline.paginate_forwards(n).await
    }
    let _ = _shape;
}

/// P0.3b-timeline-builder-with-focus — `TimelineBuilder::with_focus`.
///
/// Source: `crates/matrix-sdk-ui/src/timeline/builder.rs` (`pub fn with_focus`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_timeline_builder_with_focus() {
    fn _shape(builder: TimelineBuilder, focus: TimelineFocus) -> TimelineBuilder {
        builder.with_focus(focus)
    }
    let _ = _shape;
}

/// P0.3b-timeline-focus-type — `matrix_sdk_ui::timeline::TimelineFocus`.
///
/// Source: `crates/matrix-sdk-ui/src/timeline/mod.rs` (`pub enum TimelineFocus`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_timeline_focus_type() -> &'static str {
    std::any::type_name::<TimelineFocus>()
}

/// Run every timeline probe (compile-only; no network, stores, or secrets).
pub fn run_all() {
    probe_timeline_subscribe();
    probe_timeline_paginate_backwards();
    probe_timeline_paginate_forwards();
    probe_timeline_builder_with_focus();
    let _ = probe_timeline_focus_type();
}
