//! Desktop bridges for 3PID/email attachment through `Core::command`.
//! Password stays off JSON and uses `Core::threepid_add_email_password`.

use synara_core::app::user_profile::{
    MatrixThreepidAddResult, MatrixThreepidEmailTokenResult, MatrixThreepidSnapshot,
    MatrixThreepidWriteResult,
};
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn threepid_snapshot(
    core: &Core,
) -> Result<MatrixThreepidSnapshot, MatrixAuthCommandError> {
    let payload = dispatch(core, "matrix_threepid_snapshot", serde_json::Value::Null).await?;
    serde_json::from_value(payload).map_err(|_| threepid_response_error())
}

pub(crate) async fn threepid_delete(
    core: &Core,
    address: String,
) -> Result<MatrixThreepidWriteResult, MatrixAuthCommandError> {
    let payload = dispatch(
        core,
        "matrix_threepid_delete",
        serde_json::json!({ "address": address }),
    )
    .await?;
    parse_write(payload)
}

pub(crate) async fn threepid_request_email_token(
    core: &Core,
    email: String,
) -> Result<MatrixThreepidEmailTokenResult, MatrixAuthCommandError> {
    let payload = dispatch(
        core,
        "matrix_threepid_request_email_token",
        serde_json::json!({ "email": email }),
    )
    .await?;
    serde_json::from_value(payload).map_err(|_| threepid_response_error())
}

pub(crate) async fn threepid_add_email(
    core: &Core,
) -> Result<MatrixThreepidAddResult, MatrixAuthCommandError> {
    let payload = dispatch(core, "matrix_threepid_add_email", serde_json::Value::Null).await?;
    serde_json::from_value(payload).map_err(|_| threepid_response_error())
}

pub(crate) async fn threepid_add_email_password(
    core: &Core,
    password: String,
) -> Result<MatrixThreepidAddResult, MatrixAuthCommandError> {
    core.threepid_add_email_password(&password)
        .await
        .map_err(map_threepid_core_error)
}

async fn dispatch(
    core: &Core,
    command: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value, MatrixAuthCommandError> {
    core.command(CommandEnvelope {
        command: command.to_owned(),
        session_generation: READ_ONLY_SESSION_GENERATION,
        request_id: None,
        payload,
    })
    .await
    .map(|response| response.payload)
    .map_err(map_threepid_core_error)
}

fn parse_write(
    payload: serde_json::Value,
) -> Result<MatrixThreepidWriteResult, MatrixAuthCommandError> {
    #[derive(serde::Deserialize)]
    struct Wire {
        status: String,
    }
    let wire: Wire = serde_json::from_value(payload).map_err(|_| threepid_response_error())?;
    if wire.status != "ok" {
        return Err(threepid_response_error());
    }
    Ok(MatrixThreepidWriteResult { status: "ok" })
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

fn threepid_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native contact-address list is unavailable.",
        "v-threepid.snapshot-failed",
    )
}
