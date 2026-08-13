use super::*;

#[tauri::command]
pub async fn matrix_device_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeDeviceSnapshot, MatrixAuthCommandError> {
    crate::bridge::device_snapshot::device_snapshot(core.inner().as_ref()).await
}

#[tauri::command]
pub async fn matrix_device_rename(
    core: State<'_, Arc<synara_core::Core>>,
    device_id: String,
    display_name: String,
) -> Result<NativeDeviceSnapshot, MatrixAuthCommandError> {
    crate::bridge::device_rename::device_rename(core.inner().as_ref(), device_id, display_name)
        .await
}

#[tauri::command]
pub async fn matrix_device_delete_start(
    core: State<'_, Arc<synara_core::Core>>,
    device_ids: Vec<String>,
) -> Result<NativeDeviceDeleteResult, MatrixAuthCommandError> {
    crate::bridge::device_delete::device_delete_start(core.inner().as_ref(), device_ids).await
}

#[tauri::command]
pub async fn matrix_device_delete_password(
    state: State<'_, MatrixAuthState>,
    operation_id: u64,
    session_generation: u64,
    password: String,
) -> Result<NativeDeviceDeleteResult, MatrixAuthCommandError> {
    let password = zeroize::Zeroizing::new(password);
    if password.is_empty() {
        return Err(map_device_error("v-crypto.7-device-delete-password-empty"));
    }
    let mut session = state.session.lock().await;
    let active = require_device_session_mut(session.as_mut())?;
    let pending = active
        .devices
        .pending_deletion(operation_id, session_generation)
        .map_err(map_device_error)?;
    let user_id = active
        .client
        .user_id()
        .ok_or_else(|| map_device_error("v-crypto.7-device-delete-user-missing"))?;
    let mut auth = uiaa::Password::new(
        uiaa::UserIdentifier::Matrix(uiaa::MatrixUserIdentifier::new(user_id.to_string())),
        password.to_string(),
    );
    auth.session = Some(pending.auth_session.clone());
    let device_ids = pending.device_ids.clone();
    match active
        .client
        .delete_devices(&device_ids, Some(uiaa::AuthData::Password(auth)))
        .await
    {
        Ok(_) => active
            .devices
            .complete_deletion(&device_ids)
            .await
            .map_err(map_device_error),
        Err(error) => {
            let info = error
                .as_uiaa_response()
                .ok_or_else(|| map_device_error("v-crypto.7-device-delete-password-failed"))?;
            let authentication_failed = !info.completed.contains(&uiaa::AuthType::Password);
            active
                .devices
                .refresh_delete_challenge(info, authentication_failed)
                .map_err(map_device_error)
        }
    }
}

#[tauri::command]
pub async fn matrix_device_delete_cancel(
    core: State<'_, Arc<synara_core::Core>>,
    operation_id: u64,
    session_generation: u64,
) -> Result<(), MatrixAuthCommandError> {
    crate::bridge::device_delete::device_delete_cancel(
        core.inner().as_ref(),
        operation_id,
        session_generation,
    )
    .await
}

pub(super) fn map_device_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "v-crypto.7-device-rename-empty"
        | "v-crypto.7-device-delete-selection-empty"
        | "v-crypto.7-device-delete-selection-invalid"
        | "v-crypto.7-device-delete-not-pending"
        | "v-crypto.7-device-delete-operation-mismatch" => (
            "InvalidRequest",
            "The native Matrix device request is invalid.",
        ),
        "v-crypto.7-device-delete-stale-generation" => (
            "StaleSessionGeneration",
            "The native Matrix session changed during device logout.",
        ),
        "v-crypto.7-device-delete-auth-unsupported" => (
            "Forbidden",
            "The homeserver requires an unsupported authentication step for device logout.",
        ),
        _ => ("Unknown", "Native Matrix device management is unavailable."),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}
