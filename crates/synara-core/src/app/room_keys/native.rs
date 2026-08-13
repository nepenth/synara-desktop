//! Credential-free V-CRYPTO.5 room-key transfer presentation DTOs.
//!
//! Live Client/file I/O stays in the desktop shell.

use serde::Serialize;

use super::flow::{RoomKeyTransferFlow, RoomKeyTransferKind, RoomKeyTransferPhase};

pub const EXPORT_FILE_NAME: &str = "synara-room-keys.txt";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeRoomKeyTransferKind {
    Export,
    Import,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeRoomKeyTransferPhase {
    Idle,
    Preparing,
    InFlight,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomKeyTransferStatus {
    pub session_generation: u64,
    pub kind: Option<NativeRoomKeyTransferKind>,
    pub phase: NativeRoomKeyTransferPhase,
    pub progress_percent: Option<u8>,
    pub keys_processed: u32,
    pub rooms_touched: u32,
    pub file_label: Option<String>,
    pub failure_diagnostic_id: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomKeyTransferResult {
    pub outcome: &'static str,
    pub file_label: String,
    pub keys_processed: u32,
    pub rooms_touched: u32,
    pub total_keys_found: Option<u32>,
    pub status: NativeRoomKeyTransferStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomKeyFileSelection {
    pub selection_id: u64,
    pub file_label: String,
}

pub fn project_room_key_status(
    session_generation: u64,
    flow: &RoomKeyTransferFlow,
) -> NativeRoomKeyTransferStatus {
    NativeRoomKeyTransferStatus {
        session_generation,
        kind: flow.kind().map(|kind| match kind {
            RoomKeyTransferKind::Export => NativeRoomKeyTransferKind::Export,
            RoomKeyTransferKind::Import => NativeRoomKeyTransferKind::Import,
        }),
        phase: match flow.phase() {
            RoomKeyTransferPhase::Idle => NativeRoomKeyTransferPhase::Idle,
            RoomKeyTransferPhase::Preparing => NativeRoomKeyTransferPhase::Preparing,
            RoomKeyTransferPhase::InFlight => NativeRoomKeyTransferPhase::InFlight,
            RoomKeyTransferPhase::Succeeded => NativeRoomKeyTransferPhase::Succeeded,
            RoomKeyTransferPhase::Failed => NativeRoomKeyTransferPhase::Failed,
            RoomKeyTransferPhase::Cancelled => NativeRoomKeyTransferPhase::Cancelled,
        },
        progress_percent: flow.progress_percent(),
        keys_processed: flow.keys_processed(),
        rooms_touched: flow.rooms_touched(),
        file_label: flow.file_label().map(str::to_owned),
        failure_diagnostic_id: flow.failure_diagnostic_id(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::room_keys::flow::RoomKeyTransferOutcome;

    #[test]
    fn status_and_results_are_privacy_safe() {
        let mut flow = RoomKeyTransferFlow::new(7);
        let op = flow
            .begin(
                RoomKeyTransferKind::Export,
                Some("synara-room-keys.txt".to_owned()),
            )
            .unwrap();
        flow.mark_in_flight(op).unwrap();
        flow.succeed(
            op,
            RoomKeyTransferOutcome {
                kind: RoomKeyTransferKind::Export,
                keys_processed: 9,
                rooms_touched: 2,
            },
        )
        .unwrap();
        let status = project_room_key_status(7, &flow);
        let result = NativeRoomKeyTransferResult {
            outcome: "complete",
            file_label: "synara-room-keys.txt".to_owned(),
            keys_processed: 9,
            rooms_touched: 2,
            total_keys_found: None,
            status,
        };
        let json = serde_json::to_string(&result).unwrap().to_ascii_lowercase();
        for forbidden in [
            "session_key",
            "ciphertext",
            "\"passphrase\":",
            "\"path\":",
            "access_token",
        ] {
            assert!(!json.contains(forbidden), "{json}");
        }
    }
}
