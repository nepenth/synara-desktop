//! Compile-only linkage smoke for Matrix Rust SDK 0.19.1 (P1.2).
//!
//! Proves `matrix-sdk` / `matrix-sdk-ui` type paths resolve in the production
//! Tauri crate. Does **not** construct a live Client, open network, start sync,
//! login, or register Tauri commands.

use matrix_sdk::widget::WidgetDriver;
use matrix_sdk::Client;
use matrix_sdk_ui::timeline::Timeline;

// Keep SDK type paths resolved in non-test builds (avoids dead_code + dead-strip).
const _: fn() -> &'static str = matrix_sdk_link_markers;

/// Touch SDK type paths so the dependency graph must link.
/// Returns a static marker only — no async runtime, store, or network.
fn matrix_sdk_link_markers() -> &'static str {
    let _client: Option<Client> = None;
    let _timeline: Option<Timeline> = None;
    let _widget_driver: Option<WidgetDriver> = None;
    let _client_name = std::any::type_name::<Client>();
    let _timeline_name = std::any::type_name::<Timeline>();
    let _widget_driver_name = std::any::type_name::<WidgetDriver>();
    "matrix-sdk=0.19.1+matrix-sdk-ui=0.19.1+experimental-widgets link-smoke"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_with_matrix_sdk_0_19() {
        let marker = matrix_sdk_link_markers();
        assert!(
            marker.contains("matrix-sdk=0.19.1"),
            "expected version marker, got {marker}"
        );
        assert!(
            std::any::type_name::<Client>().contains("Client"),
            "matrix_sdk::Client type path must resolve"
        );
        assert!(
            std::any::type_name::<Timeline>().contains("Timeline"),
            "matrix_sdk_ui::timeline::Timeline type path must resolve"
        );
        assert!(
            std::any::type_name::<WidgetDriver>().contains("WidgetDriver"),
            "matrix_sdk::widget::WidgetDriver type path must resolve"
        );
    }
}
