//! Desktop AppHandle adapter for the Core dehydrated-device owner.

use std::sync::Arc;

use matrix_sdk::Client;
use tauri::{AppHandle, Emitter};

use synara_core::app::dehydrated_devices::NativeDehydratedDevicesOwner;
use synara_core::app::devices::{NativeDeviceUpdateSignal, DEVICE_LIST_UPDATED_EVENT};

/// Start the Core owner and wake the existing Sessions list on MSC3814 events.
pub async fn start(
    client: &Client,
    app: AppHandle,
    session_generation: u64,
) -> NativeDehydratedDevicesOwner {
    NativeDehydratedDevicesOwner::start(
        client,
        Arc::new(move |signal: NativeDeviceUpdateSignal| {
            let _ = app.emit(DEVICE_LIST_UPDATED_EVENT, signal);
        }),
        session_generation,
    )
    .await
}
