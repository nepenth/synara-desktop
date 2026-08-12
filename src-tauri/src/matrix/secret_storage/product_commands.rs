use super::*;

#[tauri::command]
pub async fn matrix_secret_storage_status(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeSecretStorageStatus, MatrixAuthCommandError> {
    crate::bridge::secret_storage_status::secret_storage_status(core.inner().as_ref()).await
}

#[tauri::command]
pub async fn matrix_secret_storage_bootstrap(
    state: State<'_, MatrixAuthState>,
    mut passphrase: String,
) -> Result<NativeSecretStorageOperationResult, MatrixAuthCommandError> {
    let result = matrix_secret_storage_bootstrap_inner(&state, &passphrase).await;
    passphrase.zeroize();
    result
}

pub(super) async fn matrix_secret_storage_bootstrap_inner(
    state: &State<'_, MatrixAuthState>,
    passphrase: &str,
) -> Result<NativeSecretStorageOperationResult, MatrixAuthCommandError> {
    require_secret_storage_input(passphrase, "v-crypto.4-bootstrap-passphrase-empty")?;
    let session = state.session.lock().await;
    let active = require_secret_storage_session(session.as_ref())?;
    live_secret_storage::bootstrap(&active.client, active.sync.session_generation(), passphrase)
        .await
}

#[tauri::command]
pub async fn matrix_secret_storage_unlock(
    state: State<'_, MatrixAuthState>,
    mut recovery_secret: String,
) -> Result<NativeSecretStorageOperationResult, MatrixAuthCommandError> {
    let result = matrix_secret_storage_unlock_inner(&state, &recovery_secret).await;
    recovery_secret.zeroize();
    result
}

pub(super) async fn matrix_secret_storage_unlock_inner(
    state: &State<'_, MatrixAuthState>,
    recovery_secret: &str,
) -> Result<NativeSecretStorageOperationResult, MatrixAuthCommandError> {
    require_secret_storage_input(recovery_secret, "v-crypto.4-unlock-secret-empty")?;
    let session = state.session.lock().await;
    let active = require_secret_storage_session(session.as_ref())?;
    live_secret_storage::unlock(
        &active.client,
        active.sync.session_generation(),
        recovery_secret,
    )
    .await
}

#[tauri::command]
pub async fn matrix_secret_storage_reset(
    state: State<'_, MatrixAuthState>,
    mut passphrase: String,
) -> Result<NativeSecretStorageOperationResult, MatrixAuthCommandError> {
    let result = matrix_secret_storage_reset_inner(&state, &passphrase).await;
    passphrase.zeroize();
    result
}

pub(super) async fn matrix_secret_storage_reset_inner(
    state: &State<'_, MatrixAuthState>,
    passphrase: &str,
) -> Result<NativeSecretStorageOperationResult, MatrixAuthCommandError> {
    require_secret_storage_input(passphrase, "v-crypto.4-reset-passphrase-empty")?;
    let session = state.session.lock().await;
    let active = require_secret_storage_session(session.as_ref())?;
    live_secret_storage::reset(&active.client, active.sync.session_generation(), passphrase).await
}

pub(super) fn require_secret_storage_input(
    value: &str,
    diagnostic_id: &'static str,
) -> Result<(), MatrixAuthCommandError> {
    if value.is_empty() {
        Err(MatrixAuthCommandError::new(
            "InvalidRequest",
            "A recovery key or passphrase is required.",
            diagnostic_id,
        ))
    } else {
        Ok(())
    }
}

pub(super) fn require_recovery_secret(recovery_secret: &str) -> Result<(), MatrixAuthCommandError> {
    if recovery_secret.is_empty() {
        Err(MatrixAuthCommandError::new(
            "InvalidRequest",
            "A recovery key or passphrase is required.",
            "v-crypto.3-recovery-secret-empty",
        ))
    } else {
        Ok(())
    }
}
