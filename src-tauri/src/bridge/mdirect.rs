//! Desktop bridges for `m.direct` through `Core::command`.

use synara_core::app::account_data::{NativeMDirectMutationResult, NativeMDirectSnapshot};
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn mdirect_snapshot(
    core: &Core,
) -> Result<NativeMDirectSnapshot, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: "matrix_mdirect_snapshot".to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .map_err(map_mdirect_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| mdirect_response_error())
}

pub(crate) async fn mdirect_add(
    core: &Core,
    room_id: String,
    user_id: String,
) -> Result<NativeMDirectMutationResult, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: "matrix_mdirect_add".to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "roomId": room_id,
                "userId": user_id,
            }),
        })
        .await
        .map_err(map_mdirect_core_error)?;
    parse_mutation(response.payload)
}

pub(crate) async fn mdirect_remove(
    core: &Core,
    room_id: String,
) -> Result<NativeMDirectMutationResult, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: "matrix_mdirect_remove".to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({ "roomId": room_id }),
        })
        .await
        .map_err(map_mdirect_core_error)?;
    parse_mutation(response.payload)
}

fn parse_mutation(
    payload: serde_json::Value,
) -> Result<NativeMDirectMutationResult, MatrixAuthCommandError> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Wire {
        room_id: String,
        status: String,
    }
    let wire: Wire = serde_json::from_value(payload).map_err(|_| mdirect_response_error())?;
    if wire.status != "updated" {
        return Err(mdirect_response_error());
    }
    Ok(NativeMDirectMutationResult {
        room_id: wire.room_id,
        status: "updated",
    })
}

fn map_mdirect_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => {
            let diagnostic = error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-rooms.5-mdirect-invalid-room");
            MatrixAuthCommandError::new(
                "InvalidRequest",
                "The native Matrix direct-room request is invalid.",
                diagnostic,
            )
        }
        _ => {
            let diagnostic = error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-rooms.5-mdirect-fetch-failed");
            MatrixAuthCommandError::new(
                "Unknown",
                "The native Matrix direct-room map is unavailable.",
                diagnostic,
            )
        }
    }
}

fn mdirect_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix direct-room map is unavailable.",
        "v-rooms.5-mdirect-fetch-failed",
    )
}
