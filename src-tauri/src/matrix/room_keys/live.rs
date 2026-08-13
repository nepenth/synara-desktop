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
use tokio::sync::Mutex;

use super::{
    RoomKeyTransferFlow, RoomKeyTransferKind, RoomKeyTransferOutcome, RoomKeyTransferPhase,
};
use crate::{
    desktop_file_transfer::{downloads_dir, unique_download_path},
    matrix::auth::product::MatrixAuthCommandError,
};

pub use synara_core::app::room_keys::{
    project_room_key_status, NativeRoomKeyFileSelection, NativeRoomKeyTransferKind,
    NativeRoomKeyTransferPhase, NativeRoomKeyTransferResult, NativeRoomKeyTransferStatus,
    EXPORT_FILE_NAME,
};

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
    project_room_key_status(session_generation, flow)
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
    let (destination, op_id) = prepare_export_destination(flow, path, label.clone()).await?;

    let mut keys_processed = 0u32;
    let mut rooms = HashSet::new();
    let result = client
        .encryption()
        .export_room_keys(destination.path().to_path_buf(), passphrase, |session| {
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
            destination.persist();
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
            fail_transfer(flow, op_id, "v-crypto.5-export-sdk-failed").await;
            Err(room_key_error(
                "The encrypted room-key export could not be completed.",
                "v-crypto.5-export-sdk-failed",
            ))
        }
    }
}

async fn prepare_export_destination(
    flow: &Arc<Mutex<RoomKeyTransferFlow>>,
    path: PathBuf,
    file_label: String,
) -> Result<(PendingExportFile, u64), MatrixAuthCommandError> {
    let destination = PendingExportFile::create(path).map_err(|_| export_file_error())?;
    let op_id = begin_transfer(flow, RoomKeyTransferKind::Export, file_label).await?;
    mark_in_flight(flow, op_id).await?;
    Ok((destination, op_id))
}

pub async fn import(
    client: &Client,
    session_generation: u64,
    flow: &Arc<Mutex<RoomKeyTransferFlow>>,
    selected: &SelectedRoomKeyImport,
    passphrase: &str,
) -> Result<NativeRoomKeyTransferResult, MatrixAuthCommandError> {
    let op_id = begin_transfer(
        flow,
        RoomKeyTransferKind::Import,
        selected.file_label.clone(),
    )
    .await?;
    mark_in_flight(flow, op_id).await?;

    match client
        .encryption()
        .import_room_keys(selected.path.clone(), passphrase)
        .await
    {
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
                file_label: selected.file_label.clone(),
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

#[derive(Debug)]
struct PendingExportFile {
    path: PathBuf,
    persist: bool,
}

impl PendingExportFile {
    fn create(path: PathBuf) -> std::io::Result<Self> {
        create_private_file(&path)?;
        Ok(Self {
            path,
            persist: false,
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn persist(mut self) {
        self.persist = true;
    }
}

impl Drop for PendingExportFile {
    fn drop(&mut self) {
        if !self.persist {
            let _ = fs::remove_file(&self.path);
        }
    }
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

    fn temporary_export_path(test_name: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "synara-vcrypto5-{test_name}-{}-{nonce}.keys",
            std::process::id()
        ))
    }

    #[tokio::test]
    async fn busy_export_removes_precreated_private_destination() {
        let flow = Arc::new(Mutex::new(RoomKeyTransferFlow::new(7)));
        let active = begin_transfer(&flow, RoomKeyTransferKind::Import, "active.keys".to_owned())
            .await
            .unwrap();
        mark_in_flight(&flow, active).await.unwrap();

        let path = temporary_export_path("busy-cleanup");
        let error =
            prepare_export_destination(&flow, path.clone(), "synara-room-keys.txt".to_owned())
                .await
                .unwrap_err();
        assert_eq!(error.diagnostic_id, "v-crypto.5-transfer-already-active");
        assert!(!path.exists(), "busy rejection left an export file behind");
    }

    #[test]
    fn persisted_export_destination_survives_guard_drop() {
        let path = temporary_export_path("persist");
        PendingExportFile::create(path.clone()).unwrap().persist();
        assert!(path.exists());
        fs::remove_file(path).unwrap();
    }

    #[tokio::test]
    async fn failed_import_flow_can_retry_through_in_flight() {
        let flow = Arc::new(Mutex::new(RoomKeyTransferFlow::new(7)));
        let first = begin_transfer(&flow, RoomKeyTransferKind::Import, "backup.keys".to_owned())
            .await
            .unwrap();
        mark_in_flight(&flow, first).await.unwrap();
        assert_eq!(flow.lock().await.phase(), RoomKeyTransferPhase::InFlight);
        fail_transfer(&flow, first, "v-crypto.5-import-sdk-failed").await;
        assert_eq!(flow.lock().await.phase(), RoomKeyTransferPhase::Failed);

        let retry = begin_transfer(&flow, RoomKeyTransferKind::Import, "backup.keys".to_owned())
            .await
            .unwrap();
        mark_in_flight(&flow, retry).await.unwrap();
        assert_ne!(retry, first);
        assert_eq!(flow.lock().await.phase(), RoomKeyTransferPhase::InFlight);
    }
}
