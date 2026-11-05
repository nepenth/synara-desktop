//! Apple-shell access to cross-device drafts. iOS writes and reads markdown;
//! desktop's rich Slate value never crosses this boundary.
use super::SharedCore;
use crate::app::drafts::{
    NativeDraftDecision, NativeDraftKeyState, NativeDraftResolution, NativeDraftSyncStatus,
    NativeLocalDraftState, NativeRemoteDraft,
};
use crate::core_api::{
    MatrixDraftResolveRequest, MatrixDraftSaveRequest, MatrixDraftSetEnabledRequest,
    MatrixDraftSlotRequest,
};
use crate::transport::{MatrixIpcError, MatrixIpcErrorCategory};

const DRAFTS_NO_SESSION_CODE: &str = "drafts-no-session";
const DRAFTS_NO_SESSION_DESCRIPTION: &str = "No native Matrix session is active.";
const DRAFTS_FAILED_CODE: &str = "drafts-failed";
const DRAFTS_FAILED_DESCRIPTION: &str = "Draft sync is unavailable.";

super::wire_enum::wire_enum! {
    pub enum DraftKeyStateDto {
        Ready => "ready",
        Requesting => "requesting",
        NeedsRecovery => "needs_recovery",
        Disabled => "disabled",
    }
}
super::wire_enum::wire_enum_from!(NativeDraftKeyState => DraftKeyStateDto {
    Ready, Requesting, NeedsRecovery, Disabled
});

super::wire_enum::wire_enum! {
    pub enum DraftDecisionDto {
        Keep => "keep",
        Apply => "apply",
        Prompt => "prompt",
    }
}
super::wire_enum::wire_enum_from!(NativeDraftDecision => DraftDecisionDto {
    Keep, Apply, Prompt
});

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DraftSyncStatusDto {
    pub session_generation: u64,
    pub enabled: bool,
    pub key_state: DraftKeyStateDto,
}

impl From<NativeDraftSyncStatus> for DraftSyncStatusDto {
    fn from(status: NativeDraftSyncStatus) -> Self {
        Self {
            session_generation: status.session_generation,
            enabled: status.enabled,
            key_state: status.key_state.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RemoteDraftDto {
    pub room_id: String,
    pub slot: String,
    pub updated_at: u64,
    pub device_id: String,
    pub cleared: bool,
    pub markdown: Option<String>,
    pub reply_to: Option<String>,
    pub edit_of: Option<String>,
    pub truncated: bool,
}

impl From<NativeRemoteDraft> for RemoteDraftDto {
    fn from(draft: NativeRemoteDraft) -> Self {
        Self {
            room_id: draft.room_id,
            slot: draft.slot,
            updated_at: draft.updated_at,
            device_id: draft.device_id,
            cleared: draft.cleared,
            markdown: draft.markdown,
            reply_to: draft.reply_to,
            edit_of: draft.edit_of,
            truncated: draft.truncated,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DraftResolutionDto {
    pub session_generation: u64,
    pub decision: DraftDecisionDto,
    pub draft: Option<RemoteDraftDto>,
}

impl From<NativeDraftResolution> for DraftResolutionDto {
    fn from(resolution: NativeDraftResolution) -> Self {
        Self {
            session_generation: resolution.session_generation,
            decision: resolution.decision.into(),
            draft: resolution.draft.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct DraftWriteResultDto {
    pub session_generation: u64,
    /// Record this as the local draft's `updatedAt`.
    pub updated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum DraftsCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for DraftsCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}
impl std::error::Error for DraftsCommandError {}

fn drafts_failed(code: &str, description: &'static str) -> DraftsCommandError {
    DraftsCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_drafts_core_error(error: MatrixIpcError) -> DraftsCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code.ends_with("-no-session") => {
            drafts_failed(DRAFTS_NO_SESSION_CODE, DRAFTS_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("drafts-") => drafts_failed(code, DRAFTS_FAILED_DESCRIPTION),
        _ if error.category == MatrixIpcErrorCategory::Forbidden => {
            drafts_failed(DRAFTS_NO_SESSION_CODE, DRAFTS_NO_SESSION_DESCRIPTION)
        }
        _ => drafts_failed(DRAFTS_FAILED_CODE, DRAFTS_FAILED_DESCRIPTION),
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SharedCore {
    /// Whether draft sync runs on this device, and why not. Asks the user's
    /// other verified devices for the key when it is missing.
    pub async fn drafts_status(&self) -> Result<DraftSyncStatusDto, DraftsCommandError> {
        self.core
            .drafts_status()
            .await
            .map(Into::into)
            .map_err(map_drafts_core_error)
    }

    pub async fn drafts_set_enabled(
        &self,
        enabled: bool,
    ) -> Result<DraftSyncStatusDto, DraftsCommandError> {
        self.core
            .drafts_set_enabled(MatrixDraftSetEnabledRequest { enabled })
            .await
            .map(Into::into)
            .map_err(map_drafts_core_error)
    }

    /// Queue (or with `flush`, write now) one composer's draft. Empty text
    /// with no reply or edit target syncs as a cleared draft.
    pub async fn drafts_save(
        &self,
        room_id: String,
        slot: String,
        markdown: Option<String>,
        reply_to: Option<String>,
        edit_of: Option<String>,
        flush: bool,
    ) -> Result<DraftWriteResultDto, DraftsCommandError> {
        let result = self
            .core
            .drafts_save(MatrixDraftSaveRequest {
                room_id,
                slot,
                markdown,
                desktop_slate: None,
                reply_to,
                edit_of,
                flush,
            })
            .await
            .map_err(map_drafts_core_error)?;
        Ok(DraftWriteResultDto {
            session_generation: result.session_generation,
            updated_at: result.updated_at,
        })
    }

    /// The draft was sent: clear it on every device.
    pub async fn drafts_clear(
        &self,
        room_id: String,
        slot: String,
    ) -> Result<DraftWriteResultDto, DraftsCommandError> {
        let result = self
            .core
            .drafts_clear(MatrixDraftSlotRequest { room_id, slot })
            .await
            .map_err(map_drafts_core_error)?;
        Ok(DraftWriteResultDto {
            session_generation: result.session_generation,
            updated_at: result.updated_at,
        })
    }

    /// Compare the synced draft with the composer: keep, apply or prompt.
    #[allow(clippy::too_many_arguments)]
    pub async fn drafts_resolve(
        &self,
        room_id: String,
        slot: String,
        local_updated_at: Option<u64>,
        local_dirty: bool,
        local_markdown: Option<String>,
        fetch_fresh: bool,
    ) -> Result<DraftResolutionDto, DraftsCommandError> {
        self.core
            .drafts_resolve(MatrixDraftResolveRequest {
                room_id,
                slot,
                local: NativeLocalDraftState {
                    updated_at: local_updated_at,
                    dirty: local_dirty,
                    markdown: local_markdown,
                },
                fetch_fresh,
            })
            .await
            .map(Into::into)
            .map_err(map_drafts_core_error)
    }

    /// Clear every synced draft of this account.
    pub async fn drafts_delete_all(&self) -> Result<u32, DraftsCommandError> {
        self.core
            .drafts_delete_all()
            .await
            .map(|result| result.cleared)
            .map_err(map_drafts_core_error)
    }

    /// Write every queued draft now (the app is going to the background).
    pub async fn drafts_flush(&self) -> Result<(), DraftsCommandError> {
        self.core
            .drafts_flush()
            .await
            .map_err(map_drafts_core_error)
    }
}
