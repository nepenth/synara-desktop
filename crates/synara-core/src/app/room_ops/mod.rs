//! P6.9 — Room membership / lifecycle ops queue foundation (harness).
//!
//! Tracks create, join, leave, invite, kick, ban, unban, forget intents.
//! No SDK network, no production Tauri commands, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p6.9-room-ops.md`

mod create;
mod encrypted_state;
mod error;
mod ipc;
mod queue;

pub use create::build_room_create_request;
pub use encrypted_state::{
    encrypted_state_events_setting_enabled, encryption_content_requests_encrypted_state,
    event_is_undecrypted_encrypted_state, is_call_room_type, packed_encrypted_state_type,
    power_level_tags_readback_source, room_encryption_content, room_encryption_enable_write,
    set_encrypted_state_events_setting_enabled, should_create_with_encrypted_state,
    PowerLevelTagsReadbackSource, RoomEncryptionEnableWrite,
};
pub use error::RoomOpsError;
pub use ipc::{
    MatrixRoomCreateContent, MatrixRoomCreatePowerLevels, MatrixRoomCreatePreset,
    MatrixRoomCreateRequest, MatrixRoomCreateVisibility,
};
pub use queue::{
    LocalOpId, RoomOp, RoomOpKind, RoomOpState, RoomOpsQueue, MAX_CREATE_NAME_CHARS,
    MAX_REASON_CHARS, MAX_TRACKED_OPS,
};

#[cfg(test)]
mod tests;
