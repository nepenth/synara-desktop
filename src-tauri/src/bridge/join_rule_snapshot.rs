//! Desktop bridge for `matrix_room_join_rule_snapshot` through `Core::command`.

use synara_core::app::room_profile::MatrixRoomJoinRuleSnapshot;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const JOIN_RULE_SNAPSHOT_COMMAND: &str = "matrix_room_join_rule_snapshot";

pub(crate) async fn join_rule_snapshot(
    core: &Core,
    room_id: String,
    session_generation: u64,
) -> Result<MatrixRoomJoinRuleSnapshot, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: JOIN_RULE_SNAPSHOT_COMMAND.to_owned(),
            session_generation,
            request_id: None,
            payload: serde_json::json!({
                "roomId": room_id,
                "sessionGeneration": session_generation,
            }),
        })
        .await
        .map_err(map_join_rule_snapshot_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| {
        MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix room join rule is unavailable.",
            "v-send.r-room-profile-join-rule-read-sdk-failed",
        )
    })
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
