//! Desktop AppHandle adapter for the Core join-rule owner.
//!
//! `NativeRoomJoinRuleOwner` lives in synara-core. This file only maps the
//! existing `matrix-room-join-rule-updated` Tauri event onto the Core emit sink.

use std::sync::Arc;

use matrix_sdk::Client;
use tauri::{AppHandle, Emitter};

pub use synara_core::app::room_profile::{
    project_join_rule, NativeRoomJoinRuleOwner, NativeRoomJoinRuleUpdate,
    ROOM_JOIN_RULE_UPDATED_EVENT,
};

/// Start the Core owner and emit join-rule updates on the existing Tauri event.
pub fn start(
    client: &Client,
    app: AppHandle,
    session_generation: u64,
) -> Result<NativeRoomJoinRuleOwner, &'static str> {
    NativeRoomJoinRuleOwner::start(
        client,
        Arc::new(move |update| {
            let _ = app.emit(ROOM_JOIN_RULE_UPDATED_EVENT, update);
        }),
        session_generation,
    )
}
