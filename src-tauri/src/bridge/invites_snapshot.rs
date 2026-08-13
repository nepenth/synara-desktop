//! Desktop bridges for invite snapshot/accept/decline through `Core::command`.

use synara_core::app::room_list::NativeInviteSnapshot;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn invites_snapshot(
    core: &Core,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: "matrix_invites_snapshot".to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .map_err(map_invites_snapshot_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| invites_snapshot_response_error())
}

pub(crate) async fn invites_accept(
    core: &Core,
    room_id: String,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    invite_action(core, "matrix_invites_accept", room_id).await
}

pub(crate) async fn invites_decline(
    core: &Core,
    room_id: String,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    invite_action(core, "matrix_invites_decline", room_id).await
}

async fn invite_action(
    core: &Core,
    command: &str,
    room_id: String,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: command.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({ "roomId": room_id }),
        })
        .await
        .map_err(map_invite_action_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| invites_snapshot_response_error())
}

fn map_invites_snapshot_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-rooms.1-invites-requires-session",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix invite inbox is unavailable.",
            error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-rooms.1-invite-member-read-failed"),
        ),
    }
}

fn map_invite_action_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-rooms.1-invites-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => {
            let diagnostic = error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-rooms.1-invite-invalid-room");
            let (code, message) = match diagnostic {
                "v-rooms.1-invite-not-found" | "v-rooms.1-invite-member-missing" => (
                    "NotFound",
                    "The native Matrix invitation is no longer available.",
                ),
                _ => (
                    "InvalidRequest",
                    "The native Matrix invite request is invalid.",
                ),
            };
            MatrixAuthCommandError::new(code, message, diagnostic)
        }
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix invite operation could not be completed.",
            error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-rooms.1-invite-member-read-failed"),
        ),
    }
}

fn invites_snapshot_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix invite inbox is unavailable.",
        "v-rooms.1-invite-member-read-failed",
    )
}
