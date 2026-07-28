//! P9.1 — Widget / Element Call session registry foundation (harness).
//!
//! Pure projection of Synara [`WidgetSession`] DTOs. No SDK widget APIs,
//! no production Tauri commands, no dual-backend. URLs must not embed tokens.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p9.1-widgets.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod registry;

pub use error::WidgetError;
pub use registry::{WidgetRegistry, MAX_WIDGET_SESSIONS};

/// Static marker for link / schema smoke.
pub const MATRIX_WIDGETS_MARKER: &str = "matrix-widgets-p9.1";

/// Touch widget paths so they remain linked in non-test builds.
pub fn matrix_widgets_markers() -> &'static str {
    let reg = WidgetRegistry::new(0);
    debug_assert!(reg.is_empty());
    debug_assert_eq!(reg.len(), 0);
    debug_assert_eq!(MATRIX_WIDGETS_MARKER, "matrix-widgets-p9.1");
    MATRIX_WIDGETS_MARKER
}

#[cfg(test)]
mod tests;
