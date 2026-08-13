use super::*;

#[tauri::command]
pub async fn matrix_backup_status(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeBackupStatus, MatrixAuthCommandError> {
    crate::bridge::backup_status::backup_status(core.inner().as_ref()).await
}

#[tauri::command]
pub async fn matrix_backup_setup(
    state: State<'_, MatrixAuthState>,
    mut passphrase: String,
) -> Result<NativeBackupOperationResult, MatrixAuthCommandError> {
    let result = matrix_backup_setup_inner(&state, &passphrase).await;
    passphrase.zeroize();
    result
}

pub(super) async fn matrix_backup_setup_inner(
    state: &State<'_, MatrixAuthState>,
    passphrase: &str,
) -> Result<NativeBackupOperationResult, MatrixAuthCommandError> {
    if passphrase.is_empty() {
        return Err(MatrixAuthCommandError::new(
            "InvalidRequest",
            "A recovery passphrase is required to set up encryption backup.",
            "v-crypto.3-setup-passphrase-empty",
        ));
    }
    let session = state.session.lock().await;
    let active = require_backup_session(session.as_ref())?;
    live_backup::setup(&active.client, active.sync.session_generation(), passphrase).await
}

#[tauri::command]
pub async fn matrix_backup_restore(
    state: State<'_, MatrixAuthState>,
    mut recovery_secret: String,
) -> Result<NativeBackupOperationResult, MatrixAuthCommandError> {
    let result = matrix_backup_restore_inner(&state, &recovery_secret).await;
    recovery_secret.zeroize();
    result
}

pub(super) async fn matrix_backup_restore_inner(
    state: &State<'_, MatrixAuthState>,
    recovery_secret: &str,
) -> Result<NativeBackupOperationResult, MatrixAuthCommandError> {
    require_recovery_secret(recovery_secret)?;
    let session = state.session.lock().await;
    let active = require_backup_session(session.as_ref())?;
    live_backup::restore(
        &active.client,
        active.sync.session_generation(),
        recovery_secret,
    )
    .await
}

#[tauri::command]
pub async fn matrix_backup_repair(
    state: State<'_, MatrixAuthState>,
    mut recovery_secret: String,
) -> Result<NativeBackupOperationResult, MatrixAuthCommandError> {
    let result = matrix_backup_repair_inner(&state, &recovery_secret).await;
    recovery_secret.zeroize();
    result
}

pub(super) async fn matrix_backup_repair_inner(
    state: &State<'_, MatrixAuthState>,
    recovery_secret: &str,
) -> Result<NativeBackupOperationResult, MatrixAuthCommandError> {
    require_recovery_secret(recovery_secret)?;
    let session = state.session.lock().await;
    let active = require_backup_session(session.as_ref())?;
    live_backup::repair(
        &active.client,
        active.sync.session_generation(),
        recovery_secret,
    )
    .await
}
