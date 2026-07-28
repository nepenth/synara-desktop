//! Live native room-key file transfer.
//!
//! The Matrix SDK encrypts/decrypts directly against host filesystem paths.
//! IPC projections contain phase, counts, and basenames only.

use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
    sync::Arc,
};

use matrix_sdk::Client;
use serde::Serialize;
use tokio::sync::Mutex;

use super::{
    RoomKeyTransferFlow, RoomKeyTransferKind, RoomKeyTransferOutcome, RoomKeyTransferPhase,
};
use crate::{
    desktop_file_transfer::{downloads_dir, unique_download_path},
    matrix::auth::product::MatrixAuthCommandError,
};

const EXPORT_FILE_NAME: &str = "synara-room-keys.txt";

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

#[derive(Debug)]
pub struct SelectedRoomKeyImport {
    pub selection_id: u64,
    pub path: PathBuf,
    pub file_label: String,
}

pub fn project_status(
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

pub fn require_passphrase(passphrase: &str) -> Result<(), MatrixAuthCommandError> {
    if passphrase.is_empty() {
        return Err(room_key_error(
            "A passphrase is required for encrypted room-key transfer.",
            "v-crypto.5-passphrase-empty",
        ));
    }
    Ok(())
}

pub async fn pick_import_file() -> Option<(PathBuf, String)> {
    let handle = rfd::AsyncFileDialog::new()
        .add_filter("Encrypted Matrix room keys", &["txt", "key", "keys"])
        .pick_file()
        .await?;
    let path = handle.path().to_path_buf();
    let label = file_label(&path)?;
    Some((path, label))
}

pub async fn export(
    client: &Client,
    session_generation: u64,
    flow: &Arc<Mutex<RoomKeyTransferFlow>>,
    passphrase: &str,
) -> Result<NativeRoomKeyTransferResult, MatrixAuthCommandError> {
    let downloads = downloads_dir().map_err(|_| {
        room_key_error(
            "The encrypted room-key file could not be saved.",
            "v-crypto.5-export-downloads-unavailable",
        )
    })?;
    fs::create_dir_all(&downloads).map_err(|_| export_file_error())?;
    let path = unique_download_path(&downloads, EXPORT_FILE_NAME);
    let label = file_label(&path).ok_or_else(export_file_error)?;
    create_private_file(&path).map_err(|_| export_file_error())?;

    let op_id = begin_transfer(flow, RoomKeyTransferKind::Export, label.clone()).await?;
    mark_in_flight(flow, op_id).await?;

    let mut keys_processed = 0u32;
    let mut rooms = HashSet::new();
    let result = client
        .encryption()
        .export_room_keys(path.clone(), passphrase, |session| {
            keys_processed = keys_processed.saturating_add(1);
            rooms.insert(session.room_id().to_owned());
            true
        })
        .await;

    match result {
        Ok(()) => {
            let rooms_touched = u32::try_from(rooms.len()).unwrap_or(u32::MAX);
            let status = succeed_transfer(
                flow,
                op_id,
                RoomKeyTransferOutcome {
                    kind: RoomKeyTransferKind::Export,
                    keys_processed,
                    rooms_touched,
                },
                session_generation,
            )
            .await?;
            Ok(NativeRoomKeyTransferResult {
                outcome: "complete",
                file_label: label,
                keys_processed,
                rooms_touched,
                total_keys_found: None,
                status,
            })
        }
        Err(_) => {
            let _ = fs::remove_file(path);
            fail_transfer(flow, op_id, "v-crypto.5-export-sdk-failed").await;
            Err(room_key_error(
                "The encrypted room-key export could not be completed.",
                "v-crypto.5-export-sdk-failed",
            ))
        }
    }
}

pub async fn import(
    client: &Client,
    session_generation: u64,
    flow: &Arc<Mutex<RoomKeyTransferFlow>>,
    selected: SelectedRoomKeyImport,
    passphrase: &str,
) -> Result<NativeRoomKeyTransferResult, MatrixAuthCommandError> {
    let SelectedRoomKeyImport {
        path, file_label, ..
    } = selected;
    let op_id = begin_transfer(flow, RoomKeyTransferKind::Import, file_label.clone()).await?;
    mark_in_flight(flow, op_id).await?;

    match client.encryption().import_room_keys(path, passphrase).await {
        Ok(imported) => {
            let keys_processed = u32::try_from(imported.imported_count).unwrap_or(u32::MAX);
            let total_keys_found = u32::try_from(imported.total_count).unwrap_or(u32::MAX);
            let status = succeed_transfer(
                flow,
                op_id,
                RoomKeyTransferOutcome {
                    kind: RoomKeyTransferKind::Import,
                    keys_processed,
                    rooms_touched: 0,
                },
                session_generation,
            )
            .await?;
            Ok(NativeRoomKeyTransferResult {
                outcome: "complete",
                file_label,
                keys_processed,
                rooms_touched: 0,
                total_keys_found: Some(total_keys_found),
                status,
            })
        }
        Err(_) => {
            fail_transfer(flow, op_id, "v-crypto.5-import-sdk-failed").await;
            Err(room_key_error(
                "The room-key file could not be decrypted or imported. Check the file and passphrase.",
                "v-crypto.5-import-sdk-failed",
            ))
        }
    }
}

async fn begin_transfer(
    flow: &Arc<Mutex<RoomKeyTransferFlow>>,
    kind: RoomKeyTransferKind,
    label: String,
) -> Result<u64, MatrixAuthCommandError> {
    let mut flow = flow.lock().await;
    if !flow.is_active() && flow.phase() != RoomKeyTransferPhase::Idle {
        flow.reset_to_idle().map_err(|_| transfer_state_error())?;
    }
    flow.begin(kind, Some(label))
        .map_err(|_| transfer_busy_error())
}

async fn mark_in_flight(
    flow: &Arc<Mutex<RoomKeyTransferFlow>>,
    op_id: u64,
) -> Result<(), MatrixAuthCommandError> {
    flow.lock()
        .await
        .mark_in_flight(op_id)
        .map_err(|_| transfer_state_error())
}

async fn succeed_transfer(
    flow: &Arc<Mutex<RoomKeyTransferFlow>>,
    op_id: u64,
    outcome: RoomKeyTransferOutcome,
    session_generation: u64,
) -> Result<NativeRoomKeyTransferStatus, MatrixAuthCommandError> {
    let mut flow = flow.lock().await;
    flow.succeed(op_id, outcome)
        .map_err(|_| transfer_state_error())?;
    Ok(project_status(session_generation, &flow))
}

async fn fail_transfer(
    flow: &Arc<Mutex<RoomKeyTransferFlow>>,
    op_id: u64,
    diagnostic_id: &'static str,
) {
    let _ = flow.lock().await.fail(op_id, diagnostic_id);
}

fn file_label(path: &Path) -> Option<String> {
    path.file_name()?.to_str().map(str::to_owned)
}

fn create_private_file(path: &Path) -> std::io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)?.sync_all()
}

fn export_file_error() -> MatrixAuthCommandError {
    room_key_error(
        "The encrypted room-key file could not be saved.",
        "v-crypto.5-export-file-failed",
    )
}

fn transfer_busy_error() -> MatrixAuthCommandError {
    room_key_error(
        "Another room-key transfer is already active.",
        "v-crypto.5-transfer-already-active",
    )
}

fn transfer_state_error() -> MatrixAuthCommandError {
    room_key_error(
        "The native room-key transfer state is unavailable.",
        "v-crypto.5-transfer-state-invalid",
    )
}

fn room_key_error(message: &'static str, diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new("Recovery", message, diagnostic_id)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let status = project_status(7, &flow);
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
