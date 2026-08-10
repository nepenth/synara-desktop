//! Experimental widget / Element Call surface probes (`experimental-widgets`).
//!
//! Profile: `profile-experimental-widgets`.
//! Compile-only; does not run a widget driver, open a webview, or perform I/O.
//! Module existence is not Element Call product parity.

use matrix_sdk::ruma::events::ToDeviceEventType;
use matrix_sdk::widget::{
    Capabilities, ClientProperties, ToDeviceEventFilter, VirtualElementCallWidgetConfig,
    VirtualElementCallWidgetProperties, WidgetDriver, WidgetDriverHandle, WidgetSettings,
};
use matrix_sdk::{Client, Room};

/// Probe IDs compiled under `profile-experimental-widgets`.
pub const PROBE_IDS: &[&str] = &[
    "P0.3c-widget-driver-type",
    "P0.3c-widget-driver-handle-type",
    "P0.3c-widget-settings-type",
    "P0.3c-widget-driver-new",
    "P0.3c-widget-settings-generate-webview-url",
    "P0.3c-widget-new-virtual-element-call",
    "P0.3c-widget-capabilities-type",
    "P0.3c-widget-capabilities-send-read-fields",
    "P0.3c-widget-to-device-event-filter-type",
];

/// P0.3c-widget-driver-type
pub fn probe_widget_driver_type() -> &'static str {
    std::any::type_name::<WidgetDriver>()
}

/// P0.3c-widget-driver-handle-type
pub fn probe_widget_driver_handle_type() -> &'static str {
    std::any::type_name::<WidgetDriverHandle>()
}

/// P0.3c-widget-settings-type
pub fn probe_widget_settings_type() -> &'static str {
    std::any::type_name::<WidgetSettings>()
}

/// P0.3c-widget-driver-new
pub fn probe_widget_driver_new() {
    fn _shape(settings: WidgetSettings) -> (WidgetDriver, WidgetDriverHandle) {
        WidgetDriver::new(settings)
    }
    let _ = _shape;
}

/// P0.3c-widget-settings-generate-webview-url
pub fn probe_widget_settings_generate_webview_url() {
    async fn _shape(settings: &WidgetSettings, room: &Room, props: ClientProperties) {
        let _ = settings.generate_webview_url(room, props).await;
    }
    let _ = _shape;
}

/// P0.3c-widget-new-virtual-element-call
pub fn probe_widget_new_virtual_element_call() {
    fn _shape(props: VirtualElementCallWidgetProperties, config: VirtualElementCallWidgetConfig) {
        let _ = WidgetSettings::new_virtual_element_call_widget(props, config);
    }
    let _ = _shape;
}

/// P0.3c-widget-capabilities-type
pub fn probe_widget_capabilities_type() -> &'static str {
    std::any::type_name::<Capabilities>()
}

/// P0.3c-widget-capabilities-send-read-fields — capability filter vectors (incl. call/key filters).
///
/// Source: `crates/matrix-sdk/src/widget/capabilities.rs` (`pub struct Capabilities`).
/// Does not prove encryption-key exchange runtime or membership writes.
pub fn probe_widget_capabilities_send_read_fields() {
    fn _shape(caps: &Capabilities) -> (usize, usize, bool, bool) {
        (
            caps.send.len(),
            caps.read.len(),
            caps.send_delayed_event,
            caps.update_delayed_event,
        )
    }
    let _ = _shape;
}

/// P0.3c-widget-to-device-event-filter-type — to-device capability filter constructor.
///
/// Used by widget capability negotiation (e.g. encryption_keys to-device types).
/// Source: `crates/matrix-sdk/src/widget/filter.rs` (`ToDeviceEventFilter`).
pub fn probe_widget_to_device_event_filter_type() {
    fn _shape(event_type: ToDeviceEventType) -> ToDeviceEventFilter {
        ToDeviceEventFilter::new(event_type)
    }
    let _ = _shape;
    let _ = std::any::type_name::<ToDeviceEventFilter>();
}

/// Keep `Client` referenced so UI/core unification stays exercised with this profile.
#[allow(dead_code)]
fn _client_type_anchor() -> &'static str {
    std::any::type_name::<Client>()
}

/// Run every experimental-widgets probe (compile-only).
pub fn run_all() {
    let _ = probe_widget_driver_type();
    let _ = probe_widget_driver_handle_type();
    let _ = probe_widget_settings_type();
    probe_widget_driver_new();
    probe_widget_settings_generate_webview_url();
    probe_widget_new_virtual_element_call();
    let _ = probe_widget_capabilities_type();
    probe_widget_capabilities_send_read_fields();
    probe_widget_to_device_event_filter_type();
    let _ = _client_type_anchor();
}
