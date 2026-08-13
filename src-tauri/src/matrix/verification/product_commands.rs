use super::*;

#[tauri::command]
pub async fn matrix_verification_list(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeVerificationInbox, MatrixAuthCommandError> {
    crate::bridge::verification_list::verification_list(core.inner().as_ref()).await
}

#[tauri::command]
pub async fn matrix_verification_start(
    core: State<'_, Arc<synara_core::Core>>,
    device_id: Option<String>,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    crate::bridge::verification_start::verification_start(core.inner().as_ref(), device_id).await
}

#[tauri::command]
pub async fn matrix_verification_accept(
    core: State<'_, Arc<synara_core::Core>>,
    flow_id: String,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    crate::bridge::verification_accept::verification_accept(core.inner().as_ref(), flow_id).await
}

#[tauri::command]
pub async fn matrix_verification_begin_sas(
    core: State<'_, Arc<synara_core::Core>>,
    flow_id: String,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    crate::bridge::verification_begin_sas::verification_begin_sas(core.inner().as_ref(), flow_id)
        .await
}

#[tauri::command]
pub async fn matrix_verification_confirm(
    core: State<'_, Arc<synara_core::Core>>,
    flow_id: String,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    crate::bridge::verification_confirm::verification_confirm(core.inner().as_ref(), flow_id).await
}

#[tauri::command]
pub async fn matrix_verification_mismatch(
    core: State<'_, Arc<synara_core::Core>>,
    flow_id: String,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    crate::bridge::verification_mismatch::verification_mismatch(core.inner().as_ref(), flow_id)
        .await
}

#[tauri::command]
pub async fn matrix_verification_cancel(
    core: State<'_, Arc<synara_core::Core>>,
    flow_id: String,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    crate::bridge::verification_cancel::verification_cancel(core.inner().as_ref(), flow_id).await
}

#[tauri::command]
pub async fn matrix_verification_dismiss(
    core: State<'_, Arc<synara_core::Core>>,
    flow_id: String,
) -> Result<(), MatrixAuthCommandError> {
    crate::bridge::verification_dismiss::verification_dismiss(core.inner().as_ref(), flow_id).await
}
