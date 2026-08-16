//! Desktop bridges for timeline reaction mutations through `Core::command`.

use synara_core::app::timeline::NativeReactionMutationResult;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const REACTION_TOGGLE_COMMAND: &str = "matrix_timeline_reaction_toggle";
const REACTION_ENSURE_COMMAND: &str = "matrix_reaction_ensure";
const REACTION_REDACT_COMMAND: &str = "matrix_reaction_redact";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn reaction_toggle(
    core: &Core,
    room_id: String,
    event_id: String,
    key: String,
) -> Result<NativeReactionMutationResult, MatrixAuthCommandError> {
    dispatch_reaction_key(core, REACTION_TOGGLE_COMMAND, room_id, event_id, key).await
}

pub(crate) async fn reaction_ensure(
    core: &Core,
    room_id: String,
    event_id: String,
    key: String,
) -> Result<NativeReactionMutationResult, MatrixAuthCommandError> {
    dispatch_reaction_key(core, REACTION_ENSURE_COMMAND, room_id, event_id, key).await
}

pub(crate) async fn reaction_redact(
    core: &Core,
    room_id: String,
    target_event_id: String,
    reaction_event_id: String,
    key: String,
) -> Result<NativeReactionMutationResult, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: REACTION_REDACT_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "roomId": room_id,
                "targetEventId": target_event_id,
                "reactionEventId": reaction_event_id,
                "key": key,
            }),
        })
        .await
        .map_err(map_reaction_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| reaction_response_error())
}

async fn dispatch_reaction_key(
    core: &Core,
    command: &str,
    room_id: String,
    event_id: String,
    key: String,
) -> Result<NativeReactionMutationResult, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: command.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
                "key": key,
            }),
        })
        .await
        .map_err(map_reaction_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| reaction_response_error())
}

fn map_reaction_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.3-timeline-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix reaction operation could not be completed.",
            "v-send.2-reaction-invalid-key",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix reaction operation could not be completed.",
            "v-send.2-reaction-toggle-failed",
        ),
    }
}

fn reaction_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix reaction operation could not be completed.",
        "v-send.2-reaction-toggle-failed",
    )
}
