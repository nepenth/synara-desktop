//! Desktop bridge for join-rule snapshot and write through `Core::command`.

use synara_core::app::room_profile::MatrixRoomJoinRuleSnapshot;
use synara_core::app::user_profile::MatrixProfileWriteResult;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn join_rule_snapshot(
    core: &Core,
    room_id: String,
    session_generation: u64,
) -> Result<MatrixRoomJoinRuleSnapshot, MatrixAuthCommandError> {
    let response = core
        .room_join_rule_snapshot(synara_core::core_api::MatrixRoomJoinRuleSnapshotRequest {
            room_id,
            session_generation,
        })
        .await
        .map_err(map_join_rule_snapshot_core_error)?;
    Ok(response)
}

pub(crate) async fn set_join_rule(
    core: &Core,
    room_id: String,
    join_rule: String,
    allow_room_ids: Option<Vec<String>>,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    core.room_set_join_rule(synara_core::core_api::MatrixRoomSetJoinRuleRequest {
        room_id,
        join_rule,
        allow_room_ids,
    })
    .await
    .map_err(map_join_rule_snapshot_core_error)?;
    Ok(MatrixProfileWriteResult { status: "ok" })
}

fn map_join_rule_snapshot_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-send.r-room-profile-join-rule-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix room join-rule request is invalid.",
            "v-send.r-room-profile-join-rule-invalid",
        ),
        MatrixIpcErrorCategory::StaleSessionGeneration => MatrixAuthCommandError::new(
            "Forbidden",
            "The native Matrix room join-rule session is stale.",
            "v-send.r-room-profile-join-rule-stale-generation",
        ),
        _ => {
            let (code, diagnostic_id) = match error.diagnostic_id.as_deref() {
                Some("v-send.r-room-profile-join-rule-room-not-found") => {
                    return MatrixAuthCommandError::new(
                        "NotFound",
                        "The native Matrix room is not available.",
                        "v-send.r-room-profile-join-rule-room-not-found",
                    );
                }
                Some("v-send.r-room-profile-join-rule-room-state-unavailable") => (
                    "Unknown",
                    "v-send.r-room-profile-join-rule-room-state-unavailable",
                ),
                Some("v-send.r-room-profile-join-rule-deserialize-failed") => (
                    "Unknown",
                    "v-send.r-room-profile-join-rule-deserialize-failed",
                ),
                Some("v-send.r-room-profile-join-rule-unsupported") => {
                    ("Unknown", "v-send.r-room-profile-join-rule-unsupported")
                }
                Some("v-send.r-room-profile-join-rule-set-sdk-failed") => {
                    return MatrixAuthCommandError::new(
                        "Unknown",
                        "The native Matrix room join rule could not be updated.",
                        "v-send.r-room-profile-join-rule-set-sdk-failed",
                    );
                }
                _ => ("Unknown", "v-send.r-room-profile-join-rule-read-sdk-failed"),
            };
            MatrixAuthCommandError::new(
                code,
                "The native Matrix room join rule is unavailable.",
                diagnostic_id,
            )
        }
    }
}
