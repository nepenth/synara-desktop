//! Desktop bridges for `in.synara.later` through `Core::command`.

use synara_core::app::account_data::{NativeLaterSnapshot, SynaraLaterItem};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn later_snapshot(
    core: &Core,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    core.later_snapshot().await.map_err(map_later_core_error)
}

pub(crate) async fn later_upsert(
    core: &Core,
    item: SynaraLaterItem,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    core.later_upsert(synara_core::core_api::MatrixLaterUpsertRequest { item })
        .await
        .map_err(map_later_core_error)
}

pub(crate) async fn later_complete(
    core: &Core,
    item_id: String,
    completed_at: Option<f64>,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    core.later_complete(synara_core::core_api::MatrixLaterCompleteRequest {
        item_id,
        completed_at,
    })
    .await
    .map_err(map_later_core_error)
}

pub(crate) async fn later_snooze(
    core: &Core,
    item_id: String,
    due_ts: f64,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    core.later_snooze(synara_core::core_api::MatrixLaterSnoozeRequest { item_id, due_ts })
        .await
        .map_err(map_later_core_error)
}

pub(crate) async fn later_clear_completed(
    core: &Core,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    core.later_clear_completed()
        .await
        .map_err(map_later_core_error)
}

pub(crate) async fn later_mark_reminded(
    core: &Core,
    item_id: String,
    reminded_at: Option<f64>,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    core.later_mark_reminded(synara_core::core_api::MatrixLaterMarkRemindedRequest {
        item_id,
        reminded_at,
    })
    .await
    .map_err(map_later_core_error)
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
