//! Desktop AppHandle adapter for the Core image-pack owner.
//!
//! `NativeImagePackOwner` lives in synara-core. This file only maps the
//! existing `matrix-image-packs-updated` Tauri event onto the Core emit sink.

use std::sync::Arc;

use matrix_sdk::Client;
use tauri::{AppHandle, Emitter};

pub use synara_core::app::account_data::{
    set_global_image_packs, set_room_image_pack, set_user_image_pack, snapshot_global_image_packs,
    snapshot_room_image_packs, snapshot_user_image_pack, NativeAccountDataWakeupKind,
    NativeGlobalImagePacksSnapshot, NativeImagePack, NativeImagePackOwner,
    NativeImagePackUpdateSignal, NativeRoomImagePacksSnapshot, NativeUserImagePackSnapshot,
    AGENT_APPROVAL_HISTORY_UPDATED_EVENT, IMAGE_PACKS_UPDATED_EVENT,
};

pub(crate) fn event_name_for_account_data_wakeup(
    kind: NativeAccountDataWakeupKind,
) -> &'static str {
    match kind {
        NativeAccountDataWakeupKind::ImagePacks => IMAGE_PACKS_UPDATED_EVENT,
        NativeAccountDataWakeupKind::AgentApprovalHistory => AGENT_APPROVAL_HISTORY_UPDATED_EVENT,
    }
}

/// Start the Core owner and emit pack wakeups on the existing Tauri event.
pub fn start(
    client: &Client,
    app: AppHandle,
    session_generation: u64,
) -> Result<NativeImagePackOwner, &'static str> {
    NativeImagePackOwner::start(
        client,
        Arc::new(move |signal: NativeImagePackUpdateSignal| {
            let _ = app.emit(event_name_for_account_data_wakeup(signal.kind), signal);
        }),
        session_generation,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wakeup_kinds_emit_distinct_tauri_events() {
        assert_eq!(
            event_name_for_account_data_wakeup(NativeAccountDataWakeupKind::ImagePacks),
            IMAGE_PACKS_UPDATED_EVENT
        );
        assert_eq!(
            event_name_for_account_data_wakeup(NativeAccountDataWakeupKind::AgentApprovalHistory),
            AGENT_APPROVAL_HISTORY_UPDATED_EVENT
        );
        assert_ne!(
            IMAGE_PACKS_UPDATED_EVENT,
            AGENT_APPROVAL_HISTORY_UPDATED_EVENT
        );
    }
}
