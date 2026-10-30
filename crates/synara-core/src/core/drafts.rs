//! Typed Core commands for cross-device drafts (`in.synara.draft`).
//!
//! The drafts owner lives inside the account-data owner, so every command
//! shares its session check.

use super::*;
use crate::app::drafts::{
    DraftSyncOwner, NativeDraftResolution, NativeDraftSyncStatus, NativeLocalDraftState,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixDraftSaveRequest {
    pub room_id: String,
    pub slot: String,
    #[serde(default)]
    pub markdown: Option<String>,
    #[serde(default)]
    pub desktop_slate: Option<serde_json::Value>,
    #[serde(default)]
    pub reply_to: Option<String>,
    #[serde(default)]
    pub edit_of: Option<String>,
    #[serde(default)]
    pub flush: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixDraftSlotRequest {
    pub room_id: String,
    pub slot: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixDraftResolveRequest {
    pub room_id: String,
    pub slot: String,
    #[serde(default)]
    pub local: NativeLocalDraftState,
    #[serde(default)]
    pub fetch_fresh: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixDraftSetEnabledRequest {
    pub enabled: bool,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeDraftWriteResult {
    pub session_generation: u64,
    /// The stamp this write carries; the shell records it as its local
    /// draft's `updatedAt` so its own echo is never newer.
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub updated_at: u64,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeDraftDeleteAllResult {
    pub session_generation: u64,
    pub cleared: u32,
}

fn drafts_owner(
    state: &Arc<CoreState>,
    no_session: &'static str,
) -> Result<Arc<NativeImagePackOwner>, MatrixIpcError> {
    state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden).with_diagnostic(no_session)
    })
}

pub(super) fn drafts_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "drafts-invalid-room"
        | "drafts-invalid-slot"
        | "drafts-invalid-event-id"
        | "drafts-markdown-too-large" => MatrixIpcErrorCategory::SdkInvariant,
        "drafts-sync-disabled" | "drafts-key-missing" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn owner_drafts(owner: &NativeImagePackOwner) -> &DraftSyncOwner {
    owner.drafts()
}

impl Core {
    /// Typed `matrix_drafts_status`. Asks peers for the key when it is missing.
    pub async fn drafts_status(&self) -> Result<NativeDraftSyncStatus, MatrixIpcError> {
        let owner = drafts_owner(&self.state, "drafts-status-no-session")?;
        Ok(owner_drafts(&owner).status().await)
    }

    /// Typed `matrix_drafts_set_enabled`.
    pub async fn drafts_set_enabled(
        &self,
        request: MatrixDraftSetEnabledRequest,
    ) -> Result<NativeDraftSyncStatus, MatrixIpcError> {
        let owner = drafts_owner(&self.state, "drafts-set-enabled-no-session")?;
        let drafts = owner_drafts(&owner);
        drafts.set_enabled(request.enabled);
        Ok(drafts.status().await)
    }

    /// Typed `matrix_drafts_save`.
    pub async fn drafts_save(
        &self,
        request: MatrixDraftSaveRequest,
    ) -> Result<NativeDraftWriteResult, MatrixIpcError> {
        let owner = drafts_owner(&self.state, "drafts-save-no-session")?;
        let drafts = owner_drafts(&owner);
        let updated_at = drafts
            .save(
                &request.room_id,
                &request.slot,
                request.markdown,
                request.desktop_slate,
                request.reply_to,
                request.edit_of,
                request.flush,
            )
            .await
            .map_err(drafts_error)?;
        Ok(NativeDraftWriteResult {
            session_generation: drafts.session_generation(),
            updated_at,
        })
    }

    /// Typed `matrix_drafts_clear` (after a send).
    pub async fn drafts_clear(
        &self,
        request: MatrixDraftSlotRequest,
    ) -> Result<NativeDraftWriteResult, MatrixIpcError> {
        let owner = drafts_owner(&self.state, "drafts-clear-no-session")?;
        let drafts = owner_drafts(&owner);
        let updated_at = drafts
            .clear(&request.room_id, &request.slot)
            .await
            .map_err(drafts_error)?;
        Ok(NativeDraftWriteResult {
            session_generation: drafts.session_generation(),
            updated_at,
        })
    }

    /// Typed `matrix_drafts_resolve`.
    pub async fn drafts_resolve(
        &self,
        request: MatrixDraftResolveRequest,
    ) -> Result<NativeDraftResolution, MatrixIpcError> {
        let owner = drafts_owner(&self.state, "drafts-resolve-no-session")?;
        owner_drafts(&owner)
            .resolve(
                &request.room_id,
                &request.slot,
                request.local,
                request.fetch_fresh,
            )
            .await
            .map_err(drafts_error)
    }

    /// Typed `matrix_drafts_delete_all`.
    pub async fn drafts_delete_all(&self) -> Result<NativeDraftDeleteAllResult, MatrixIpcError> {
        let owner = drafts_owner(&self.state, "drafts-delete-all-no-session")?;
        let drafts = owner_drafts(&owner);
        let cleared = drafts.delete_all().await.map_err(drafts_error)?;
        Ok(NativeDraftDeleteAllResult {
            session_generation: drafts.session_generation(),
            cleared,
        })
    }

    /// Typed `matrix_drafts_flush`: write every queued draft now.
    pub async fn drafts_flush(&self) -> Result<(), MatrixIpcError> {
        let owner = drafts_owner(&self.state, "drafts-flush-no-session")?;
        owner_drafts(&owner).flush_all().await;
        Ok(())
    }
}
