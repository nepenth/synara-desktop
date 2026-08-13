//! Desktop bridges for `in.synara.later` through `Core::command`.

use synara_core::app::account_data::{NativeLaterSnapshot, SynaraLaterItem};
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn later_snapshot(
    core: &Core,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    dispatch_null(core, "matrix_later_snapshot").await
}

pub(crate) async fn later_upsert(
    core: &Core,
    item: SynaraLaterItem,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    dispatch(
        core,
        "matrix_later_upsert",
        serde_json::json!({ "item": item }),
    )
    .await
}

pub(crate) async fn later_complete(
    core: &Core,
    item_id: String,
    completed_at: Option<f64>,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    dispatch(
        core,
        "matrix_later_complete",
        serde_json::json!({
            "itemId": item_id,
            "completedAt": completed_at,
        }),
    )
    .await
}

pub(crate) async fn later_snooze(
    core: &Core,
    item_id: String,
    due_ts: f64,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    dispatch(
        core,
        "matrix_later_snooze",
        serde_json::json!({
            "itemId": item_id,
            "dueTs": due_ts,
        }),
    )
    .await
}

pub(crate) async fn later_clear_completed(
    core: &Core,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    dispatch_null(core, "matrix_later_clear_completed").await
}

pub(crate) async fn later_mark_reminded(
    core: &Core,
    item_id: String,
    reminded_at: Option<f64>,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    dispatch(
        core,
        "matrix_later_mark_reminded",
        serde_json::json!({
            "itemId": item_id,
            "remindedAt": reminded_at,
        }),
    )
    .await
}

async fn dispatch_null(
    core: &Core,
    command: &str,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    dispatch(core, command, serde_json::Value::Null).await
}

async fn dispatch(
    core: &Core,
    command: &str,
    payload: serde_json::Value,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: command.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload,
        })
        .await
        .map_err(map_later_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| later_response_error())
}

fn map_later_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix later/notes request is invalid.",
            error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-timeline-later-invalid-item"),
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix later/notes account data is unavailable.",
            error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-timeline-later-fetch-failed"),
        ),
    }
}

fn later_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix later/notes account data is unavailable.",
        "v-timeline-later-fetch-failed",
    )
}
