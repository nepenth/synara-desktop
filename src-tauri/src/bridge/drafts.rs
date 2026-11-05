//! Desktop bridges for encrypted `in.synara.draft` sync through the typed Core.

use synara_core::app::drafts::{
    NativeDraftResolution, NativeDraftSyncStatus, NativeLocalDraftState,
};
use synara_core::core_api::{
    MatrixDraftResolveRequest, MatrixDraftSaveRequest, MatrixDraftSetEnabledRequest,
    MatrixDraftSlotRequest, NativeDraftDeleteAllResult, NativeDraftWriteResult,
};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn drafts_status(
    core: &Core,
) -> Result<NativeDraftSyncStatus, MatrixAuthCommandError> {
    core.drafts_status().await.map_err(map_drafts_core_error)
}

pub(crate) async fn drafts_set_enabled(
    core: &Core,
    enabled: bool,
) -> Result<NativeDraftSyncStatus, MatrixAuthCommandError> {
    core.drafts_set_enabled(MatrixDraftSetEnabledRequest { enabled })
        .await
        .map_err(map_drafts_core_error)
}

pub(crate) async fn drafts_save(
    core: &Core,
    request: MatrixDraftSaveRequest,
) -> Result<NativeDraftWriteResult, MatrixAuthCommandError> {
    core.drafts_save(request)
        .await
        .map_err(map_drafts_core_error)
}

pub(crate) async fn drafts_clear(
    core: &Core,
    room_id: String,
    slot: String,
) -> Result<NativeDraftWriteResult, MatrixAuthCommandError> {
    core.drafts_clear(MatrixDraftSlotRequest { room_id, slot })
        .await
        .map_err(map_drafts_core_error)
}

pub(crate) async fn drafts_resolve(
    core: &Core,
    room_id: String,
    slot: String,
    local: NativeLocalDraftState,
    fetch_fresh: bool,
) -> Result<NativeDraftResolution, MatrixAuthCommandError> {
    core.drafts_resolve(MatrixDraftResolveRequest {
        room_id,
        slot,
        local,
        fetch_fresh,
    })
    .await
    .map_err(map_drafts_core_error)
}

pub(crate) async fn drafts_delete_all(
    core: &Core,
) -> Result<NativeDraftDeleteAllResult, MatrixAuthCommandError> {
    core.drafts_delete_all()
        .await
        .map_err(map_drafts_core_error)
}

pub(crate) async fn drafts_flush(core: &Core) -> Result<(), MatrixAuthCommandError> {
    core.drafts_flush().await.map_err(map_drafts_core_error)
}

fn map_drafts_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error.diagnostic_id.as_deref().unwrap_or("drafts-failed");
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "Draft sync is not available on this device.",
            diagnostic,
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The draft sync request is invalid.",
            diagnostic,
        ),
        _ => MatrixAuthCommandError::new("Unknown", "Draft sync is unavailable.", diagnostic),
    }
}
