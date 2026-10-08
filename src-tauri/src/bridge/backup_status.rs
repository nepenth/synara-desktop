//! Desktop bridge for `matrix_backup_status` through `Core::command`.

use synara_core::app::backup::NativeBackupStatus;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn backup_status(
    core: &Core,
) -> Result<NativeBackupStatus, MatrixAuthCommandError> {
    let response = core
        .backup_status()
        .await
        .map_err(map_backup_status_core_error)?;
    Ok(response)
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
