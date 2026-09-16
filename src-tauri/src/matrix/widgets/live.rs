//! Desktop AppHandle adapter for the Core experimental widget owner.

use std::sync::Arc;

use matrix_sdk::Client;
use tauri::AppHandle;

use super::host::{destroy_widget_window, deliver_to_widget_window};
pub use synara_core::app::widgets::NativeWidgetOwner;

/// Start the Core owner and map driver JSON onto the isolated widget webview.
pub fn start(
    client: &Client,
    app: AppHandle,
    session_generation: u64,
) -> Result<NativeWidgetOwner, &'static str> {
    let deliver_app = app.clone();
    let destroy_app = app;
    NativeWidgetOwner::start(
        client,
        Arc::new(move |session_id, message| {
            deliver_to_widget_window(&deliver_app, &session_id, &message);
        }),
        Arc::new(move |session_id| {
            destroy_widget_window(&destroy_app, &session_id);
        }),
        session_generation,
    )
}
