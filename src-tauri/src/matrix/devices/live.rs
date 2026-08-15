//! Desktop AppHandle adapter for the Core device-list owner.
//!
//! `NativeDeviceOwner` lives in synara-core. This file only maps the
//! existing `matrix-device-list-updated` Tauri event onto the Core emit sink.

use std::sync::Arc;

use matrix_sdk::Client;
use tauri::{AppHandle, Emitter};

use synara_core::app::devices::{
    NativeDeviceOwner, NativeDeviceUpdateSignal, DEVICE_LIST_UPDATED_EVENT,
};

/// Start the Core owner and emit device-list wakeups on the existing Tauri event.
pub async fn start(
    client: &Client,
    app: AppHandle,
    session_generation: u64,
) -> Result<NativeDeviceOwner, &'static str> {
    NativeDeviceOwner::start(
        client,
        Arc::new(move |signal: NativeDeviceUpdateSignal| {
            let _ = app.emit(DEVICE_LIST_UPDATED_EVENT, signal);
        }),
        session_generation,
    )
    .await
}
