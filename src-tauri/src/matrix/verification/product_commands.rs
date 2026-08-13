use super::*;

#[tauri::command]
pub async fn matrix_verification_list(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeVerificationInbox, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_verification_session(session.as_ref())?;
    Ok(active.verification.list().await)
}

#[tauri::command]
pub async fn matrix_verification_start(
    state: State<'_, MatrixAuthState>,
    device_id: Option<String>,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_verification_session(session.as_ref())?;
    active
        .verification
        .start(&active.client, device_id)
        .await
        .map_err(crate::matrix::verification::live::map_verification_error)
}

#[tauri::command]
pub async fn matrix_verification_accept(
    state: State<'_, MatrixAuthState>,
    flow_id: String,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_verification_session(session.as_ref())?;
    active
        .verification
        .accept(&flow_id)
        .await
        .map_err(crate::matrix::verification::live::map_verification_error)
}

#[tauri::command]
pub async fn matrix_verification_begin_sas(
    state: State<'_, MatrixAuthState>,
    flow_id: String,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_verification_session(session.as_ref())?;
    active
        .verification
        .begin_sas(&flow_id)
        .await
        .map_err(crate::matrix::verification::live::map_verification_error)
}

#[tauri::command]
pub async fn matrix_verification_confirm(
    state: State<'_, MatrixAuthState>,
    flow_id: String,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_verification_session(session.as_ref())?;
    active
        .verification
        .confirm(&flow_id)
        .await
        .map_err(crate::matrix::verification::live::map_verification_error)
}

#[tauri::command]
pub async fn matrix_verification_mismatch(
    state: State<'_, MatrixAuthState>,
    flow_id: String,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_verification_session(session.as_ref())?;
    active
        .verification
        .mismatch(&flow_id)
        .await
        .map_err(crate::matrix::verification::live::map_verification_error)
}

#[tauri::command]
pub async fn matrix_verification_cancel(
    state: State<'_, MatrixAuthState>,
    flow_id: String,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_verification_session(session.as_ref())?;
    active
        .verification
        .cancel(&flow_id)
        .await
        .map_err(crate::matrix::verification::live::map_verification_error)
}

#[tauri::command]
pub async fn matrix_verification_dismiss(
    state: State<'_, MatrixAuthState>,
    flow_id: String,
) -> Result<(), MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_verification_session(session.as_ref())?;
    active
        .verification
        .dismiss(&flow_id)
        .await
        .map_err(crate::matrix::verification::live::map_verification_error)
}
