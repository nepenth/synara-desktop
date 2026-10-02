//! Desktop backup error/presentation adapters; SDK semantics live in Core.
use crate::matrix::auth::product::MatrixAuthCommandError;
use matrix_sdk::Client;
pub use synara_core::app::backup::{NativeBackupOperationResult, NativeBackupStatus};

pub async fn setup(
    client: &Client,
    session_generation: u64,
    passphrase: &str,
) -> Result<NativeBackupOperationResult, MatrixAuthCommandError> {
    synara_core::app::backup::setup(client, session_generation, passphrase)
        .await
        .map_err(map_operation_error)
}

pub async fn restore(
    client: &Client,
    session_generation: u64,
    recovery_secret: &str,
) -> Result<NativeBackupOperationResult, MatrixAuthCommandError> {
    synara_core::app::backup::restore_operation(client, session_generation, recovery_secret)
        .await
        .map_err(map_operation_error)
}

pub async fn repair(
    client: &Client,
    session_generation: u64,
    recovery_secret: &str,
) -> Result<NativeBackupOperationResult, MatrixAuthCommandError> {
    synara_core::app::backup::repair(client, session_generation, recovery_secret)
        .await
        .map_err(map_operation_error)
}

fn map_operation_error(diagnostic: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic {
        "v-crypto.3-setup-existing-backup" => (
            "InvalidRequest",
            "An existing encryption backup must be restored before setup can continue.",
        ),
        "v-crypto.3-restore-rejected" => (
            "Forbidden",
            "The recovery key or passphrase was rejected. Check it and try again.",
        ),
        "v-crypto.3-repair-rejected" => (
            "Forbidden",
            "Encryption backup repair failed. Check your recovery key or passphrase and try again.",
        ),
        _ => (
            "Unknown",
            "Native encryption backup could not be activated.",
        ),
    };
    backup_error(code, message, diagnostic)
}
fn backup_error(
    code: &'static str,
    message: &'static str,
    diagnostic: &'static str,
) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(code, message, diagnostic)
}
