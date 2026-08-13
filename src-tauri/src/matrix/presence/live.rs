//! Desktop AppHandle adapter for the Core presence owner.
//!
//! `NativePresenceOwner` lives in synara-core. This file only maps the
//! existing `matrix-presence-updated` Tauri event onto the Core emit sink.

use std::sync::Arc;

use matrix_sdk::Client;
use tauri::{AppHandle, Emitter};

pub use synara_core::app::presence::{
    NativePresenceOwner, NativePresenceSnapshot, NativePresenceSnapshotResult, NativePresenceState,
    NativePresenceSubscription, NativePresenceUpdate, NativePresenceUpdateOutcome,
    PRESENCE_UPDATED_EVENT,
};

/// Start the Core owner and emit presence updates on the existing Tauri event.
pub fn start(
    client: &Client,
    app: AppHandle,
    session_generation: u64,
) -> Result<NativePresenceOwner, &'static str> {
    NativePresenceOwner::start(
        client,
        Arc::new(move |update| {
            let _ = app.emit(PRESENCE_UPDATED_EVENT, update);
        }),
        session_generation,
    )
}
