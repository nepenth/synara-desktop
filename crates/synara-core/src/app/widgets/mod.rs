//! Experimental Element-style widget host (ADR 0004).
//!
//! Runtime enablement is an in-client setting (default off). Cargo compiles
//! `experimental-widgets` on this branch; that is not user enablement.

#![allow(dead_code)]
#![allow(unused_imports)]

mod capabilities;
mod live;
mod registry;
mod url;

pub use capabilities::{
    filter_requested_capabilities, SynaraWidgetCapabilitiesProvider, WidgetGrantPolicy,
};
pub use live::{
    AgentWidgetEntry, ListedWidget, NativeWidgetOwner, WidgetListSnapshot, WidgetOpenResult,
    WidgetToWebviewEmit, WidgetWindowDestroy,
};
pub use registry::{
    WidgetKind, WidgetRegistry, WidgetSessionRecord, MATRIX_WIDGETS_MARKER, MAX_WIDGET_SESSIONS,
};
pub use url::{is_safe_widget_url, widget_target_origin};

/// Static marker for link / schema smoke.
pub const MATRIX_WIDGETS_EXPERIMENTAL_MARKER: &str = MATRIX_WIDGETS_MARKER;

/// Touch widget paths so they remain linked in non-test builds.
pub fn matrix_widgets_markers() -> &'static str {
    debug_assert_eq!(MAX_WIDGET_SESSIONS, 32);
    debug_assert!(is_safe_widget_url("https://widgets.example.org/app", false));
    debug_assert!(!is_safe_widget_url("http://127.0.0.1:3000/", false));
    debug_assert_eq!(MATRIX_WIDGETS_MARKER, "matrix-widgets-experimental-v1");
    MATRIX_WIDGETS_MARKER
}

#[cfg(test)]
mod tests;
