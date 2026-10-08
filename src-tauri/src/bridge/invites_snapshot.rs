//! Desktop bridges for invite snapshot/accept/decline through `Core::command`.

use synara_core::app::room_list::NativeInviteSnapshot;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn invites_snapshot(
    core: &Core,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    let response = core
        .invites_snapshot()
        .await
        .map_err(map_invites_snapshot_core_error)?;
    Ok(response)
}

pub(crate) async fn invites_accept(
    core: &Core,
    room_id: String,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    core.invites_accept(synara_core::core_api::MatrixInviteActionRequest { room_id })
        .await
        .map_err(map_invite_action_core_error)
}

pub(crate) async fn invites_decline(
    core: &Core,
    room_id: String,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    core.invites_decline(synara_core::core_api::MatrixInviteActionRequest { room_id })
        .await
        .map_err(map_invite_action_core_error)
}

pub(crate) async fn invites_report_spam(
    core: &Core,
    room_id: String,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    core.invites_report_spam(synara_core::core_api::MatrixInviteActionRequest { room_id })
        .await
        .map_err(map_invite_action_core_error)
}

pub(crate) async fn invites_block_sender(
    core: &Core,
    room_id: String,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    core.invites_block_sender(synara_core::core_api::MatrixInviteActionRequest { room_id })
        .await
        .map_err(map_invite_action_core_error)
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
