//! Desktop bridge for `matrix_backup_status` through `Core::command`.

use synara_core::app::backup::NativeBackupStatus;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn backup_status(
    core: &Core,
) -> Result<NativeBackupStatus, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: "matrix_backup_status".to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .map_err(map_backup_status_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| backup_status_response_error())
}

fn map_backup_status_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.3-backup-requires-session",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "Encryption backup status is unavailable.",
            error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-crypto.3-status-query-failed"),
        ),
    }
}

fn backup_status_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Encryption backup status is unavailable.",
        "v-crypto.3-status-query-failed",
    )
}
