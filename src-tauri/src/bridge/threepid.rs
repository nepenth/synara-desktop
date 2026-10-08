//! Desktop bridges for 3PID/email attachment through `Core::command`.
//! Password stays off JSON and uses `Core::threepid_add_email_password`.

use synara_core::app::user_profile::{
    MatrixThreepidAddResult, MatrixThreepidEmailTokenResult, MatrixThreepidSnapshot,
    MatrixThreepidWriteResult,
};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn threepid_snapshot(
    core: &Core,
) -> Result<MatrixThreepidSnapshot, MatrixAuthCommandError> {
    let payload = core
        .threepid_snapshot()
        .await
        .map_err(map_threepid_core_error)?;
    Ok(payload)
}

pub(crate) async fn threepid_delete(
    core: &Core,
    address: String,
) -> Result<MatrixThreepidWriteResult, MatrixAuthCommandError> {
    let payload = core
        .threepid_delete(synara_core::core_api::MatrixThreepidAddressRequest { address })
        .await
        .map_err(map_threepid_core_error)?;
    Ok(payload)
}

pub(crate) async fn threepid_request_email_token(
    core: &Core,
    email: String,
) -> Result<MatrixThreepidEmailTokenResult, MatrixAuthCommandError> {
    let payload = core
        .threepid_request_email_token(synara_core::core_api::MatrixThreepidEmailRequest { email })
        .await
        .map_err(map_threepid_core_error)?;
    Ok(payload)
}

pub(crate) async fn threepid_add_email(
    core: &Core,
) -> Result<MatrixThreepidAddResult, MatrixAuthCommandError> {
    let payload = core
        .threepid_add_email()
        .await
        .map_err(map_threepid_core_error)?;
    Ok(payload)
}

pub(crate) async fn threepid_add_email_password(
    core: &Core,
    password: String,
) -> Result<MatrixThreepidAddResult, MatrixAuthCommandError> {
    core.threepid_add_email_password(&password)
        .await
        .map_err(map_threepid_core_error)
}

fn map_threepid_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error
        .diagnostic_id
        .as_deref()
        .unwrap_or("v-threepid.snapshot-failed");
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native contact-address request is invalid.",
            diagnostic,
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native contact-address list is unavailable.",
            diagnostic,
        ),
    }
}
