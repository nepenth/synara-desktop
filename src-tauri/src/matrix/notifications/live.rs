//! Desktop AppHandle adapter for the Core notification observation stream.
//!
//! `NativeNotificationObservationOwner` lives in synara-core. This file only
//! maps the `matrix-notification-observed` Tauri event onto the Core emit
//! sink; policy stays in Core's decision owner.

use std::sync::Arc;

use matrix_sdk::Client;
use tauri::{AppHandle, Emitter};

pub use synara_core::app::notifications::{
    NativeNotificationObservationOwner, NOTIFICATION_OBSERVED_EVENT,
};

/// Start the Core owner and push each observation on the Tauri event.
pub fn start(
    client: &Client,
    app: AppHandle,
    session_generation: u64,
) -> Result<NativeNotificationObservationOwner, &'static str> {
    NativeNotificationObservationOwner::start(
        client,
        Arc::new(move |observation| {
            let _ = app.emit(NOTIFICATION_OBSERVED_EVENT, observation);
        }),
        session_generation,
    )
}
