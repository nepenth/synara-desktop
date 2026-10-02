//! Native room-key import/export ownership.
//!
//! The transfer flow stores counts and privacy-safe labels only. Live SDK and
//! host-file ownership lives in the desktop shell; room keys, passphrases, file
//! bytes, and import paths never appear in command results.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p8.6-room-key-export.md`

mod error;
mod flow;
mod native;

pub use error::RoomKeyError;
pub use flow::{
    RoomKeyTransferFlow, RoomKeyTransferKind, RoomKeyTransferOutcome, RoomKeyTransferPhase,
};
pub use native::{
    project_room_key_status, NativeRoomKeyFileSelection, NativeRoomKeyTransferKind,
    NativeRoomKeyTransferPhase, NativeRoomKeyTransferResult, NativeRoomKeyTransferStatus,
    EXPORT_FILE_NAME,
};

#[cfg(test)]
mod tests;
