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
    state: State<'_, MatrixAuthState>,
    device_ids: Vec<String>,
) -> Result<NativeDeviceDeleteResult, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_device_session_mut(session.as_mut())?;
    active.pending_device_deletion = None;
    let device_ids = validate_device_deletion(active, device_ids).await?;
    match active.client.delete_devices(&device_ids, None).await {
        Ok(_) => complete_device_deletion(active, &device_ids).await,
        Err(error) => {
            let info = error
                .as_uiaa_response()
                .ok_or_else(|| map_device_error("v-crypto.7-device-delete-start-failed"))?;
            retain_device_delete_challenge(active, device_ids, info).await
        }
    }
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
    let pending = validate_pending_device_deletion(active, operation_id, session_generation)?;
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
        Ok(_) => complete_device_deletion(active, &device_ids).await,
        Err(error) => {
            let info = error
                .as_uiaa_response()
                .ok_or_else(|| map_device_error("v-crypto.7-device-delete-password-failed"))?;
            let authentication_failed = !info.completed.contains(&uiaa::AuthType::Password);
            refresh_device_delete_challenge(active, info, authentication_failed).await
        }
    }
}

#[tauri::command]
pub async fn matrix_device_delete_cancel(
    state: State<'_, MatrixAuthState>,
    operation_id: u64,
    session_generation: u64,
) -> Result<(), MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_device_session_mut(session.as_mut())?;
    validate_pending_device_deletion(active, operation_id, session_generation)?;
    active.pending_device_deletion = None;
    Ok(())
}

pub(super) async fn validate_device_deletion(
    active: &ManagedMatrixSession,
    device_ids: Vec<String>,
) -> Result<Vec<matrix_sdk::ruma::OwnedDeviceId>, MatrixAuthCommandError> {
    if device_ids.is_empty() {
        return Err(map_device_error("v-crypto.7-device-delete-selection-empty"));
    }
    let snapshot = live_device_snapshot(&active.client, active.sync.session_generation())
        .await
        .map_err(map_device_error)?;
    let current = snapshot
        .devices
        .iter()
        .find(|device| device.is_current)
        .map(|device| device.device_id.as_str())
        .ok_or_else(|| map_device_error("v-crypto.7-device-delete-current-missing"))?;
    let mut unique = std::collections::BTreeSet::new();
    for device_id in device_ids {
        if device_id.is_empty() || device_id == current || !snapshot.contains(&device_id) {
            return Err(map_device_error(
                "v-crypto.7-device-delete-selection-invalid",
            ));
        }
        unique.insert(matrix_sdk::ruma::OwnedDeviceId::from(device_id));
    }
    Ok(unique.into_iter().collect())
}

pub(super) fn validate_pending_device_deletion(
    active: &ManagedMatrixSession,
    operation_id: u64,
    session_generation: u64,
) -> Result<&PendingDeviceDeletion, MatrixAuthCommandError> {
    if active.sync.session_generation() != session_generation {
        return Err(map_device_error(
            "v-crypto.7-device-delete-stale-generation",
        ));
    }
    let pending = active
        .pending_device_deletion
        .as_ref()
        .ok_or_else(|| map_device_error("v-crypto.7-device-delete-not-pending"))?;
    if pending.session_generation != session_generation {
        return Err(map_device_error(
            "v-crypto.7-device-delete-stale-generation",
        ));
    }
    if pending.operation_id != operation_id {
        return Err(map_device_error(
            "v-crypto.7-device-delete-operation-mismatch",
        ));
    }
    Ok(pending)
}

pub(super) async fn retain_device_delete_challenge(
    active: &mut ManagedMatrixSession,
    device_ids: Vec<matrix_sdk::ruma::OwnedDeviceId>,
    info: &uiaa::UiaaInfo,
) -> Result<NativeDeviceDeleteResult, MatrixAuthCommandError> {
    let operation_id = active
        .next_device_delete_operation_id
        .checked_add(1)
        .ok_or_else(|| map_device_error("v-crypto.7-device-delete-operation-overflow"))?;
    active.next_device_delete_operation_id = operation_id;
    install_device_delete_challenge(active, operation_id, device_ids, info, false).await
}

pub(super) async fn refresh_device_delete_challenge(
    active: &mut ManagedMatrixSession,
    info: &uiaa::UiaaInfo,
    authentication_failed: bool,
) -> Result<NativeDeviceDeleteResult, MatrixAuthCommandError> {
    let pending = active
        .pending_device_deletion
        .take()
        .ok_or_else(|| map_device_error("v-crypto.7-device-delete-not-pending"))?;
    install_device_delete_challenge(
        active,
        pending.operation_id,
        pending.device_ids,
        info,
        authentication_failed,
    )
    .await
}

pub(super) async fn install_device_delete_challenge(
    active: &mut ManagedMatrixSession,
    operation_id: u64,
    device_ids: Vec<matrix_sdk::ruma::OwnedDeviceId>,
    info: &uiaa::UiaaInfo,
    authentication_failed: bool,
) -> Result<NativeDeviceDeleteResult, MatrixAuthCommandError> {
    let auth_session = info
        .session
        .clone()
        .ok_or_else(|| map_device_error("v-crypto.7-device-delete-auth-session-missing"))?;
    let available = supported_delete_authentication(info);
    let authentication = if available
        .contains(&crate::matrix::devices::NativeDeviceDeleteAuthentication::Password)
    {
        crate::matrix::devices::NativeDeviceDeleteAuthentication::Password
    } else {
        return Err(map_device_error(
            "v-crypto.7-device-delete-auth-unsupported",
        ));
    };
    active.pending_device_deletion = Some(PendingDeviceDeletion {
        operation_id,
        session_generation: active.sync.session_generation(),
        device_ids,
        auth_session,
    });
    Ok(NativeDeviceDeleteResult::AuthenticationRequired {
        challenge: NativeDeviceDeleteChallenge {
            operation_id,
            session_generation: active.sync.session_generation(),
            authentication,
            authentication_failed,
        },
    })
}

pub(super) async fn complete_device_deletion(
    active: &mut ManagedMatrixSession,
    deleted: &[matrix_sdk::ruma::OwnedDeviceId],
) -> Result<NativeDeviceDeleteResult, MatrixAuthCommandError> {
    let snapshot = live_device_snapshot(&active.client, active.sync.session_generation())
        .await
        .map_err(map_device_error)?;
    if deleted
        .iter()
        .any(|device_id| snapshot.contains(device_id.as_str()))
    {
        return Err(map_device_error(
            "v-crypto.7-device-delete-readback-incomplete",
        ));
    }
    active.pending_device_deletion = None;
    Ok(NativeDeviceDeleteResult::Complete { snapshot })
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
