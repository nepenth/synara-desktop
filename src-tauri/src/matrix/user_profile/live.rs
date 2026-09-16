//! Desktop AppHandle adapter for the Core own-profile stream.

use std::sync::Arc;

use matrix_sdk::Client;
use tauri::{AppHandle, Emitter};

pub use synara_core::app::user_profile::{NativeOwnProfileOwner, OWN_PROFILE_CHANGED_EVENT};

/// Start the Core owner and emit own-profile updates on the Tauri event.
pub fn start(
    client: &Client,
    app: AppHandle,
    session_generation: u64,
) -> Result<NativeOwnProfileOwner, &'static str> {
    NativeOwnProfileOwner::start(
        client,
        Arc::new(move |update| {
            let _ = app.emit(OWN_PROFILE_CHANGED_EVENT, update);
        }),
        session_generation,
    )
}
