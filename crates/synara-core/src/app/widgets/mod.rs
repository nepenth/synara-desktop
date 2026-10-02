//! Experimental Element-style widget host (ADR 0004).
//!
//! Runtime enablement is an in-client setting (default off). Cargo compiles
//! `experimental-widgets` on this branch; that is not user enablement.

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
pub use registry::{WidgetKind, WidgetRegistry, WidgetSessionRecord, MAX_WIDGET_SESSIONS};
pub use url::{is_safe_widget_url, widget_target_origin};

#[cfg(test)]
mod tests;
