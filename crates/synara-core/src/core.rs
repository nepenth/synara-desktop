//! Shared native-core entry points (P2 foundation).
//!
//! `Core` owns safe session projection/lifecycle plus the transport command
//! registry. It intentionally has no Tauri dependency; P2 command groups add
//! handlers, P3 makes the desktop shell a thin `Core::command` registrar.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::app::account_data::{
    NativeGlobalImagePacksSnapshot, NativeImagePackOwner, NativeLaterSnapshot,
    NativeMDirectMutationResult, NativeMDirectSnapshot, NativeRoomImagePacksSnapshot,
    NativeRoomNotesSnapshot, NativeUserImagePackSnapshot, RoomNoteMoveDirection, SynaraLaterItem,
    SynaraRoomNoteItem,
};
use crate::app::auth::{
    discover_login_flows, login_flows_response, probe_register_flows, AuthError,
    HttpLoginFlowTransport, HttpRegisterFlowTransport, MatrixLoginFlowsResponse,
    RegisterFlowsProbe,
};
use crate::app::backup::NativeBackupStatus;
use crate::app::cross_signing::NativeCrossSigningSetupResult;
use crate::app::devices::{NativeDeviceDeleteResult, NativeDeviceOwner, NativeDeviceSnapshot};
use crate::app::members::{
    NativePowerLevelWriteResult, NativeRoomCreatorsSnapshot, NativeRoomMembersSnapshot,
    NativeRoomPowerLevelTagsSnapshot, NativeRoomPowerLevelsSnapshot, ROOM_POWER_LEVELS_EVENT_TYPE,
    ROOM_POWER_LEVEL_TAGS_EVENT_TYPE,
};
use crate::app::presence::{
    NativePresenceOwner, NativePresenceSnapshotResult, NativePresenceSubscription,
};
use crate::app::room_directory::{
    DirectoryRoomTypeFilter, DirectorySearchInput, NativeRoomDirectoryProtocols,
    NativeRoomDirectorySearchResponse,
};
use crate::app::room_keys::NativeRoomKeyTransferStatus;
use crate::app::room_list::{
    snapshot_from_sync_owner, NativeInviteSnapshot, NativeRoomListSnapshot,
};
use crate::app::room_ops::MatrixRoomCreateRequest;
use crate::app::room_profile::{
    MatrixRoomDirectoryVisibilityResult, MatrixRoomDirectoryVisibilityWriteResult,
    MatrixRoomJoinRuleSnapshot, NativeRoomJoinRuleOwner,
};
use crate::app::send::{
    MatrixPollRespondResult, MatrixSendPollResult, MatrixSendStickerResult, MatrixSendTextResult,
};
use crate::app::spaces::{
    NativeRestrictedJoinReparentResult, NativeSpaceChildMutationResult,
    NativeSpaceChildrenSnapshot, NativeSpaceHierarchySnapshot, NativeSpaceParentsSnapshot,
};
use crate::app::sync::{
    SyncReadinessSnapshot, SyncServiceOwner, SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID,
};
use crate::app::timeline::{
    NativeComposerReplyDraftReadback, NativeReactionMutationResult, NativeTimelineActionReadback,
    NativeTimelineCloseRequest, NativeTimelineDirection, NativeTimelineEventReadback,
    NativeTimelineJumpLatestRequest, NativeTimelineOpenPosition, NativeTimelineOpenReadback,
    NativeTimelineOpenRequest, NativeTimelineOwner, NativeTimelineReadAction,
    NativeTimelineReadStateReadback, NativeTimelineReadStateRequest,
    NativeTimelineViewPaginationRequest, TimelineViewSnapshot,
};
use crate::app::typing::{NativeTypingOwner, NativeTypingSnapshot};
use crate::app::user_profile::MatrixProfileWriteResult;
use crate::app::verification::{
    NativeVerificationInbox, NativeVerificationOwner, NativeVerificationRequest,
};
use crate::dto::SessionSnapshot;
use crate::platform::{
    Platform, PlatformCrossSigningOwnIdentity, PlatformCrossSigningPrivateState,
    PlatformCrossSigningStatus, PlatformCrossSigningStatusError, PlatformCryptoCrossSigningState,
    PlatformCryptoStatus, PlatformMediaConfig, PlatformMediaConfigError,
    PlatformSecretStorageAction, PlatformSecretStorageState, PlatformSecretStorageStatus,
    PlatformSecretStorageStatusError, PlatformSyncFailure, PlatformSyncStatus,
};
use crate::transport::{
    CommandEnvelope, CommandFuture, CommandRegistry, CommandResponseEnvelope, MatrixIpcError,
    MatrixIpcErrorCategory, MAX_WIRE_COUNTER,
};

/// React-compatible payload for `matrix_session_snapshot`.
///
/// This deliberately selects only the fields returned by the desktop command,
/// rather than serializing the broader safe session projection wholesale.
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum MatrixSessionSnapshotResponse {
    LoggedOut,
    LoggedIn {
        user_id: String,
        device_id: String,
        homeserver_url: String,
        #[serde(rename = "sessionGeneration")]
        session_generation: u64,
    },
}

impl From<Option<SessionSnapshot>> for MatrixSessionSnapshotResponse {
    fn from(snapshot: Option<SessionSnapshot>) -> Self {
        match snapshot {
            None => Self::LoggedOut,
            Some(snapshot) => Self::LoggedIn {
                user_id: snapshot.user_id,
                device_id: snapshot.device_id,
                homeserver_url: snapshot.homeserver_url,
                session_generation: snapshot.session_generation,
            },
        }
    }
}

/// Fixed public cross-signing state for `matrix_crypto_status`.
///
/// Core alone serializes this public vocabulary after a Platform has reduced
/// its shell-owned SDK observation to a closed enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum MatrixCryptoCrossSigningStateResponse {
    Unavailable,
    NotSetUp,
    Partial,
    Ready,
}

/// Exact React/Tauri payload for `matrix_crypto_status`.
///
/// Keep this separate from the Platform projection: this type owns the wire
/// field names and is constructed only after Core validates the closed input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct MatrixCryptoStatusResponse {
    session_generation: u64,
    encryption_enabled: bool,
    cross_signing_state: MatrixCryptoCrossSigningStateResponse,
}

impl MatrixCryptoStatusResponse {
    fn from_platform(status: PlatformCryptoStatus) -> Result<Self, MatrixIpcError> {
        let cross_signing_state = match status.cross_signing_state() {
            PlatformCryptoCrossSigningState::Unavailable => {
                MatrixCryptoCrossSigningStateResponse::Unavailable
            }
            PlatformCryptoCrossSigningState::NotSetUp => {
                MatrixCryptoCrossSigningStateResponse::NotSetUp
            }
            PlatformCryptoCrossSigningState::Partial => {
                MatrixCryptoCrossSigningStateResponse::Partial
            }
            PlatformCryptoCrossSigningState::Ready => MatrixCryptoCrossSigningStateResponse::Ready,
        };
        let response = Self {
            session_generation: status.session_generation(),
            encryption_enabled: status.encryption_enabled(),
            cross_signing_state,
        };
        response
            .is_valid()
            .then_some(response)
            .ok_or_else(|| core_state_error("p2-crypto-status-invalid-platform-projection"))
    }

    fn is_valid(&self) -> bool {
        matches!(
            (self.encryption_enabled, self.cross_signing_state),
            (false, MatrixCryptoCrossSigningStateResponse::Unavailable)
                | (true, MatrixCryptoCrossSigningStateResponse::NotSetUp)
                | (true, MatrixCryptoCrossSigningStateResponse::Partial)
                | (true, MatrixCryptoCrossSigningStateResponse::Ready)
        )
    }
}

/// Fixed public readiness vocabulary for `matrix_cross_signing_status`.
///
/// `recovery_required` remains a read-only legacy status label. This transport
/// command performs no setup, recovery, or verification action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum MatrixCrossSigningReadinessResponse {
    Unavailable,
    SetupRequired,
    RecoveryRequired,
    VerificationRequired,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum MatrixCrossSigningKeyPublicationResponse {
    Missing,
    Published,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum MatrixCrossSigningPrivateIdentityResponse {
    Missing,
    Partial,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum MatrixOwnIdentityVerificationResponse {
    Missing,
    Unverified,
    Verified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum MatrixCrossSigningBootstrapResponse {
    Needed,
    NotNeeded,
}

/// Exact legacy camel-case React/Tauri DTO for `matrix_cross_signing_status`.
///
/// This is deliberately separate from the Platform projection. Core alone
/// reconstructs all public labels after it has received only a bounded
/// generation and two closed private enums.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct MatrixCrossSigningStatusResponse {
    session_generation: u64,
    readiness: MatrixCrossSigningReadinessResponse,
    master_signing: MatrixCrossSigningKeyPublicationResponse,
    self_signing: MatrixCrossSigningKeyPublicationResponse,
    user_signing: MatrixCrossSigningKeyPublicationResponse,
    private_identity: MatrixCrossSigningPrivateIdentityResponse,
    own_identity_verification: MatrixOwnIdentityVerificationResponse,
    bootstrap: MatrixCrossSigningBootstrapResponse,
}

impl MatrixCrossSigningStatusResponse {
    fn from_platform(status: PlatformCrossSigningStatus) -> Result<Self, MatrixIpcError> {
        let private_identity = match status.private_state() {
            PlatformCrossSigningPrivateState::Unavailable
            | PlatformCrossSigningPrivateState::Missing => {
                MatrixCrossSigningPrivateIdentityResponse::Missing
            }
            PlatformCrossSigningPrivateState::Partial => {
                MatrixCrossSigningPrivateIdentityResponse::Partial
            }
            PlatformCrossSigningPrivateState::Complete => {
                MatrixCrossSigningPrivateIdentityResponse::Complete
            }
        };
        let (publication, own_identity_verification) = match status.own_identity() {
            PlatformCrossSigningOwnIdentity::Missing => (
                MatrixCrossSigningKeyPublicationResponse::Missing,
                MatrixOwnIdentityVerificationResponse::Missing,
            ),
            PlatformCrossSigningOwnIdentity::Unverified => (
                MatrixCrossSigningKeyPublicationResponse::Published,
                MatrixOwnIdentityVerificationResponse::Unverified,
            ),
            PlatformCrossSigningOwnIdentity::Verified => (
                MatrixCrossSigningKeyPublicationResponse::Published,
                MatrixOwnIdentityVerificationResponse::Verified,
            ),
        };
        let readiness = match (status.private_state(), status.own_identity()) {
            (PlatformCrossSigningPrivateState::Unavailable, _) => {
                MatrixCrossSigningReadinessResponse::Unavailable
            }
            (_, PlatformCrossSigningOwnIdentity::Missing) => {
                MatrixCrossSigningReadinessResponse::SetupRequired
            }
            (
                PlatformCrossSigningPrivateState::Missing
                | PlatformCrossSigningPrivateState::Partial,
                PlatformCrossSigningOwnIdentity::Unverified
                | PlatformCrossSigningOwnIdentity::Verified,
            ) => MatrixCrossSigningReadinessResponse::RecoveryRequired,
            (
                PlatformCrossSigningPrivateState::Complete,
                PlatformCrossSigningOwnIdentity::Unverified,
            ) => MatrixCrossSigningReadinessResponse::VerificationRequired,
            (
                PlatformCrossSigningPrivateState::Complete,
                PlatformCrossSigningOwnIdentity::Verified,
            ) => MatrixCrossSigningReadinessResponse::Ready,
        };
        let bootstrap = match (status.private_state(), status.own_identity()) {
            (PlatformCrossSigningPrivateState::Unavailable, _)
            | (
                _,
                PlatformCrossSigningOwnIdentity::Unverified
                | PlatformCrossSigningOwnIdentity::Verified,
            ) => MatrixCrossSigningBootstrapResponse::NotNeeded,
            (_, PlatformCrossSigningOwnIdentity::Missing) => {
                MatrixCrossSigningBootstrapResponse::Needed
            }
        };
        let response = Self {
            session_generation: status.session_generation(),
            readiness,
            master_signing: publication,
            self_signing: publication,
            user_signing: publication,
            private_identity,
            own_identity_verification,
            bootstrap,
        };
        response
            .is_valid()
            .then_some(response)
            .ok_or_else(|| core_state_error("p2-cross-signing-status-invalid-platform-projection"))
    }

    /// Revalidate the complete legacy truth table before serializing it.
    fn is_valid(&self) -> bool {
        if self.session_generation > MAX_WIRE_COUNTER
            || self.master_signing != self.self_signing
            || self.master_signing != self.user_signing
        {
            return false;
        }

        let identity_is_consistent = matches!(
            (self.master_signing, self.own_identity_verification),
            (
                MatrixCrossSigningKeyPublicationResponse::Missing,
                MatrixOwnIdentityVerificationResponse::Missing
            ) | (
                MatrixCrossSigningKeyPublicationResponse::Published,
                MatrixOwnIdentityVerificationResponse::Unverified
                    | MatrixOwnIdentityVerificationResponse::Verified
            )
        );
        if !identity_is_consistent {
            return false;
        }

        matches!(
            (
                self.readiness,
                self.private_identity,
                self.own_identity_verification,
                self.bootstrap,
            ),
            (
                MatrixCrossSigningReadinessResponse::Unavailable,
                MatrixCrossSigningPrivateIdentityResponse::Missing,
                _,
                MatrixCrossSigningBootstrapResponse::NotNeeded,
            ) | (
                MatrixCrossSigningReadinessResponse::SetupRequired,
                MatrixCrossSigningPrivateIdentityResponse::Missing
                    | MatrixCrossSigningPrivateIdentityResponse::Partial
                    | MatrixCrossSigningPrivateIdentityResponse::Complete,
                MatrixOwnIdentityVerificationResponse::Missing,
                MatrixCrossSigningBootstrapResponse::Needed,
            ) | (
                MatrixCrossSigningReadinessResponse::RecoveryRequired,
                MatrixCrossSigningPrivateIdentityResponse::Missing
                    | MatrixCrossSigningPrivateIdentityResponse::Partial,
                MatrixOwnIdentityVerificationResponse::Unverified
                    | MatrixOwnIdentityVerificationResponse::Verified,
                MatrixCrossSigningBootstrapResponse::NotNeeded,
            ) | (
                MatrixCrossSigningReadinessResponse::VerificationRequired,
                MatrixCrossSigningPrivateIdentityResponse::Complete,
                MatrixOwnIdentityVerificationResponse::Unverified,
                MatrixCrossSigningBootstrapResponse::NotNeeded,
            ) | (
                MatrixCrossSigningReadinessResponse::Ready,
                MatrixCrossSigningPrivateIdentityResponse::Complete,
                MatrixOwnIdentityVerificationResponse::Verified,
                MatrixCrossSigningBootstrapResponse::NotNeeded,
            )
        )
    }
}

/// Fixed public state vocabulary for `matrix_secret_storage_status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum MatrixSecretStorageStateResponse {
    Unavailable,
    NotSetUp,
    Locked,
    Ready,
}

/// Fixed public action vocabulary for `matrix_secret_storage_status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum MatrixSecretStorageActionResponse {
    BootstrapRequired,
    UnlockRequired,
    None,
}

/// Fixed public missing-secret label vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum MatrixMissingSecretResponse {
    CrossSigningMaster,
    CrossSigningSelfSigning,
    CrossSigningUserSigning,
    EncryptionBackup,
}

/// Exact React/Tauri DTO for `matrix_secret_storage_status`.
///
/// Core reconstructs this legacy object only from the platform's closed,
/// scalar projection. The `missingSecrets` list is public wire shape; its
/// canonical labels and ordering are owned here, not supplied by the shell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct MatrixSecretStorageStatusResponse {
    session_generation: u64,
    state: MatrixSecretStorageStateResponse,
    exists: bool,
    unlocked: bool,
    default_key_set: bool,
    passphrase_configured: bool,
    bootstrap_ready: bool,
    missing_secrets: Vec<MatrixMissingSecretResponse>,
    action: MatrixSecretStorageActionResponse,
}

impl MatrixSecretStorageStatusResponse {
    fn from_platform(status: PlatformSecretStorageStatus) -> Result<Self, MatrixIpcError> {
        let state = match status.state() {
            PlatformSecretStorageState::Unavailable => {
                MatrixSecretStorageStateResponse::Unavailable
            }
            PlatformSecretStorageState::NotSetUp => MatrixSecretStorageStateResponse::NotSetUp,
            PlatformSecretStorageState::Locked => MatrixSecretStorageStateResponse::Locked,
            PlatformSecretStorageState::Ready => MatrixSecretStorageStateResponse::Ready,
        };
        let action = match status.action() {
            PlatformSecretStorageAction::BootstrapRequired => {
                MatrixSecretStorageActionResponse::BootstrapRequired
            }
            PlatformSecretStorageAction::UnlockRequired => {
                MatrixSecretStorageActionResponse::UnlockRequired
            }
            PlatformSecretStorageAction::None => MatrixSecretStorageActionResponse::None,
        };
        let missing = status.missing_secrets();
        let mut missing_secrets = Vec::with_capacity(4);
        if missing.cross_signing_master() {
            missing_secrets.push(MatrixMissingSecretResponse::CrossSigningMaster);
        }
        if missing.cross_signing_self_signing() {
            missing_secrets.push(MatrixMissingSecretResponse::CrossSigningSelfSigning);
        }
        if missing.cross_signing_user_signing() {
            missing_secrets.push(MatrixMissingSecretResponse::CrossSigningUserSigning);
        }
        if missing.encryption_backup() {
            missing_secrets.push(MatrixMissingSecretResponse::EncryptionBackup);
        }
        let response = Self {
            session_generation: status.session_generation(),
            state,
            exists: status.exists(),
            unlocked: status.unlocked(),
            default_key_set: status.default_key_set(),
            passphrase_configured: status.passphrase_configured(),
            bootstrap_ready: status.bootstrap_ready(),
            missing_secrets,
            action,
        };
        response
            .is_valid()
            .then_some(response)
            .ok_or_else(|| core_state_error("p2-secret-storage-status-invalid-platform-projection"))
    }

    fn is_valid(&self) -> bool {
        if self.session_generation > MAX_WIRE_COUNTER {
            return false;
        }
        matches!(
            (self.state, self.unlocked, self.action),
            (
                MatrixSecretStorageStateResponse::Unavailable,
                false,
                MatrixSecretStorageActionResponse::UnlockRequired,
            ) | (
                MatrixSecretStorageStateResponse::NotSetUp,
                false,
                MatrixSecretStorageActionResponse::BootstrapRequired,
            ) | (
                MatrixSecretStorageStateResponse::Locked,
                false,
                MatrixSecretStorageActionResponse::UnlockRequired,
            ) | (
                MatrixSecretStorageStateResponse::Ready,
                true,
                MatrixSecretStorageActionResponse::None,
            )
        )
    }
}

/// Exact React/Tauri payload for `matrix_media_config`.
///
/// The legacy command deliberately has no renderer-supplied input and returns
/// this one-key object verbatim. Keep it independent from the Platform
/// projection: Core owns the public field spelling and serializes only after
/// checking the shared JavaScript-safe counter bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
struct MatrixMediaConfigResponse {
    #[serde(rename = "m.upload.size")]
    upload_size: u64,
}

impl MatrixMediaConfigResponse {
    fn from_platform(config: PlatformMediaConfig) -> Result<Self, MatrixIpcError> {
        let response = Self {
            upload_size: config.upload_size(),
        };
        (response.upload_size <= MAX_WIRE_COUNTER)
            .then_some(response)
            .ok_or_else(|| core_state_error("p2-media-config-invalid-platform-projection"))
    }
}

/// Exact React/Tauri envelope payload for `matrix_login_flows`.
///
/// The renderer sends the camel-case `homeserverUrl` key; unknown keys are
/// rejected so accidental credential fields do not cross this boundary.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixLoginFlowsRequest {
    homeserver_url: String,
}

/// Exact React/Tauri envelope payload for `matrix_presence_snapshot`.
///
/// The renderer sends the camel-case `userId` key; unknown keys are rejected
/// so this read-only route cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixPresenceSnapshotRequest {
    user_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_presence_subscribe`.
///
/// Shares the snapshot's camel-case `userId` key. Unknown keys are rejected
/// so this subscribe route cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixPresenceSubscribeRequest {
    user_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_presence_unsubscribe`.
///
/// The renderer sends the camel-case `subscriptionId` key; unknown keys are
/// rejected so this release route cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixPresenceUnsubscribeRequest {
    subscription_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_get_room_image_packs`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixGetRoomImagePacksRequest {
    room_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixSetImagePackContentRequest {
    content: serde_json::Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixSetRoomImagePackRequest {
    room_id: String,
    state_key: String,
    content: serde_json::Value,
}

/// Exact React/Tauri envelope payload for `matrix_mdirect_add`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixMDirectAddRequest {
    room_id: String,
    user_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_mdirect_remove`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixMDirectRemoveRequest {
    room_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixLaterUpsertRequest {
    item: SynaraLaterItem,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixLaterCompleteRequest {
    item_id: String,
    #[serde(default)]
    completed_at: Option<f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixLaterSnoozeRequest {
    item_id: String,
    due_ts: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixLaterMarkRemindedRequest {
    item_id: String,
    #[serde(default)]
    reminded_at: Option<f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixRoomNotesUpsertRequest {
    item: SynaraRoomNoteItem,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixRoomNotesItemRequest {
    room_id: String,
    item_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixRoomNotesCompleteTodoRequest {
    room_id: String,
    item_id: String,
    completed: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixRoomNotesMoveTodoRequest {
    room_id: String,
    item_id: String,
    direction: RoomNoteMoveDirection,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixTypingSetRequest {
    room_id: String,
    typing: bool,
}

/// Exact React/Tauri envelope payload for `matrix_device_rename`.
///
/// The renderer sends camel-case `deviceId` and `displayName`; unknown keys
/// are rejected so this write cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixDeviceRenameRequest {
    device_id: String,
    display_name: String,
}

/// Exact React/Tauri envelope payload for `matrix_device_delete_start`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixDeviceDeleteStartRequest {
    device_ids: Vec<String>,
}

/// Exact React/Tauri envelope payload for `matrix_device_delete_cancel`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixDeviceDeleteCancelRequest {
    operation_id: u64,
    session_generation: u64,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_close`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixTimelineCloseRequest {
    stream_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_open`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixTimelineOpenRequest {
    room_id: String,
    position: NativeTimelineOpenPosition,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_jump_latest`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixTimelineJumpLatestRequest {
    stream_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_event_readback`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixTimelineEventReadbackRequest {
    room_id: String,
    event_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_paginate`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixTimelinePaginateRequest {
    stream_id: String,
    direction: NativeTimelineDirection,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_set_read_state`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixTimelineSetReadStateRequest {
    stream_id: String,
    action: NativeTimelineReadAction,
}

/// Exact React/Tauri envelope payload for reaction toggle/ensure.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixTimelineReactionKeyRequest {
    room_id: String,
    event_id: String,
    key: String,
}

/// Exact React/Tauri envelope payload for `matrix_reaction_redact`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixReactionRedactRequest {
    room_id: String,
    target_event_id: String,
    reaction_event_id: String,
    key: String,
}

/// Exact React/Tauri envelope payload for `matrix_send_text`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixSendTextRequest {
    room_id: String,
    body: String,
    #[serde(default)]
    msg_type: Option<String>,
    #[serde(default)]
    formatted_body: Option<String>,
    #[serde(default)]
    mention_user_ids: Option<Vec<String>>,
    #[serde(default)]
    mention_room: Option<bool>,
    #[serde(default)]
    reply_to: Option<String>,
    #[serde(default)]
    thread_root: Option<String>,
    #[serde(default)]
    txn_id: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_send_sticker`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixSendStickerRequest {
    room_id: String,
    body: String,
    mxc: String,
    #[serde(default)]
    width: Option<u64>,
    #[serde(default)]
    height: Option<u64>,
    #[serde(default)]
    mimetype: Option<String>,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    reply_to: Option<String>,
    #[serde(default)]
    thread_root: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_send_poll`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixSendPollRequest {
    room_id: String,
    question: String,
    answers: Vec<String>,
    max_selections: u32,
    #[serde(default)]
    thread_root: Option<String>,
    #[serde(default)]
    reply_to: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_poll_respond`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixPollRespondRequest {
    room_id: String,
    poll_event_id: String,
    answer_ids: Vec<String>,
}

/// Exact React/Tauri envelope payload for `matrix_edit_message`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixEditMessageRequest {
    room_id: String,
    event_id: String,
    body: String,
    #[serde(default)]
    msg_type: Option<String>,
    #[serde(default)]
    formatted_body: Option<String>,
    #[serde(default)]
    mention_user_ids: Option<Vec<String>>,
    #[serde(default)]
    mention_room: Option<bool>,
    #[serde(default)]
    txn_id: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_edit_text`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixTimelineEditTextRequest {
    room_id: String,
    event_id: String,
    body: String,
    #[serde(default)]
    formatted_body: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_redact`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixTimelineRedactRequest {
    room_id: String,
    event_id: String,
    #[serde(default)]
    reason: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_report`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixTimelineReportRequest {
    room_id: String,
    event_id: String,
    #[serde(default)]
    reason: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_pin` / `matrix_timeline_unpin`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixTimelinePinRequest {
    room_id: String,
    event_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_poll_vote`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixTimelinePollVoteRequest {
    room_id: String,
    event_id: String,
    #[serde(default)]
    answer_ids: Vec<String>,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_call_decline`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixTimelineCallDeclineRequest {
    room_id: String,
    event_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_forward_text`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixTimelineForwardTextRequest {
    source_room_id: String,
    event_id: String,
    target_room_id: String,
    #[serde(default)]
    as_quote: bool,
}

/// Exact React/Tauri envelope payload for `matrix_timeline_forward_media`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixTimelineForwardMediaRequest {
    source_room_id: String,
    event_id: String,
    target_room_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_composer_set_reply_draft`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixComposerSetReplyDraftRequest {
    room_id: String,
    event_id: String,
    #[serde(default)]
    start_thread: bool,
}

/// Exact React/Tauri envelope payload for composer get/clear reply-draft.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixComposerReplyDraftRoomRequest {
    room_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_verification_accept`.
///
/// The renderer sends the camel-case `flowId` key; unknown keys are rejected
/// so this write cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixVerificationAcceptRequest {
    flow_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_verification_begin_sas`.
///
/// Shares accept's camel-case `flowId` key. Unknown keys are rejected so
/// this write cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixVerificationBeginSasRequest {
    flow_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_verification_cancel`.
///
/// Shares accept's camel-case `flowId` key. Unknown keys are rejected so
/// this write cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixVerificationCancelRequest {
    flow_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_verification_confirm`.
///
/// Shares accept's camel-case `flowId` key. Unknown keys are rejected so
/// this write cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixVerificationConfirmRequest {
    flow_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_verification_dismiss`.
///
/// Shares accept's camel-case `flowId` key. Unknown keys are rejected so
/// this write cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixVerificationDismissRequest {
    flow_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_verification_mismatch`.
///
/// Shares accept's camel-case `flowId` key. Unknown keys are rejected so
/// this write cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixVerificationMismatchRequest {
    flow_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_verification_start`.
///
/// The renderer sends optional camel-case `deviceId`. Unknown keys are
/// rejected so this write cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixVerificationStartRequest {
    #[serde(default)]
    device_id: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_room_join_rule_snapshot`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixRoomJoinRuleSnapshotRequest {
    room_id: String,
    session_generation: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixInviteActionRequest {
    room_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixRoomLeaveRequest {
    room_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixRoomJoinRequest {
    room_id_or_alias: String,
    #[serde(default)]
    via_servers: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixRoomModerationRequest {
    room_id: String,
    user_id: String,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixRoomUnbanRequest {
    room_id: String,
    user_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixRoomSetPowerLevelRequest {
    room_id: String,
    user_id: String,
    power_level: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixRoomSetPowerLevelStateRequest {
    room_id: String,
    content: serde_json::Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixRoomMembersSnapshotRequest {
    room_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_space_hierarchy_snapshot`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixSpaceHierarchySnapshotRequest {
    room_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_space_child_set`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixSpaceChildSetRequest {
    parent_id: String,
    child_id: String,
    via: Vec<String>,
    #[serde(default)]
    order: Option<String>,
    #[serde(default)]
    suggested: Option<bool>,
}

/// Exact React/Tauri envelope payload for `matrix_space_child_remove`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixSpaceChildRemoveRequest {
    parent_id: String,
    child_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_restricted_join_reparent`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixRestrictedJoinReparentRequest {
    room_id: String,
    #[serde(default)]
    remove_parent_id: Option<String>,
    add_parent_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_set_room_name`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixSetRoomNameRequest {
    room_id: String,
    name: String,
}

/// Exact React/Tauri envelope payload for `matrix_set_room_topic`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixSetRoomTopicRequest {
    room_id: String,
    topic: String,
}

/// Exact React/Tauri envelope payload for `matrix_set_room_avatar`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixSetRoomAvatarRequest {
    room_id: String,
    mxc: String,
}

/// Exact React/Tauri envelope payload for `matrix_set_own_display_name`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixSetOwnDisplayNameRequest {
    display_name: String,
}

/// Exact React/Tauri envelope payload for `matrix_set_own_avatar`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixSetOwnAvatarRequest {
    mxc: String,
}

/// Exact React/Tauri envelope payload for `matrix_room_directory_search`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixRoomDirectorySearchRequest {
    session_generation: u64,
    request_id: u64,
    #[serde(default)]
    server_name: Option<String>,
    #[serde(default)]
    term: Option<String>,
    #[serde(default)]
    room_type: Option<DirectoryRoomTypeFilter>,
    #[serde(default)]
    third_party_instance_id: Option<String>,
    limit: u64,
    #[serde(default)]
    since: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_room_directory_cancel`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixRoomDirectoryCancelRequest {
    session_generation: u64,
    request_id: u64,
}

/// Exact React/Tauri envelope payload for `matrix_get_room_directory_visibility`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixGetRoomDirectoryVisibilityRequest {
    room_id: String,
    session_generation: u64,
}

/// Exact React/Tauri envelope payload for `matrix_set_room_directory_visibility`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixSetRoomDirectoryVisibilityRequest {
    room_id: String,
    session_generation: u64,
    visibility: String,
}

/// Exact React/Tauri envelope payload for `matrix_register_flows`.
///
/// This read-only probe accepts exactly the existing camel-case homeserver
/// field and rejects all credential or UIAA-continuation fields at the core
/// boundary.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixRegisterFlowsRequest {
    homeserver_url: String,
}

/// Internal state passed to command handlers. It never carries shell types.
/// Opaque state context supplied to registered core command handlers.
///
/// Shells never construct it; fields stay private so handlers use only stable
/// core accessors instead of reaching into platform/session ownership.
pub struct CoreState {
    platform: Arc<dyn Platform>,
    session: Mutex<Option<SessionSnapshot>>,
    typing: Mutex<Option<Arc<NativeTypingOwner>>>,
    presence: Mutex<Option<Arc<NativePresenceOwner>>>,
    verification: Mutex<Option<Arc<NativeVerificationOwner>>>,
    devices: Mutex<Option<Arc<NativeDeviceOwner>>>,
    join_rules: Mutex<Option<Arc<NativeRoomJoinRuleOwner>>>,
    image_packs: Mutex<Option<Arc<NativeImagePackOwner>>>,
    timelines: Mutex<Option<Arc<NativeTimelineOwner>>>,
    sync: Mutex<Option<Arc<SyncServiceOwner>>>,
}

impl CoreState {
    pub fn platform(&self) -> Arc<dyn Platform> {
        Arc::clone(&self.platform)
    }

    pub fn session_snapshot(&self) -> Result<Option<SessionSnapshot>, MatrixIpcError> {
        self.session
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| core_state_error("p2-core-state-poisoned"))
    }

    fn typing_owner(&self) -> Result<Option<Arc<NativeTypingOwner>>, MatrixIpcError> {
        self.typing
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| core_state_error("p2-core-state-poisoned"))
    }

    fn presence_owner(&self) -> Result<Option<Arc<NativePresenceOwner>>, MatrixIpcError> {
        self.presence
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| core_state_error("p2-core-state-poisoned"))
    }

    fn verification_owner(&self) -> Result<Option<Arc<NativeVerificationOwner>>, MatrixIpcError> {
        self.verification
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| core_state_error("p2-core-state-poisoned"))
    }

    fn sync_owner(&self) -> Result<Option<Arc<SyncServiceOwner>>, MatrixIpcError> {
        self.sync
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| core_state_error("p2-core-state-poisoned"))
    }

    fn device_owner(&self) -> Result<Option<Arc<NativeDeviceOwner>>, MatrixIpcError> {
        self.devices
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| core_state_error("p2-core-state-poisoned"))
    }

    fn join_rule_owner(&self) -> Result<Option<Arc<NativeRoomJoinRuleOwner>>, MatrixIpcError> {
        self.join_rules
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| core_state_error("p2-core-state-poisoned"))
    }

    fn image_pack_owner(&self) -> Result<Option<Arc<NativeImagePackOwner>>, MatrixIpcError> {
        self.image_packs
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| core_state_error("p2-core-state-poisoned"))
    }

    fn timeline_owner(&self) -> Result<Option<Arc<NativeTimelineOwner>>, MatrixIpcError> {
        self.timelines
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| core_state_error("p2-core-state-poisoned"))
    }
}

/// Platform-neutral native engine root.
pub struct Core {
    state: Arc<CoreState>,
    registry: CommandRegistry,
}

impl Core {
    /// Build a core with the built-in P2 command handlers. P3 shells
    /// instantiate this once at startup; [`Self::with_registry`] remains for
    /// explicit construction and handler-focused tests.
    pub fn new(platform: Arc<dyn Platform>) -> Self {
        Self::with_registry(platform, built_in_registry())
    }

    pub fn with_registry(platform: Arc<dyn Platform>, registry: CommandRegistry) -> Self {
        Self {
            state: Arc::new(CoreState {
                platform,
                session: Mutex::new(None),
                typing: Mutex::new(None),
                presence: Mutex::new(None),
                verification: Mutex::new(None),
                devices: Mutex::new(None),
                join_rules: Mutex::new(None),
                image_packs: Mutex::new(None),
                timelines: Mutex::new(None),
                sync: Mutex::new(None),
            }),
            registry,
        }
    }

    /// Dispatch one validated `matrix_*` request to the registered core handler.
    pub async fn command(
        &self,
        request: CommandEnvelope,
    ) -> Result<CommandResponseEnvelope, MatrixIpcError> {
        request
            .validate()
            .map_err(|_| core_state_error("p2-command-invalid-envelope"))?;
        let handler = self
            .registry
            .handler(&request.command)
            .ok_or_else(|| core_state_error("p2-command-unregistered"))?;
        let response_payload = handler
            .handle(Arc::clone(&self.state), request.clone())
            .await?;
        Ok(request.response(response_payload))
    }

    /// Open a safe session projection. Credential material remains in the
    /// platform vault/session owner, never this DTO.
    pub async fn open(&self, session: SessionSnapshot) -> Result<(), MatrixIpcError> {
        let mut guard = self
            .state
            .session
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *guard = Some(session);
        Ok(())
    }

    /// Close the in-memory core projection. P2 deliberately does not erase
    /// platform persistence; lifecycle/destructive policies remain explicit.
    pub async fn close(&self) -> Result<(), MatrixIpcError> {
        let mut guard = self
            .state
            .session
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *guard = None;
        drop(guard);
        let mut typing = self
            .state
            .typing
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *typing = None;
        drop(typing);
        let mut presence = self
            .state
            .presence
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *presence = None;
        drop(presence);
        let mut verification = self
            .state
            .verification
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *verification = None;
        drop(verification);
        let mut devices = self
            .state
            .devices
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *devices = None;
        drop(devices);
        let mut join_rules = self
            .state
            .join_rules
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *join_rules = None;
        drop(join_rules);
        let mut image_packs = self
            .state
            .image_packs
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *image_packs = None;
        drop(image_packs);
        let mut timelines = self
            .state
            .timelines
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *timelines = None;
        drop(timelines);
        let mut sync = self
            .state
            .sync
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *sync = None;
        Ok(())
    }

    /// Install the live typing owner created by the shell after login/restore.
    /// Core snapshots it for `matrix_typing_snapshot`; the shell keeps an Arc
    /// for event-handler lifetime.
    pub fn attach_typing(&self, owner: Arc<NativeTypingOwner>) -> Result<(), MatrixIpcError> {
        let mut typing = self
            .state
            .typing
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *typing = Some(owner);
        Ok(())
    }

    /// Install the live presence owner created by the shell after login/restore.
    /// Core snapshots it for `matrix_presence_snapshot`; the shell keeps an Arc
    /// for subscribe/unsubscribe and event-handler lifetime.
    pub fn attach_presence(&self, owner: Arc<NativePresenceOwner>) -> Result<(), MatrixIpcError> {
        let mut presence = self
            .state
            .presence
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *presence = Some(owner);
        Ok(())
    }

    /// Install the live verification owner created by the shell after login/restore.
    /// Core lists it for `matrix_verification_list`; the shell keeps an Arc
    /// for request/SAS mutations.
    pub fn attach_verification(
        &self,
        owner: Arc<NativeVerificationOwner>,
    ) -> Result<(), MatrixIpcError> {
        let mut verification = self
            .state
            .verification
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *verification = Some(owner);
        Ok(())
    }

    /// Install the live device owner created by the shell after login/restore.
    /// Core snapshots it for `matrix_device_snapshot`; the shell keeps an Arc
    /// for the wakeup stream lifetime.
    pub fn attach_devices(&self, owner: Arc<NativeDeviceOwner>) -> Result<(), MatrixIpcError> {
        let mut devices = self
            .state
            .devices
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *devices = Some(owner);
        Ok(())
    }

    /// Install the live join-rule owner created by the shell after login/restore.
    pub fn attach_join_rules(
        &self,
        owner: Arc<NativeRoomJoinRuleOwner>,
    ) -> Result<(), MatrixIpcError> {
        let mut join_rules = self
            .state
            .join_rules
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *join_rules = Some(owner);
        Ok(())
    }

    /// Install the live image-pack owner created by the shell after login/restore.
    pub fn attach_image_packs(
        &self,
        owner: Arc<NativeImagePackOwner>,
    ) -> Result<(), MatrixIpcError> {
        let mut image_packs = self
            .state
            .image_packs
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *image_packs = Some(owner);
        Ok(())
    }

    /// Install the live timeline registry created by the shell after login/restore.
    pub fn attach_sync(&self, owner: Arc<SyncServiceOwner>) -> Result<(), MatrixIpcError> {
        let mut sync = self
            .state
            .sync
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *sync = Some(owner);
        Ok(())
    }

    pub fn attach_timelines(&self, owner: Arc<NativeTimelineOwner>) -> Result<(), MatrixIpcError> {
        let mut timelines = self
            .state
            .timelines
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *timelines = Some(owner);
        Ok(())
    }

    pub fn session_snapshot(&self) -> Result<Option<SessionSnapshot>, MatrixIpcError> {
        self.state.session_snapshot()
    }

    pub fn registered_commands(&self) -> Vec<String> {
        self.registry.command_names()
    }
}

fn built_in_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();
    registry
        .register("matrix_session_snapshot", matrix_session_snapshot)
        .expect("built-in matrix_session_snapshot must remain in the command census");
    registry
        .register("matrix_sync_status", matrix_sync_status)
        .expect("built-in matrix_sync_status must remain in the command census");
    registry
        .register("matrix_crypto_status", matrix_crypto_status)
        .expect("built-in matrix_crypto_status must remain in the command census");
    registry
        .register("matrix_cross_signing_status", matrix_cross_signing_status)
        .expect("built-in matrix_cross_signing_status must remain in the command census");
    registry
        .register("matrix_cross_signing_setup", matrix_cross_signing_setup)
        .expect("built-in matrix_cross_signing_setup must remain in the command census");
    registry
        .register("matrix_room_list_snapshot", matrix_room_list_snapshot)
        .expect("built-in matrix_room_list_snapshot must remain in the command census");
    registry
        .register("matrix_invites_accept", matrix_invites_accept)
        .expect("built-in matrix_invites_accept must remain in the command census");
    registry
        .register("matrix_invites_decline", matrix_invites_decline)
        .expect("built-in matrix_invites_decline must remain in the command census");
    registry
        .register("matrix_invites_snapshot", matrix_invites_snapshot)
        .expect("built-in matrix_invites_snapshot must remain in the command census");
    registry
        .register("matrix_secret_storage_status", matrix_secret_storage_status)
        .expect("built-in matrix_secret_storage_status must remain in the command census");
    registry
        .register("matrix_backup_status", matrix_backup_status)
        .expect("built-in matrix_backup_status must remain in the command census");
    registry
        .register(
            "matrix_room_key_transfer_status",
            matrix_room_key_transfer_status,
        )
        .expect("built-in matrix_room_key_transfer_status must remain in the command census");
    registry
        .register(
            "matrix_room_directory_protocols",
            matrix_room_directory_protocols,
        )
        .expect("built-in matrix_room_directory_protocols must remain in the command census");
    registry
        .register("matrix_room_directory_search", matrix_room_directory_search)
        .expect("built-in matrix_room_directory_search must remain in the command census");
    registry
        .register("matrix_room_directory_cancel", matrix_room_directory_cancel)
        .expect("built-in matrix_room_directory_cancel must remain in the command census");
    registry
        .register("matrix_send_text", matrix_send_text)
        .expect("built-in matrix_send_text must remain in the command census");
    registry
        .register("matrix_send_sticker", matrix_send_sticker)
        .expect("built-in matrix_send_sticker must remain in the command census");
    registry
        .register("matrix_send_poll", matrix_send_poll)
        .expect("built-in matrix_send_poll must remain in the command census");
    registry
        .register(
            "matrix_space_parents_snapshot",
            matrix_space_parents_snapshot,
        )
        .expect("built-in matrix_space_parents_snapshot must remain in the command census");
    registry
        .register(
            "matrix_space_hierarchy_snapshot",
            matrix_space_hierarchy_snapshot,
        )
        .expect("built-in matrix_space_hierarchy_snapshot must remain in the command census");
    registry
        .register(
            "matrix_space_children_snapshot",
            matrix_space_children_snapshot,
        )
        .expect("built-in matrix_space_children_snapshot must remain in the command census");
    registry
        .register("matrix_space_child_set", matrix_space_child_set)
        .expect("built-in matrix_space_child_set must remain in the command census");
    registry
        .register("matrix_space_child_remove", matrix_space_child_remove)
        .expect("built-in matrix_space_child_remove must remain in the command census");
    registry
        .register(
            "matrix_restricted_join_reparent",
            matrix_restricted_join_reparent,
        )
        .expect("built-in matrix_restricted_join_reparent must remain in the command census");
    registry
        .register("matrix_poll_respond", matrix_poll_respond)
        .expect("built-in matrix_poll_respond must remain in the command census");
    registry
        .register("matrix_edit_message", matrix_edit_message)
        .expect("built-in matrix_edit_message must remain in the command census");
    registry
        .register("matrix_media_config", matrix_media_config)
        .expect("built-in matrix_media_config must remain in the command census");
    registry
        .register("matrix_login_flows", matrix_login_flows)
        .expect("built-in matrix_login_flows must remain in the command census");
    registry
        .register("matrix_register_flows", matrix_register_flows)
        .expect("built-in matrix_register_flows must remain in the command census");
    registry
        .register("matrix_typing_snapshot", matrix_typing_snapshot)
        .expect("built-in matrix_typing_snapshot must remain in the command census");
    registry
        .register("matrix_presence_snapshot", matrix_presence_snapshot)
        .expect("built-in matrix_presence_snapshot must remain in the command census");
    registry
        .register("matrix_presence_subscribe", matrix_presence_subscribe)
        .expect("built-in matrix_presence_subscribe must remain in the command census");
    registry
        .register("matrix_presence_unsubscribe", matrix_presence_unsubscribe)
        .expect("built-in matrix_presence_unsubscribe must remain in the command census");
    registry
        .register("matrix_verification_accept", matrix_verification_accept)
        .expect("built-in matrix_verification_accept must remain in the command census");
    registry
        .register(
            "matrix_verification_begin_sas",
            matrix_verification_begin_sas,
        )
        .expect("built-in matrix_verification_begin_sas must remain in the command census");
    registry
        .register("matrix_verification_cancel", matrix_verification_cancel)
        .expect("built-in matrix_verification_cancel must remain in the command census");
    registry
        .register("matrix_verification_confirm", matrix_verification_confirm)
        .expect("built-in matrix_verification_confirm must remain in the command census");
    registry
        .register("matrix_verification_dismiss", matrix_verification_dismiss)
        .expect("built-in matrix_verification_dismiss must remain in the command census");
    registry
        .register("matrix_verification_list", matrix_verification_list)
        .expect("built-in matrix_verification_list must remain in the command census");
    registry
        .register("matrix_verification_mismatch", matrix_verification_mismatch)
        .expect("built-in matrix_verification_mismatch must remain in the command census");
    registry
        .register("matrix_verification_start", matrix_verification_start)
        .expect("built-in matrix_verification_start must remain in the command census");
    registry
        .register("matrix_device_snapshot", matrix_device_snapshot)
        .expect("built-in matrix_device_snapshot must remain in the command census");
    registry
        .register("matrix_device_rename", matrix_device_rename)
        .expect("built-in matrix_device_rename must remain in the command census");
    registry
        .register("matrix_device_delete_start", matrix_device_delete_start)
        .expect("built-in matrix_device_delete_start must remain in the command census");
    registry
        .register("matrix_device_delete_cancel", matrix_device_delete_cancel)
        .expect("built-in matrix_device_delete_cancel must remain in the command census");
    registry
        .register(
            "matrix_room_join_rule_snapshot",
            matrix_room_join_rule_snapshot,
        )
        .expect("built-in matrix_room_join_rule_snapshot must remain in the command census");
    registry
        .register("matrix_room_leave", matrix_room_leave)
        .expect("built-in matrix_room_leave must remain in the command census");
    registry
        .register("matrix_room_join", matrix_room_join)
        .expect("built-in matrix_room_join must remain in the command census");
    registry
        .register("matrix_room_invite", matrix_room_invite)
        .expect("built-in matrix_room_invite must remain in the command census");
    registry
        .register("matrix_room_kick", matrix_room_kick)
        .expect("built-in matrix_room_kick must remain in the command census");
    registry
        .register("matrix_room_ban", matrix_room_ban)
        .expect("built-in matrix_room_ban must remain in the command census");
    registry
        .register("matrix_room_create", matrix_room_create)
        .expect("built-in matrix_room_create must remain in the command census");
    registry
        .register("matrix_room_members_snapshot", matrix_room_members_snapshot)
        .expect("built-in matrix_room_members_snapshot must remain in the command census");
    registry
        .register(
            "matrix_room_power_levels_snapshot",
            matrix_room_power_levels_snapshot,
        )
        .expect("built-in matrix_room_power_levels_snapshot must remain in the command census");
    registry
        .register(
            "matrix_room_creators_snapshot",
            matrix_room_creators_snapshot,
        )
        .expect("built-in matrix_room_creators_snapshot must remain in the command census");
    registry
        .register(
            "matrix_room_power_level_tags_snapshot",
            matrix_room_power_level_tags_snapshot,
        )
        .expect("built-in matrix_room_power_level_tags_snapshot must remain in the command census");
    registry
        .register("matrix_room_unban", matrix_room_unban)
        .expect("built-in matrix_room_unban must remain in the command census");
    registry
        .register("matrix_room_set_power_level", matrix_room_set_power_level)
        .expect("built-in matrix_room_set_power_level must remain in the command census");
    registry
        .register("matrix_room_set_power_levels", matrix_room_set_power_levels)
        .expect("built-in matrix_room_set_power_levels must remain in the command census");
    registry
        .register(
            "matrix_room_set_power_level_tags",
            matrix_room_set_power_level_tags,
        )
        .expect("built-in matrix_room_set_power_level_tags must remain in the command census");
    registry
        .register("matrix_set_room_name", matrix_set_room_name)
        .expect("built-in matrix_set_room_name must remain in the command census");
    registry
        .register("matrix_set_room_topic", matrix_set_room_topic)
        .expect("built-in matrix_set_room_topic must remain in the command census");
    registry
        .register("matrix_set_room_avatar", matrix_set_room_avatar)
        .expect("built-in matrix_set_room_avatar must remain in the command census");
    registry
        .register(
            "matrix_get_room_directory_visibility",
            matrix_get_room_directory_visibility,
        )
        .expect("built-in matrix_get_room_directory_visibility must remain in the command census");
    registry
        .register(
            "matrix_set_room_directory_visibility",
            matrix_set_room_directory_visibility,
        )
        .expect("built-in matrix_set_room_directory_visibility must remain in the command census");
    registry
        .register(
            "matrix_get_global_image_packs",
            matrix_get_global_image_packs,
        )
        .expect("built-in matrix_get_global_image_packs must remain in the command census");
    registry
        .register("matrix_get_user_image_pack", matrix_get_user_image_pack)
        .expect("built-in matrix_get_user_image_pack must remain in the command census");
    registry
        .register("matrix_get_room_image_packs", matrix_get_room_image_packs)
        .expect("built-in matrix_get_room_image_packs must remain in the command census");
    registry
        .register("matrix_set_user_image_pack", matrix_set_user_image_pack)
        .expect("built-in matrix_set_user_image_pack must remain in the command census");
    registry
        .register(
            "matrix_set_global_image_packs",
            matrix_set_global_image_packs,
        )
        .expect("built-in matrix_set_global_image_packs must remain in the command census");
    registry
        .register("matrix_set_own_display_name", matrix_set_own_display_name)
        .expect("built-in matrix_set_own_display_name must remain in the command census");
    registry
        .register("matrix_set_own_avatar", matrix_set_own_avatar)
        .expect("built-in matrix_set_own_avatar must remain in the command census");
    registry
        .register("matrix_set_room_image_pack", matrix_set_room_image_pack)
        .expect("built-in matrix_set_room_image_pack must remain in the command census");
    registry
        .register("matrix_later_snapshot", matrix_later_snapshot)
        .expect("built-in matrix_later_snapshot must remain in the command census");
    registry
        .register("matrix_later_upsert", matrix_later_upsert)
        .expect("built-in matrix_later_upsert must remain in the command census");
    registry
        .register("matrix_later_complete", matrix_later_complete)
        .expect("built-in matrix_later_complete must remain in the command census");
    registry
        .register("matrix_later_snooze", matrix_later_snooze)
        .expect("built-in matrix_later_snooze must remain in the command census");
    registry
        .register("matrix_later_clear_completed", matrix_later_clear_completed)
        .expect("built-in matrix_later_clear_completed must remain in the command census");
    registry
        .register("matrix_later_mark_reminded", matrix_later_mark_reminded)
        .expect("built-in matrix_later_mark_reminded must remain in the command census");
    registry
        .register("matrix_room_notes_snapshot", matrix_room_notes_snapshot)
        .expect("built-in matrix_room_notes_snapshot must remain in the command census");
    registry
        .register("matrix_room_notes_upsert", matrix_room_notes_upsert)
        .expect("built-in matrix_room_notes_upsert must remain in the command census");
    registry
        .register("matrix_room_notes_delete", matrix_room_notes_delete)
        .expect("built-in matrix_room_notes_delete must remain in the command census");
    registry
        .register(
            "matrix_room_notes_complete_todo",
            matrix_room_notes_complete_todo,
        )
        .expect("built-in matrix_room_notes_complete_todo must remain in the command census");
    registry
        .register("matrix_room_notes_move_todo", matrix_room_notes_move_todo)
        .expect("built-in matrix_room_notes_move_todo must remain in the command census");
    registry
        .register("matrix_mdirect_snapshot", matrix_mdirect_snapshot)
        .expect("built-in matrix_mdirect_snapshot must remain in the command census");
    registry
        .register("matrix_mdirect_add", matrix_mdirect_add)
        .expect("built-in matrix_mdirect_add must remain in the command census");
    registry
        .register("matrix_mdirect_remove", matrix_mdirect_remove)
        .expect("built-in matrix_mdirect_remove must remain in the command census");
    registry
        .register("matrix_typing_set", matrix_typing_set)
        .expect("built-in matrix_typing_set must remain in the command census");
    registry
        .register("matrix_timeline_close", matrix_timeline_close)
        .expect("built-in matrix_timeline_close must remain in the command census");
    registry
        .register("matrix_timeline_open", matrix_timeline_open)
        .expect("built-in matrix_timeline_open must remain in the command census");
    registry
        .register("matrix_timeline_jump_latest", matrix_timeline_jump_latest)
        .expect("built-in matrix_timeline_jump_latest must remain in the command census");
    registry
        .register(
            "matrix_timeline_event_readback",
            matrix_timeline_event_readback,
        )
        .expect("built-in matrix_timeline_event_readback must remain in the command census");
    registry
        .register("matrix_timeline_paginate", matrix_timeline_paginate)
        .expect("built-in matrix_timeline_paginate must remain in the command census");
    registry
        .register(
            "matrix_timeline_reaction_toggle",
            matrix_timeline_reaction_toggle,
        )
        .expect("built-in matrix_timeline_reaction_toggle must remain in the command census");
    registry
        .register(
            "matrix_timeline_set_read_state",
            matrix_timeline_set_read_state,
        )
        .expect("built-in matrix_timeline_set_read_state must remain in the command census");
    registry
        .register("matrix_reaction_ensure", matrix_reaction_ensure)
        .expect("built-in matrix_reaction_ensure must remain in the command census");
    registry
        .register("matrix_reaction_redact", matrix_reaction_redact)
        .expect("built-in matrix_reaction_redact must remain in the command census");
    registry
        .register("matrix_timeline_edit_text", matrix_timeline_edit_text)
        .expect("built-in matrix_timeline_edit_text must remain in the command census");
    registry
        .register("matrix_timeline_redact", matrix_timeline_redact)
        .expect("built-in matrix_timeline_redact must remain in the command census");
    registry
        .register("matrix_timeline_report", matrix_timeline_report)
        .expect("built-in matrix_timeline_report must remain in the command census");
    registry
        .register("matrix_timeline_pin", matrix_timeline_pin)
        .expect("built-in matrix_timeline_pin must remain in the command census");
    registry
        .register("matrix_timeline_unpin", matrix_timeline_unpin)
        .expect("built-in matrix_timeline_unpin must remain in the command census");
    registry
        .register("matrix_timeline_poll_vote", matrix_timeline_poll_vote)
        .expect("built-in matrix_timeline_poll_vote must remain in the command census");
    registry
        .register("matrix_timeline_call_decline", matrix_timeline_call_decline)
        .expect("built-in matrix_timeline_call_decline must remain in the command census");
    registry
        .register("matrix_timeline_forward_text", matrix_timeline_forward_text)
        .expect("built-in matrix_timeline_forward_text must remain in the command census");
    registry
        .register(
            "matrix_timeline_forward_media",
            matrix_timeline_forward_media,
        )
        .expect("built-in matrix_timeline_forward_media must remain in the command census");
    registry
        .register(
            "matrix_composer_set_reply_draft",
            matrix_composer_set_reply_draft,
        )
        .expect("built-in matrix_composer_set_reply_draft must remain in the command census");
    registry
        .register(
            "matrix_composer_clear_reply_draft",
            matrix_composer_clear_reply_draft,
        )
        .expect("built-in matrix_composer_clear_reply_draft must remain in the command census");
    registry
        .register(
            "matrix_composer_get_reply_draft",
            matrix_composer_get_reply_draft,
        )
        .expect("built-in matrix_composer_get_reply_draft must remain in the command census");
    registry
}

fn matrix_typing_set(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTypingSetRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-typing-set-invalid-payload"))?;
        let owner = state.typing_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-typing-set-no-session")
        })?;
        owner
            .set(&payload.room_id, payload.typing)
            .await
            .map_err(typing_set_owner_error)?;
        Ok(serde_json::Value::Null)
    })
}

fn typing_set_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms.4-typing-invalid-room" => MatrixIpcErrorCategory::SdkInvariant,
        "v-rooms.4-typing-owner-user-missing" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn matrix_timeline_close(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineCloseRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-close-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-close-no-session")
        })?;
        let closed = owner.lock().await.close_view(NativeTimelineCloseRequest {
            stream_id: payload.stream_id,
        });
        Ok(serde_json::Value::Bool(closed))
    })
}

fn matrix_timeline_open(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineOpenRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-open-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-open-no-session")
        })?;
        let readback: NativeTimelineOpenReadback = owner
            .open_at(NativeTimelineOpenRequest {
                room_id: payload.room_id,
                position: payload.position,
            })
            .await
            .map_err(timeline_open_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-open-serialization-failed"))
    })
}

fn matrix_timeline_jump_latest(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineJumpLatestRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-jump-latest-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-jump-latest-no-session")
        })?;
        let readback: NativeTimelineOpenReadback = owner
            .jump_latest(NativeTimelineJumpLatestRequest {
                stream_id: payload.stream_id,
            })
            .await
            .map_err(timeline_open_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-jump-latest-serialization-failed"))
    })
}

fn timeline_open_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "d0.3-timeline-invalid-room-id"
        | "v-timeline-view-not-open"
        | "v-timeline-normal-room-not-found" => MatrixIpcErrorCategory::SdkInvariant,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn matrix_timeline_event_readback(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineEventReadbackRequest =
            serde_json::from_value(request.payload)
                .map_err(|_| core_state_error("p2-timeline-event-readback-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-event-readback-no-session")
        })?;
        let readback: NativeTimelineEventReadback = owner
            .event_readback(&payload.room_id, &payload.event_id)
            .await
            .map_err(timeline_event_readback_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-event-readback-serialization-failed"))
    })
}

fn timeline_event_readback_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "d0.3-timeline-invalid-room-id" | "v-crypto.6-invalid-event-id" => {
            MatrixIpcErrorCategory::SdkInvariant
        }
        "v-crypto.6-event-room-not-found" | "d0.3-timeline-room-not-found" => {
            MatrixIpcErrorCategory::Forbidden
        }
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn matrix_timeline_paginate(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelinePaginateRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-paginate-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-paginate-no-session")
        })?;
        let snapshot: TimelineViewSnapshot = owner
            .paginate(NativeTimelineViewPaginationRequest {
                stream_id: payload.stream_id,
                direction: payload.direction,
            })
            .await
            .map_err(timeline_paginate_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-timeline-paginate-serialization-failed"))
    })
}

fn timeline_paginate_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-timeline-view-not-open" => MatrixIpcErrorCategory::SdkInvariant,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn matrix_timeline_set_read_state(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineSetReadStateRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-set-read-state-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-set-read-state-no-session")
        })?;
        let readback: NativeTimelineReadStateReadback = owner
            .set_read_state(NativeTimelineReadStateRequest {
                stream_id: payload.stream_id,
                action: payload.action,
            })
            .await
            .map_err(timeline_paginate_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-set-read-state-serialization-failed"))
    })
}

fn matrix_timeline_reaction_toggle(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineReactionKeyRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-reaction-toggle-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-reaction-toggle-no-session")
        })?;
        let result: NativeReactionMutationResult = owner
            .toggle_reaction(&payload.room_id, &payload.event_id, &payload.key)
            .await
            .map_err(timeline_reaction_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-timeline-reaction-toggle-serialization-failed"))
    })
}

fn matrix_reaction_ensure(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineReactionKeyRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-reaction-ensure-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-reaction-ensure-no-session")
        })?;
        let result: NativeReactionMutationResult = owner
            .ensure_reaction(&payload.room_id, &payload.event_id, &payload.key)
            .await
            .map_err(timeline_reaction_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-reaction-ensure-serialization-failed"))
    })
}

fn matrix_reaction_redact(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixReactionRedactRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-reaction-redact-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-reaction-redact-no-session")
        })?;
        let result: NativeReactionMutationResult = owner
            .redact_reaction(
                &payload.room_id,
                &payload.target_event_id,
                &payload.reaction_event_id,
                &payload.key,
            )
            .await
            .map_err(timeline_reaction_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-reaction-redact-serialization-failed"))
    })
}

fn timeline_reaction_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "d0.3-timeline-invalid-room-id"
        | "v-crypto.6-invalid-event-id"
        | "v-send.2-reaction-invalid-key"
        | "v-send.2-reaction-redact-annotation-not-found" => MatrixIpcErrorCategory::SdkInvariant,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn matrix_send_text(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSendTextRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-send-text-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-send-text-no-session")
        })?;
        let result: MatrixSendTextResult = owner
            .send_text(
                payload.room_id,
                payload.body,
                payload.msg_type,
                payload.formatted_body,
                payload.mention_user_ids,
                payload.mention_room,
                payload.reply_to,
                payload.thread_root,
                payload.txn_id,
            )
            .await
            .map_err(send_text_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-send-text-serialization-failed"))
    })
}

fn send_text_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "d0.4-send-invalid-room-id"
        | "d0.4-send-invalid-reply-event-id"
        | "d0.4-send-invalid-transaction-id"
        | "d0.4-send-room-not-found"
        | "v-send.4-invalid-message-type"
        | "v-send.4-invalid-mention-user-id"
        | "v-send.5-invalid-thread-root-event-id"
        | "v-send.r-edit-invalid-event-id"
        | "v-send.r-edit-room-not-found"
        | "v-send-sticker-invalid-body"
        | "v-send-sticker-invalid-mxc"
        | "v-send-sticker-invalid-mimetype"
        | "v-send-sticker-room-not-found"
        | "v-send.3-poll-invalid-question"
        | "v-send.3-poll-invalid-answers"
        | "v-send.3-poll-invalid-event-id"
        | "v-send.3-poll-invalid-answer-ids"
        | "v-send.3-poll-room-not-found"
        | "p6.1-invalid-room-id"
        | "p6.1-empty-body"
        | "p6.1-body-too-large" => MatrixIpcErrorCategory::SdkInvariant,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn matrix_send_sticker(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSendStickerRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-send-sticker-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-send-sticker-no-session")
        })?;
        let result: MatrixSendStickerResult = owner
            .send_sticker(
                payload.room_id,
                payload.body,
                payload.mxc,
                payload.width,
                payload.height,
                payload.mimetype,
                payload.size,
                payload.reply_to,
                payload.thread_root,
            )
            .await
            .map_err(send_text_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-send-sticker-serialization-failed"))
    })
}

fn matrix_send_poll(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSendPollRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-send-poll-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-send-poll-no-session")
        })?;
        let result: MatrixSendPollResult = owner
            .send_poll(
                payload.room_id,
                payload.question,
                payload.answers,
                payload.max_selections,
                payload.thread_root,
                payload.reply_to,
            )
            .await
            .map_err(send_text_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-send-poll-serialization-failed"))
    })
}

fn matrix_poll_respond(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixPollRespondRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-poll-respond-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-poll-respond-no-session")
        })?;
        let result: MatrixPollRespondResult = owner
            .poll_respond(payload.room_id, payload.poll_event_id, payload.answer_ids)
            .await
            .map_err(send_text_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-poll-respond-serialization-failed"))
    })
}

fn matrix_edit_message(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixEditMessageRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-edit-message-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-edit-message-no-session")
        })?;
        let result: MatrixSendTextResult = owner
            .edit_message(
                payload.room_id,
                payload.event_id,
                payload.body,
                payload.msg_type,
                payload.formatted_body,
                payload.mention_user_ids,
                payload.mention_room,
                payload.txn_id,
            )
            .await
            .map_err(send_text_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-edit-message-serialization-failed"))
    })
}

fn matrix_timeline_edit_text(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineEditTextRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-edit-text-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-edit-text-no-session")
        })?;
        let readback: NativeTimelineActionReadback = owner
            .edit_text(
                &payload.room_id,
                &payload.event_id,
                &payload.body,
                payload.formatted_body.as_deref(),
            )
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-edit-text-serialization-failed"))
    })
}

fn matrix_timeline_redact(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineRedactRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-redact-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-redact-no-session")
        })?;
        let readback: NativeTimelineActionReadback = owner
            .redact_event(
                &payload.room_id,
                &payload.event_id,
                payload.reason.as_deref(),
            )
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-redact-serialization-failed"))
    })
}

fn matrix_timeline_report(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineReportRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-report-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-report-no-session")
        })?;
        let readback: NativeTimelineActionReadback = owner
            .report(
                &payload.room_id,
                &payload.event_id,
                payload.reason.as_deref(),
            )
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-report-serialization-failed"))
    })
}

fn matrix_timeline_pin(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelinePinRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-pin-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-pin-no-session")
        })?;
        let readback: NativeTimelineActionReadback = owner
            .pin_event(&payload.room_id, &payload.event_id)
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-pin-serialization-failed"))
    })
}

fn matrix_timeline_unpin(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelinePinRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-unpin-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-unpin-no-session")
        })?;
        let readback: NativeTimelineActionReadback = owner
            .unpin_event(&payload.room_id, &payload.event_id)
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-unpin-serialization-failed"))
    })
}

fn matrix_timeline_poll_vote(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelinePollVoteRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-poll-vote-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-poll-vote-no-session")
        })?;
        let readback: NativeTimelineActionReadback = owner
            .poll_vote(&payload.room_id, &payload.event_id, payload.answer_ids)
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-poll-vote-serialization-failed"))
    })
}

fn matrix_timeline_call_decline(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineCallDeclineRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-call-decline-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-call-decline-no-session")
        })?;
        let readback: NativeTimelineActionReadback = owner
            .decline_call(&payload.room_id, &payload.event_id)
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-call-decline-serialization-failed"))
    })
}

fn matrix_timeline_forward_text(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineForwardTextRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-forward-text-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-forward-text-no-session")
        })?;
        let readback: NativeTimelineActionReadback = owner
            .forward_text(
                &payload.source_room_id,
                &payload.event_id,
                &payload.target_room_id,
                payload.as_quote,
            )
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-forward-text-serialization-failed"))
    })
}

fn matrix_timeline_forward_media(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixTimelineForwardMediaRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-timeline-forward-media-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-forward-media-no-session")
        })?;
        let readback: NativeTimelineActionReadback = owner
            .forward_media(
                &payload.source_room_id,
                &payload.event_id,
                &payload.target_room_id,
            )
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-timeline-forward-media-serialization-failed"))
    })
}

fn matrix_composer_set_reply_draft(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixComposerSetReplyDraftRequest =
            serde_json::from_value(request.payload)
                .map_err(|_| core_state_error("p2-composer-set-reply-draft-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-composer-set-reply-draft-no-session")
        })?;
        let readback: NativeComposerReplyDraftReadback = owner
            .set_reply_draft(&payload.room_id, &payload.event_id, payload.start_thread)
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-composer-set-reply-draft-serialization-failed"))
    })
}

fn matrix_composer_clear_reply_draft(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixComposerReplyDraftRoomRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-composer-clear-reply-draft-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-composer-clear-reply-draft-no-session")
        })?;
        let readback: NativeComposerReplyDraftReadback = owner
            .clear_reply_draft(&payload.room_id)
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-composer-clear-reply-draft-serialization-failed"))
    })
}

fn matrix_composer_get_reply_draft(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixComposerReplyDraftRoomRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-composer-get-reply-draft-invalid-payload"))?;
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-composer-get-reply-draft-no-session")
        })?;
        let readback: NativeComposerReplyDraftReadback = owner
            .get_reply_draft(&payload.room_id)
            .await
            .map_err(timeline_action_owner_error)?;
        serde_json::to_value(readback)
            .map_err(|_| core_state_error("p2-composer-get-reply-draft-serialization-failed"))
    })
}

fn timeline_action_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "d0.4-send-invalid-room-id"
        | "d0.4-send-formatted-body-too-large"
        | "v-timeline-edit-invalid-event-id"
        | "v-timeline-edit-empty-body"
        | "v-timeline-edit-room-not-found"
        | "v-timeline-redact-invalid-event-id"
        | "v-timeline-redact-room-not-found"
        | "v-timeline-report-invalid-event-id"
        | "v-timeline-report-room-not-found"
        | "v-timeline-pin-invalid-event-id"
        | "v-timeline-pin-room-not-found"
        | "v-timeline-unpin-invalid-event-id"
        | "v-timeline-unpin-room-not-found"
        | "v-timeline-poll-vote-invalid-event-id"
        | "v-timeline-poll-vote-room-not-found"
        | "v-timeline-call-decline-invalid-event-id"
        | "v-timeline-call-decline-room-not-found"
        | "v-timeline-call-decline-own-call"
        | "v-timeline-call-decline-bad-event-type"
        | "v-timeline-forward-invalid-event-id"
        | "v-timeline-forward-source-room-not-found"
        | "v-timeline-forward-target-room-not-found"
        | "v-timeline-forward-event-unavailable"
        | "v-timeline-forward-event-decode-failed"
        | "v-timeline-forward-event-redacted"
        | "v-timeline-forward-unsupported-event"
        | "v-timeline-forward-media-invalid-event-id"
        | "v-timeline-forward-media-source-room-not-found"
        | "v-timeline-forward-media-target-room-not-found"
        | "v-timeline-forward-media-event-unavailable"
        | "v-timeline-forward-media-event-decode-failed"
        | "v-timeline-forward-media-event-redacted"
        | "v-timeline-forward-media-unsupported-event"
        | "v-timeline-reply-draft-invalid-event-id"
        | "v-timeline-reply-draft-room-not-found"
        | "v-timeline-reply-draft-event-unavailable"
        | "v-timeline-reply-draft-event-decode-failed"
        | "v-timeline-reply-draft-event-redacted"
        | "v-timeline-reply-draft-unsupported-event" => MatrixIpcErrorCategory::SdkInvariant,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn matrix_typing_snapshot(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-typing-snapshot-invalid-payload"));
        }
        let owner = state.typing_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-typing-snapshot-no-session")
        })?;
        let snapshot: NativeTypingSnapshot = owner.snapshot().await;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-typing-snapshot-serialization-failed"))
    })
}

fn matrix_presence_snapshot(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixPresenceSnapshotRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-presence-snapshot-invalid-payload"))?;
        let owner = state.presence_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-presence-snapshot-no-session")
        })?;
        let snapshot: NativePresenceSnapshotResult = owner
            .snapshot(&payload.user_id)
            .await
            .map_err(presence_snapshot_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-presence-snapshot-serialization-failed"))
    })
}

fn matrix_presence_subscribe(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixPresenceSubscribeRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-presence-subscribe-invalid-payload"))?;
        let owner = state.presence_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-presence-subscribe-no-session")
        })?;
        let subscription: NativePresenceSubscription = owner
            .subscribe(&payload.user_id)
            .await
            .map_err(presence_snapshot_owner_error)?;
        serde_json::to_value(subscription)
            .map_err(|_| core_state_error("p2-presence-subscribe-serialization-failed"))
    })
}

fn matrix_presence_unsubscribe(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixPresenceUnsubscribeRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-presence-unsubscribe-invalid-payload"))?;
        let owner = state.presence_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-presence-unsubscribe-no-session")
        })?;
        owner
            .unsubscribe(&payload.subscription_id)
            .await
            .map_err(presence_snapshot_owner_error)?;
        Ok(serde_json::Value::Null)
    })
}

/// Map live presence-owner diagnostics onto closed Core transport categories.
/// Preserve the owner diagnostic id so the desktop bridge can restore the
/// established Tauri error shape without leaking user ids or status text.
fn matrix_verification_list(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-verification-list-invalid-payload"));
        }
        let owner = state.verification_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-verification-list-no-session")
        })?;
        let inbox: NativeVerificationInbox = owner.list().await;
        serde_json::to_value(inbox)
            .map_err(|_| core_state_error("p2-verification-list-serialization-failed"))
    })
}

fn matrix_verification_accept(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixVerificationAcceptRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-verification-accept-invalid-payload"))?;
        let owner = state.verification_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-verification-accept-no-session")
        })?;
        let request: NativeVerificationRequest = owner
            .accept(&payload.flow_id)
            .await
            .map_err(verification_accept_owner_error)?;
        serde_json::to_value(request)
            .map_err(|_| core_state_error("p2-verification-accept-serialization-failed"))
    })
}

fn verification_accept_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-crypto.1-flow-not-found"
        | "v-crypto.1-sas-invalid-state"
        | "v-crypto.1-confirm-before-sas"
        | "v-crypto.1-sas-unavailable"
        | "v-crypto.1-dismiss-active-flow"
        | "v-crypto.1-device-not-found" => MatrixIpcErrorCategory::SdkInvariant,
        "v-crypto.1-start-requires-session" => MatrixIpcErrorCategory::Forbidden,
        "v-crypto.1-own-identity-unavailable" => MatrixIpcErrorCategory::UnsupportedCapability,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn matrix_verification_begin_sas(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixVerificationBeginSasRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-verification-begin-sas-invalid-payload"))?;
        let owner = state.verification_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-verification-begin-sas-no-session")
        })?;
        let request: NativeVerificationRequest = owner
            .begin_sas(&payload.flow_id)
            .await
            .map_err(verification_accept_owner_error)?;
        serde_json::to_value(request)
            .map_err(|_| core_state_error("p2-verification-begin-sas-serialization-failed"))
    })
}

fn matrix_verification_cancel(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixVerificationCancelRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-verification-cancel-invalid-payload"))?;
        let owner = state.verification_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-verification-cancel-no-session")
        })?;
        let request: NativeVerificationRequest = owner
            .cancel(&payload.flow_id)
            .await
            .map_err(verification_accept_owner_error)?;
        serde_json::to_value(request)
            .map_err(|_| core_state_error("p2-verification-cancel-serialization-failed"))
    })
}

fn matrix_verification_confirm(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixVerificationConfirmRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-verification-confirm-invalid-payload"))?;
        let owner = state.verification_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-verification-confirm-no-session")
        })?;
        let request: NativeVerificationRequest = owner
            .confirm(&payload.flow_id)
            .await
            .map_err(verification_accept_owner_error)?;
        serde_json::to_value(request)
            .map_err(|_| core_state_error("p2-verification-confirm-serialization-failed"))
    })
}

fn matrix_verification_dismiss(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixVerificationDismissRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-verification-dismiss-invalid-payload"))?;
        let owner = state.verification_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-verification-dismiss-no-session")
        })?;
        owner
            .dismiss(&payload.flow_id)
            .await
            .map_err(verification_accept_owner_error)?;
        Ok(serde_json::Value::Null)
    })
}

fn matrix_verification_mismatch(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixVerificationMismatchRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-verification-mismatch-invalid-payload"))?;
        let owner = state.verification_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-verification-mismatch-no-session")
        })?;
        let request: NativeVerificationRequest = owner
            .mismatch(&payload.flow_id)
            .await
            .map_err(verification_accept_owner_error)?;
        serde_json::to_value(request)
            .map_err(|_| core_state_error("p2-verification-mismatch-serialization-failed"))
    })
}

fn matrix_verification_start(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixVerificationStartRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-verification-start-invalid-payload"))?;
        let owner = state.verification_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-verification-start-no-session")
        })?;
        let request: NativeVerificationRequest = owner
            .start(payload.device_id)
            .await
            .map_err(verification_accept_owner_error)?;
        serde_json::to_value(request)
            .map_err(|_| core_state_error("p2-verification-start-serialization-failed"))
    })
}

fn matrix_backup_status(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-backup-status-invalid-payload"));
        }
        let owner = state.device_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-backup-status-no-session")
        })?;
        let snapshot: NativeBackupStatus = owner
            .backup_status()
            .await
            .map_err(backup_status_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-backup-status-serialization-failed"))
    })
}

fn backup_status_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    MatrixIpcError::new(MatrixIpcErrorCategory::Unknown).with_diagnostic(diagnostic_id)
}

fn matrix_room_key_transfer_status(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error(
                "p2-room-key-transfer-status-invalid-payload",
            ));
        }
        let owner = state.device_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-key-transfer-status-no-session")
        })?;
        let snapshot: NativeRoomKeyTransferStatus = owner.room_key_status().await;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-room-key-transfer-status-serialization-failed"))
    })
}

fn matrix_cross_signing_setup(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-cross-signing-setup-invalid-payload"));
        }
        let owner = state.device_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-cross-signing-setup-no-session")
        })?;
        let result: NativeCrossSigningSetupResult = owner
            .cross_signing_setup()
            .await
            .map_err(cross_signing_setup_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-cross-signing-setup-serialization-failed"))
    })
}

fn cross_signing_setup_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-crypto.2-cross-signing-auth-unsupported" => MatrixIpcErrorCategory::Forbidden,
        "v-crypto.2-cross-signing-user-missing" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn matrix_room_list_snapshot(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-room-list-snapshot-invalid-payload"));
        }
        let owner = state.sync_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-list-snapshot-no-session")
        })?;
        let snapshot: NativeRoomListSnapshot = snapshot_from_sync_owner(&owner)
            .await
            .map_err(room_list_snapshot_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-room-list-snapshot-serialization-failed"))
    })
}

fn room_list_snapshot_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    MatrixIpcError::new(MatrixIpcErrorCategory::Unknown).with_diagnostic(diagnostic_id)
}

fn matrix_invites_snapshot(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-invites-snapshot-invalid-payload"));
        }
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-invites-snapshot-no-session")
        })?;
        let snapshot: NativeInviteSnapshot = owner
            .invites_snapshot()
            .await
            .map_err(invites_snapshot_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-invites-snapshot-serialization-failed"))
    })
}

fn matrix_invites_accept(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixInviteActionRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-invites-accept-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-invites-accept-no-session")
        })?;
        let snapshot: NativeInviteSnapshot = owner
            .invite_accept(&payload.room_id)
            .await
            .map_err(invite_action_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-invites-accept-serialization-failed"))
    })
}

fn matrix_invites_decline(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixInviteActionRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-invites-decline-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-invites-decline-no-session")
        })?;
        let snapshot: NativeInviteSnapshot = owner
            .invite_decline(&payload.room_id)
            .await
            .map_err(invite_action_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-invites-decline-serialization-failed"))
    })
}

fn invites_snapshot_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms.1-invites-requires-session"
        | "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn invite_action_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms.1-invite-invalid-room"
        | "v-rooms.1-invite-invalid-sender"
        | "v-rooms.1-invite-not-found"
        | "v-rooms.1-invite-member-missing" => MatrixIpcErrorCategory::SdkInvariant,
        "v-rooms.1-invites-requires-session"
        | "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn matrix_room_directory_protocols(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error(
                "p2-room-directory-protocols-invalid-payload",
            ));
        }
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-directory-protocols-no-session")
        })?;
        let snapshot: NativeRoomDirectoryProtocols = owner
            .directory_protocols()
            .await
            .map_err(directory_protocols_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-room-directory-protocols-serialization-failed"))
    })
}

fn directory_protocols_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms.directory-protocol-id-cap"
        | "v-rooms.directory-protocol-instance-invalid"
        | "v-rooms.directory-protocol-instance-cap" => MatrixIpcErrorCategory::SdkInvariant,
        "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn directory_correlation_invalid(session_generation: u64, request_id: u64) -> bool {
    session_generation == 0
        || request_id == 0
        || session_generation > MAX_WIRE_COUNTER
        || request_id > MAX_WIRE_COUNTER
}

fn matrix_room_directory_search(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomDirectorySearchRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-directory-search-invalid-payload"))?;
        if directory_correlation_invalid(payload.session_generation, payload.request_id) {
            return Err(MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                .with_diagnostic("v-rooms.directory-invalid-correlation"));
        }
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-directory-search-no-session")
        })?;
        let result: NativeRoomDirectorySearchResponse = owner
            .directory_search(
                payload.session_generation,
                payload.request_id,
                DirectorySearchInput {
                    server_name: payload.server_name,
                    term: payload.term,
                    room_type: payload.room_type,
                    third_party_instance_id: payload.third_party_instance_id,
                    limit: payload.limit,
                    since: payload.since,
                },
            )
            .await
            .map_err(directory_search_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-room-directory-search-serialization-failed"))
    })
}

fn matrix_room_directory_cancel(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomDirectoryCancelRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-directory-cancel-invalid-payload"))?;
        if directory_correlation_invalid(payload.session_generation, payload.request_id) {
            return Err(MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                .with_diagnostic("v-rooms.directory-invalid-correlation"));
        }
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-directory-cancel-no-session")
        })?;
        let result: NativeRoomDirectorySearchResponse = owner
            .directory_cancel(payload.session_generation, payload.request_id)
            .map_err(directory_search_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-room-directory-cancel-serialization-failed"))
    })
}

fn directory_search_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms.directory-invalid-correlation"
        | "v-rooms.directory-invalid-limit"
        | "v-rooms.directory-invalid-server"
        | "v-rooms.directory-invalid-term"
        | "v-rooms.directory-invalid-instance"
        | "v-rooms.directory-invalid-since" => MatrixIpcErrorCategory::SdkInvariant,
        "v-rooms.directory-stale-generation-before-request"
        | "v-rooms.directory-stale-generation-after-request"
        | "v-rooms.directory-cancel-stale-generation" => {
            MatrixIpcErrorCategory::StaleSessionGeneration
        }
        "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn matrix_device_snapshot(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-device-snapshot-invalid-payload"));
        }
        let owner = state.device_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-device-snapshot-no-session")
        })?;
        let snapshot: NativeDeviceSnapshot = owner
            .snapshot()
            .await
            .map_err(device_snapshot_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-device-snapshot-serialization-failed"))
    })
}

fn matrix_device_rename(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixDeviceRenameRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-device-rename-invalid-payload"))?;
        let owner = state.device_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-device-rename-no-session")
        })?;
        let snapshot: NativeDeviceSnapshot = owner
            .rename(&payload.device_id, &payload.display_name)
            .await
            .map_err(device_snapshot_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-device-rename-serialization-failed"))
    })
}

fn matrix_device_delete_start(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixDeviceDeleteStartRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-device-delete-start-invalid-payload"))?;
        let owner = state.device_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-device-delete-start-no-session")
        })?;
        let result: NativeDeviceDeleteResult = owner
            .delete_start(payload.device_ids)
            .await
            .map_err(device_snapshot_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-device-delete-start-serialization-failed"))
    })
}

fn matrix_device_delete_cancel(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixDeviceDeleteCancelRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-device-delete-cancel-invalid-payload"))?;
        let owner = state.device_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-device-delete-cancel-no-session")
        })?;
        owner
            .delete_cancel(payload.operation_id, payload.session_generation)
            .map_err(device_snapshot_owner_error)?;
        Ok(serde_json::Value::Null)
    })
}

fn matrix_room_join_rule_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomJoinRuleSnapshotRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-join-rule-snapshot-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-join-rule-snapshot-no-session")
        })?;
        let snapshot: MatrixRoomJoinRuleSnapshot = owner
            .snapshot(&payload.room_id, payload.session_generation)
            .await
            .map_err(join_rule_snapshot_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-join-rule-snapshot-serialization-failed"))
    })
}

fn matrix_room_leave(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomLeaveRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-leave-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-leave-no-session")
        })?;
        owner
            .leave(&payload.room_id)
            .await
            .map_err(room_leave_join_owner_error)?;
        Ok(serde_json::Value::Null)
    })
}

fn matrix_room_join(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomJoinRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-join-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-join-no-session")
        })?;
        owner
            .join(&payload.room_id_or_alias, payload.via_servers)
            .await
            .map_err(room_leave_join_owner_error)?;
        Ok(serde_json::Value::Null)
    })
}

fn room_leave_join_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms-room-leave-invalid-room"
        | "v-rooms-room-leave-room-not-found"
        | "v-rooms-room-join-invalid-room"
        | "v-rooms-room-join-invalid-via-server" => MatrixIpcErrorCategory::SdkInvariant,
        "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn matrix_room_invite(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomModerationRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-invite-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-invite-no-session")
        })?;
        owner
            .invite(&payload.room_id, &payload.user_id, payload.reason)
            .await
            .map_err(room_moderation_owner_error)?;
        Ok(serde_json::Value::Null)
    })
}

fn matrix_room_kick(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomModerationRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-kick-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-kick-no-session")
        })?;
        owner
            .kick(&payload.room_id, &payload.user_id, payload.reason)
            .await
            .map_err(room_moderation_owner_error)?;
        Ok(serde_json::Value::Null)
    })
}

fn matrix_room_ban(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomModerationRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-ban-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-ban-no-session")
        })?;
        owner
            .ban(&payload.room_id, &payload.user_id, payload.reason)
            .await
            .map_err(room_moderation_owner_error)?;
        Ok(serde_json::Value::Null)
    })
}

fn matrix_room_unban(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomUnbanRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-unban-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-unban-no-session")
        })?;
        owner
            .unban(&payload.room_id, &payload.user_id)
            .await
            .map_err(room_moderation_owner_error)?;
        Ok(serde_json::Value::Null)
    })
}

fn matrix_room_create(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomCreateRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-create-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-create-no-session")
        })?;
        let room_id = owner
            .create_room(payload)
            .await
            .map_err(room_create_owner_error)?;
        Ok(serde_json::Value::String(room_id))
    })
}

fn matrix_room_members_snapshot(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomMembersSnapshotRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-members-snapshot-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-members-snapshot-no-session")
        })?;
        let snapshot: NativeRoomMembersSnapshot = owner
            .members_snapshot(&payload.room_id)
            .await
            .map_err(room_members_snapshot_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-room-members-snapshot-serialization-failed"))
    })
}

fn matrix_room_power_levels_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomMembersSnapshotRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-power-levels-snapshot-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-power-levels-snapshot-no-session")
        })?;
        let snapshot: NativeRoomPowerLevelsSnapshot = owner
            .power_levels_snapshot(&payload.room_id)
            .await
            .map_err(room_members_snapshot_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-room-power-levels-snapshot-serialization-failed"))
    })
}

fn matrix_room_creators_snapshot(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomMembersSnapshotRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-creators-snapshot-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-creators-snapshot-no-session")
        })?;
        let snapshot: NativeRoomCreatorsSnapshot = owner
            .creators_snapshot(&payload.room_id)
            .await
            .map_err(room_members_snapshot_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-room-creators-snapshot-serialization-failed"))
    })
}

fn matrix_room_power_level_tags_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomMembersSnapshotRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-power-level-tags-snapshot-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-power-level-tags-snapshot-no-session")
        })?;
        let snapshot: NativeRoomPowerLevelTagsSnapshot = owner
            .power_level_tags_snapshot(&payload.room_id)
            .await
            .map_err(room_members_snapshot_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-room-power-level-tags-snapshot-serialization-failed"))
    })
}

fn room_members_snapshot_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms-members-read-invalid-room" | "v-rooms-members-read-room-not-found" => {
            MatrixIpcErrorCategory::SdkInvariant
        }
        "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn room_create_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms-room-create-invalid-name"
        | "v-rooms-room-create-invalid-topic"
        | "v-rooms-room-create-invalid-room-version"
        | "v-rooms-room-create-invalid-alias"
        | "v-rooms-room-create-invalid-invite"
        | "v-rooms-room-create-invalid-creation-content"
        | "v-rooms-room-create-invalid-additional-creator"
        | "v-rooms-room-create-invalid-parent"
        | "v-rooms-room-create-invalid-join-rule"
        | "v-rooms-room-create-missing-restricted-parent"
        | "v-rooms-room-create-invalid-power-level" => MatrixIpcErrorCategory::SdkInvariant,
        "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn matrix_space_parents_snapshot(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error(
                "p2-space-parents-snapshot-invalid-payload",
            ));
        }
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-space-parents-snapshot-no-session")
        })?;
        let snapshot: NativeSpaceParentsSnapshot = owner
            .space_parents_snapshot()
            .await
            .map_err(space_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-space-parents-snapshot-serialization-failed"))
    })
}

fn matrix_space_hierarchy_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSpaceHierarchySnapshotRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-space-hierarchy-snapshot-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-space-hierarchy-snapshot-no-session")
        })?;
        let snapshot: NativeSpaceHierarchySnapshot = owner
            .space_hierarchy_snapshot(&payload.room_id)
            .await
            .map_err(space_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-space-hierarchy-snapshot-serialization-failed"))
    })
}

fn matrix_space_children_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error(
                "p2-space-children-snapshot-invalid-payload",
            ));
        }
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-space-children-snapshot-no-session")
        })?;
        let snapshot: NativeSpaceChildrenSnapshot = owner
            .space_children_snapshot()
            .await
            .map_err(space_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-space-children-snapshot-serialization-failed"))
    })
}

fn matrix_space_child_set(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSpaceChildSetRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-space-child-set-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-space-child-set-no-session")
        })?;
        let result: NativeSpaceChildMutationResult = owner
            .space_child_set(
                &payload.parent_id,
                &payload.child_id,
                &payload.via,
                payload.order.as_deref(),
                payload.suggested,
            )
            .await
            .map_err(space_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-space-child-set-serialization-failed"))
    })
}

fn matrix_space_child_remove(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSpaceChildRemoveRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-space-child-remove-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-space-child-remove-no-session")
        })?;
        let result: NativeSpaceChildMutationResult = owner
            .space_child_remove(&payload.parent_id, &payload.child_id)
            .await
            .map_err(space_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-space-child-remove-serialization-failed"))
    })
}

fn matrix_restricted_join_reparent(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRestrictedJoinReparentRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-restricted-join-reparent-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-restricted-join-reparent-no-session")
        })?;
        let result: NativeRestrictedJoinReparentResult = owner
            .restricted_join_reparent(
                &payload.room_id,
                payload.remove_parent_id.as_deref(),
                &payload.add_parent_id,
            )
            .await
            .map_err(space_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-restricted-join-reparent-serialization-failed"))
    })
}

fn space_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms.2c-invalid-parent"
        | "v-rooms.2c-invalid-child"
        | "v-rooms.2c-invalid-room"
        | "v-rooms.2c-invalid-via"
        | "v-rooms.2c-invalid-order"
        | "v-rooms.2b-space-hierarchy-invalid-room"
        | "v-rooms.2c-room-missing"
        | "v-rooms.2c-room-not-joined" => MatrixIpcErrorCategory::SdkInvariant,
        "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn room_moderation_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms-members-moderation-invalid-room"
        | "v-rooms-members-moderation-invalid-user"
        | "v-rooms-members-moderation-invalid-power-level"
        | "v-rooms-members-moderation-room-not-found" => MatrixIpcErrorCategory::SdkInvariant,
        "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn matrix_room_set_power_level(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomSetPowerLevelRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-set-power-level-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-set-power-level-no-session")
        })?;
        owner
            .set_power_level(&payload.room_id, &payload.user_id, payload.power_level)
            .await
            .map_err(room_moderation_owner_error)?;
        Ok(serde_json::Value::Null)
    })
}

fn matrix_room_set_power_levels(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomSetPowerLevelStateRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-set-power-levels-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-set-power-levels-no-session")
        })?;
        let result: NativePowerLevelWriteResult = owner
            .set_power_level_state(
                &payload.room_id,
                payload.content,
                ROOM_POWER_LEVELS_EVENT_TYPE,
            )
            .await
            .map_err(room_power_level_state_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-room-set-power-levels-serialization-failed"))
    })
}

fn matrix_room_set_power_level_tags(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomSetPowerLevelStateRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-set-power-level-tags-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-set-power-level-tags-no-session")
        })?;
        let result: NativePowerLevelWriteResult = owner
            .set_power_level_state(
                &payload.room_id,
                payload.content,
                ROOM_POWER_LEVEL_TAGS_EVENT_TYPE,
            )
            .await
            .map_err(room_power_level_state_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-room-set-power-level-tags-serialization-failed"))
    })
}

fn room_power_level_state_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms-power-levels-invalid-room"
        | "v-rooms-power-levels-invalid-content"
        | "v-rooms-power-levels-invalid-power"
        | "v-rooms-power-levels-invalid-power-map"
        | "v-rooms-power-levels-invalid-tag-key"
        | "v-rooms-power-levels-invalid-tag"
        | "v-rooms-power-levels-invalid-tag-name"
        | "v-rooms-power-levels-invalid-tag-color"
        | "v-rooms-power-levels-invalid-icon"
        | "v-rooms-power-levels-invalid-icon-info"
        | "v-rooms-power-levels-invalid-icon-field"
        | "v-rooms-power-levels-content-too-large"
        | "v-rooms-power-levels-room-not-found" => MatrixIpcErrorCategory::SdkInvariant,
        "v-rooms-power-levels-stale-session-generation" => {
            MatrixIpcErrorCategory::StaleSessionGeneration
        }
        "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn matrix_set_room_name(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetRoomNameRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-set-room-name-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-set-room-name-no-session")
        })?;
        let result: MatrixProfileWriteResult = owner
            .set_name(&payload.room_id, &payload.name)
            .await
            .map_err(room_profile_write_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-set-room-name-serialization-failed"))
    })
}

fn matrix_set_room_topic(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetRoomTopicRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-set-room-topic-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-set-room-topic-no-session")
        })?;
        let result: MatrixProfileWriteResult = owner
            .set_topic(&payload.room_id, &payload.topic)
            .await
            .map_err(room_profile_write_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-set-room-topic-serialization-failed"))
    })
}

fn matrix_set_room_avatar(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetRoomAvatarRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-set-room-avatar-invalid-payload"))?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-set-room-avatar-no-session")
        })?;
        let result: MatrixProfileWriteResult = owner
            .set_avatar(&payload.room_id, &payload.mxc)
            .await
            .map_err(room_profile_write_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-set-room-avatar-serialization-failed"))
    })
}

fn matrix_get_room_directory_visibility(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixGetRoomDirectoryVisibilityRequest =
            serde_json::from_value(request.payload).map_err(|_| {
                core_state_error("p2-get-room-directory-visibility-invalid-payload")
            })?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-get-room-directory-visibility-no-session")
        })?;
        let result: MatrixRoomDirectoryVisibilityResult = owner
            .get_directory_visibility(&payload.room_id, payload.session_generation)
            .await
            .map_err(directory_visibility_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-get-room-directory-visibility-serialization-failed"))
    })
}

fn matrix_set_room_directory_visibility(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetRoomDirectoryVisibilityRequest =
            serde_json::from_value(request.payload).map_err(|_| {
                core_state_error("p2-set-room-directory-visibility-invalid-payload")
            })?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-set-room-directory-visibility-no-session")
        })?;
        let result: MatrixRoomDirectoryVisibilityWriteResult = owner
            .set_directory_visibility(
                &payload.room_id,
                payload.session_generation,
                &payload.visibility,
            )
            .await
            .map_err(directory_visibility_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-set-room-directory-visibility-serialization-failed"))
    })
}

fn directory_visibility_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-send.r-room-profile-directory-visibility-invalid"
        | "v-send.r-room-profile-directory-visibility-room-not-found" => {
            MatrixIpcErrorCategory::SdkInvariant
        }
        "v-send.r-room-profile-directory-visibility-requires-session" => {
            MatrixIpcErrorCategory::Forbidden
        }
        "v-send.r-room-profile-directory-visibility-stale-generation" => {
            MatrixIpcErrorCategory::StaleSessionGeneration
        }
        "v-send.r-room-profile-directory-visibility-permission-denied" => {
            MatrixIpcErrorCategory::Forbidden
        }
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn room_profile_write_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "d0.4-send-invalid-room-id"
        | "v-send.r-room-profile-name-too-long"
        | "v-send.r-room-profile-topic-too-long"
        | "v-send.r-avatar-invalid-mxc"
        | "v-send.r-room-profile-room-not-found" => MatrixIpcErrorCategory::SdkInvariant,
        "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn matrix_get_global_image_packs(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-global-image-packs-invalid-payload"));
        }
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-global-image-packs-no-session")
        })?;
        let snapshot: NativeGlobalImagePacksSnapshot = owner
            .snapshot_global()
            .await
            .map_err(image_pack_snapshot_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-global-image-packs-serialization-failed"))
    })
}

fn matrix_get_user_image_pack(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-user-image-pack-invalid-payload"));
        }
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-user-image-pack-no-session")
        })?;
        let snapshot: NativeUserImagePackSnapshot = owner
            .snapshot_user()
            .await
            .map_err(image_pack_snapshot_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-user-image-pack-serialization-failed"))
    })
}

fn matrix_get_room_image_packs(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixGetRoomImagePacksRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-image-packs-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-image-packs-no-session")
        })?;
        let snapshot: NativeRoomImagePacksSnapshot = owner
            .snapshot_room(&payload.room_id)
            .await
            .map_err(image_pack_snapshot_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-room-image-packs-serialization-failed"))
    })
}

fn matrix_set_user_image_pack(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetImagePackContentRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-set-user-image-pack-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-set-user-image-pack-no-session")
        })?;
        owner
            .set_user(payload.content)
            .await
            .map_err(image_pack_write_owner_error)?;
        Ok(serde_json::json!({"status":"ok"}))
    })
}

fn matrix_set_global_image_packs(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetImagePackContentRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-set-global-image-packs-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-set-global-image-packs-no-session")
        })?;
        owner
            .set_global(payload.content)
            .await
            .map_err(image_pack_write_owner_error)?;
        Ok(serde_json::json!({"status":"ok"}))
    })
}

fn matrix_set_room_image_pack(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetRoomImagePackRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-set-room-image-pack-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-set-room-image-pack-no-session")
        })?;
        owner
            .set_room(&payload.room_id, &payload.state_key, payload.content)
            .await
            .map_err(image_pack_write_owner_error)?;
        Ok(serde_json::json!({"status":"ok"}))
    })
}

fn matrix_later_snapshot(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-later-snapshot-invalid-payload"));
        }
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-later-snapshot-no-session")
        })?;
        let snapshot: NativeLaterSnapshot =
            owner.later_snapshot().await.map_err(later_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-later-snapshot-serialization-failed"))
    })
}

fn matrix_later_upsert(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixLaterUpsertRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-later-upsert-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-later-upsert-no-session")
        })?;
        let snapshot: NativeLaterSnapshot = owner
            .later_upsert(payload.item)
            .await
            .map_err(later_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-later-upsert-serialization-failed"))
    })
}

fn matrix_later_complete(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixLaterCompleteRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-later-complete-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-later-complete-no-session")
        })?;
        let snapshot: NativeLaterSnapshot = owner
            .later_complete(payload.item_id, payload.completed_at)
            .await
            .map_err(later_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-later-complete-serialization-failed"))
    })
}

fn matrix_later_snooze(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixLaterSnoozeRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-later-snooze-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-later-snooze-no-session")
        })?;
        let snapshot: NativeLaterSnapshot = owner
            .later_snooze(payload.item_id, payload.due_ts)
            .await
            .map_err(later_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-later-snooze-serialization-failed"))
    })
}

fn matrix_later_clear_completed(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-later-clear-completed-invalid-payload"));
        }
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-later-clear-completed-no-session")
        })?;
        let snapshot: NativeLaterSnapshot = owner
            .later_clear_completed()
            .await
            .map_err(later_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-later-clear-completed-serialization-failed"))
    })
}

fn matrix_later_mark_reminded(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixLaterMarkRemindedRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-later-mark-reminded-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-later-mark-reminded-no-session")
        })?;
        let snapshot: NativeLaterSnapshot = owner
            .later_mark_reminded(payload.item_id, payload.reminded_at)
            .await
            .map_err(later_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-later-mark-reminded-serialization-failed"))
    })
}

fn later_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-timeline-later-invalid-item" => MatrixIpcErrorCategory::SdkInvariant,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn matrix_set_own_display_name(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetOwnDisplayNameRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-set-own-display-name-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-set-own-display-name-no-session")
        })?;
        let result: MatrixProfileWriteResult = owner
            .set_own_display_name(&payload.display_name)
            .await
            .map_err(own_profile_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-set-own-display-name-serialization-failed"))
    })
}

fn matrix_set_own_avatar(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetOwnAvatarRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-set-own-avatar-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-set-own-avatar-no-session")
        })?;
        let result: MatrixProfileWriteResult = owner
            .set_own_avatar(&payload.mxc)
            .await
            .map_err(own_profile_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-set-own-avatar-serialization-failed"))
    })
}

fn own_profile_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-send.r-avatar-display-name-too-long" | "v-send.r-avatar-invalid-mxc" => {
            MatrixIpcErrorCategory::SdkInvariant
        }
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn matrix_room_notes_snapshot(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-room-notes-snapshot-invalid-payload"));
        }
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-notes-snapshot-no-session")
        })?;
        let snapshot: NativeRoomNotesSnapshot = owner
            .room_notes_snapshot()
            .await
            .map_err(room_notes_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-room-notes-snapshot-serialization-failed"))
    })
}

fn matrix_room_notes_upsert(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomNotesUpsertRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-notes-upsert-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-notes-upsert-no-session")
        })?;
        let snapshot: NativeRoomNotesSnapshot = owner
            .room_notes_upsert(payload.item)
            .await
            .map_err(room_notes_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-room-notes-upsert-serialization-failed"))
    })
}

fn matrix_room_notes_delete(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomNotesItemRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-notes-delete-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-notes-delete-no-session")
        })?;
        let snapshot: NativeRoomNotesSnapshot = owner
            .room_notes_delete(payload.room_id, payload.item_id)
            .await
            .map_err(room_notes_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-room-notes-delete-serialization-failed"))
    })
}

fn matrix_room_notes_complete_todo(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomNotesCompleteTodoRequest =
            serde_json::from_value(request.payload)
                .map_err(|_| core_state_error("p2-room-notes-complete-todo-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-notes-complete-todo-no-session")
        })?;
        let snapshot: NativeRoomNotesSnapshot = owner
            .room_notes_complete_todo(payload.room_id, payload.item_id, payload.completed)
            .await
            .map_err(room_notes_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-room-notes-complete-todo-serialization-failed"))
    })
}

fn matrix_room_notes_move_todo(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomNotesMoveTodoRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-notes-move-todo-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-notes-move-todo-no-session")
        })?;
        let snapshot: NativeRoomNotesSnapshot = owner
            .room_notes_move_todo(payload.room_id, payload.item_id, payload.direction)
            .await
            .map_err(room_notes_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-room-notes-move-todo-serialization-failed"))
    })
}

fn room_notes_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-timeline-room-notes-invalid-item" => MatrixIpcErrorCategory::SdkInvariant,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn matrix_mdirect_snapshot(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-mdirect-snapshot-invalid-payload"));
        }
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-mdirect-snapshot-no-session")
        })?;
        let snapshot: NativeMDirectSnapshot = owner
            .mdirect_snapshot()
            .await
            .map_err(mdirect_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-mdirect-snapshot-serialization-failed"))
    })
}

fn matrix_mdirect_add(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixMDirectAddRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-mdirect-add-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-mdirect-add-no-session")
        })?;
        let result: NativeMDirectMutationResult = owner
            .mdirect_add(&payload.room_id, &payload.user_id)
            .await
            .map_err(mdirect_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-mdirect-add-serialization-failed"))
    })
}

fn matrix_mdirect_remove(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixMDirectRemoveRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-mdirect-remove-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-mdirect-remove-no-session")
        })?;
        let result: NativeMDirectMutationResult = owner
            .mdirect_remove(&payload.room_id)
            .await
            .map_err(mdirect_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-mdirect-remove-serialization-failed"))
    })
}

fn mdirect_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms.5-mdirect-invalid-room" | "v-rooms.5-mdirect-invalid-user" => {
            MatrixIpcErrorCategory::SdkInvariant
        }
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn image_pack_write_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-send.r-pack-write-invalid-content"
        | "v-send.r-pack-read-invalid-room"
        | "v-send.r-pack-write-room-missing" => MatrixIpcErrorCategory::SdkInvariant,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn image_pack_snapshot_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-send.r-pack-read-invalid-room" => MatrixIpcErrorCategory::SdkInvariant,
        "v-send.r-pack-read-no-user" | "v-send.r-pack-read-subscribe-no-user" => {
            MatrixIpcErrorCategory::Forbidden
        }
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn join_rule_snapshot_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-send.r-room-profile-join-rule-invalid" => MatrixIpcErrorCategory::SdkInvariant,
        "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        "v-send.r-room-profile-join-rule-stale-generation" => {
            MatrixIpcErrorCategory::StaleSessionGeneration
        }
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn device_snapshot_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-crypto.7-device-rename-empty"
        | "v-crypto.7-device-delete-selection-empty"
        | "v-crypto.7-device-delete-selection-invalid"
        | "v-crypto.7-device-delete-not-pending"
        | "v-crypto.7-device-delete-operation-mismatch" => MatrixIpcErrorCategory::SdkInvariant,
        "v-crypto.7-device-delete-stale-generation" => {
            MatrixIpcErrorCategory::StaleSessionGeneration
        }
        "v-crypto.7-device-owner-user-missing"
        | "v-crypto.7-device-snapshot-current-missing"
        | "v-crypto.7-device-snapshot-user-missing"
        | "v-crypto.7-device-delete-current-missing"
        | "v-crypto.7-device-delete-user-missing"
        | "v-crypto.7-device-delete-auth-unsupported" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn presence_snapshot_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-presence-invalid-user-id" | "v-presence-invalid-subscription-id" => {
            MatrixIpcErrorCategory::SdkInvariant
        }
        "v-presence-user-owner-missing" | "v-presence-session-not-live" => {
            MatrixIpcErrorCategory::Forbidden
        }
        "v-presence-stale-session-generation" => MatrixIpcErrorCategory::StaleSessionGeneration,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

fn matrix_session_snapshot(state: Arc<CoreState>, _request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let response = MatrixSessionSnapshotResponse::from(state.session_snapshot()?);
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-session-snapshot-serialization-failed"))
    })
}

/// Reconstruct the public status DTO from the string-free Platform projection.
///
/// This is the only Platform-to-public mapping: Core constructs the fixed
/// `p4.1-sync-service-error` value from the closed failure enum, then validates
/// the full DTO contract before it can be serialized.
fn public_sync_status(status: PlatformSyncStatus) -> Result<SyncReadinessSnapshot, MatrixIpcError> {
    let failure_diagnostic_id = match status.failure() {
        None => None,
        Some(PlatformSyncFailure::SyncService) => Some(SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID),
    };
    let snapshot = SyncReadinessSnapshot {
        readiness: status.readiness(),
        session_generation: status.session_generation(),
        offline_mode_enabled: status.offline_mode_enabled(),
        failure_diagnostic_id,
        sliding_sync_capable: status.sliding_sync_capable(),
    };
    snapshot
        .is_valid_public_sync_status()
        .then_some(snapshot)
        .ok_or_else(|| core_state_error("p2-sync-status-invalid-platform-projection"))
}

/// `matrix_sync_status` is deliberately a payload-free observation. Core owns
/// its registry entry and exact wire serialization; the Platform remains the
/// sole owner of the live SDK client from which it reads the safe projection.
fn matrix_sync_status(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-sync-status-invalid-payload"));
        }
        let platform = state.platform();
        let status = platform
            .sync_status()
            .await
            // Platform status errors are closed enums, and Core still exposes
            // only its static command error through this public observation.
            .map_err(|_| core_state_error("p2-sync-status-platform-unavailable"))?;
        let snapshot = public_sync_status(status)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-sync-status-serialization-failed"))
    })
}

/// `matrix_crypto_status` is deliberately a payload-free observation. Core
/// owns its registry entry, validation, and exact wire serialization; the
/// Platform remains the sole owner of the live SDK crypto observation.
fn matrix_crypto_status(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-crypto-status-invalid-payload"));
        }
        let platform = state.platform();
        let status = platform
            .crypto_status()
            .await
            // A Platform crypto error is a closed enum. Never attach a shell
            // error, SDK diagnostic, identity, or key to the public command.
            .map_err(|_| core_state_error("p2-crypto-status-platform-unavailable"))?;
        let response = MatrixCryptoStatusResponse::from_platform(status)?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-crypto-status-serialization-failed"))
    })
}

/// `matrix_cross_signing_status` is a payload-free read observation. Core owns
/// its registration, exact wire DTO, and legacy truth-table reconstruction;
/// the Platform remains the sole owner of the Matrix SDK identity query and
/// its client/crypto/store/network side effects.
fn matrix_cross_signing_status(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-cross-signing-status-invalid-payload"));
        }
        let status = state
            .platform()
            .cross_signing_status()
            .await
            .map_err(cross_signing_status_transport_error)?;
        let response = MatrixCrossSigningStatusResponse::from_platform(status)?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-cross-signing-status-serialization-failed"))
    })
}

/// Convert only the closed desktop status failures into static Core errors.
/// The three legacy pairs are reconstructed here; the desktop bridge accepts
/// exactly those category/diagnostic pairs and no dynamic Core text.
fn cross_signing_status_transport_error(error: PlatformCrossSigningStatusError) -> MatrixIpcError {
    match error {
        PlatformCrossSigningStatusError::NoSession => {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("v-crypto.2-cross-signing-requires-session")
        }
        PlatformCrossSigningStatusError::UserMissing => {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("v-crypto.2-cross-signing-user-missing")
        }
        PlatformCrossSigningStatusError::IdentityQueryFailed => {
            MatrixIpcError::new(MatrixIpcErrorCategory::Unknown)
                .with_diagnostic("v-crypto.2-cross-signing-identity-query-failed")
        }
        PlatformCrossSigningStatusError::UnsafeSessionGeneration => {
            MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                .with_diagnostic("p2-cross-signing-status-unsafe-session-generation")
        }
    }
}

/// `matrix_secret_storage_status` is a payload-free read observation.
///
/// The Platform retains the sole live SDK/session/key/store ownership and
/// supplies only fixed booleans and closed enums. Core reconstructs the exact
/// legacy response and never receives a secret, identifier, raw diagnostic,
/// or SDK/account-data value.
fn matrix_secret_storage_status(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-secret-storage-status-invalid-payload"));
        }
        let status = state
            .platform()
            .secret_storage_status()
            .await
            .map_err(secret_storage_status_transport_error)?;
        let response = MatrixSecretStorageStatusResponse::from_platform(status)?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-secret-storage-status-serialization-failed"))
    })
}

/// Convert only the closed shell failures into the established public static
/// error categories and diagnostics. No shell-provided text crosses this map.
fn secret_storage_status_transport_error(
    error: PlatformSecretStorageStatusError,
) -> MatrixIpcError {
    match error {
        PlatformSecretStorageStatusError::NoSession => {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("v-crypto.4-secret-storage-requires-session")
        }
        PlatformSecretStorageStatusError::DefaultKeyLoadFailed => {
            MatrixIpcError::new(MatrixIpcErrorCategory::RecoveryFailure)
                .with_diagnostic("v-crypto.4-status-default-key-failed")
        }
        PlatformSecretStorageStatusError::KeyInfoLoadFailed => {
            MatrixIpcError::new(MatrixIpcErrorCategory::RecoveryFailure)
                .with_diagnostic("v-crypto.4-status-key-info-failed")
        }
        PlatformSecretStorageStatusError::SecretCheckFailed => {
            MatrixIpcError::new(MatrixIpcErrorCategory::RecoveryFailure)
                .with_diagnostic("v-crypto.4-status-secret-check-failed")
        }
        PlatformSecretStorageStatusError::UnsafeSessionGeneration
        | PlatformSecretStorageStatusError::InvalidSnapshot => {
            core_state_error("p2-secret-storage-status-invalid-platform-projection")
        }
    }
}

/// `matrix_media_config` has no renderer payload. Core owns the envelope and
/// exact legacy object serialization only; the Platform remains the sole owner
/// of the Matrix SDK client/session/cache/store and its cache/network load.
fn matrix_media_config(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-media-config-invalid-payload"));
        }
        let platform = state.platform();
        let config = platform
            .media_config()
            .await
            .map_err(media_config_transport_error)?;
        let response = MatrixMediaConfigResponse::from_platform(config)?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-media-config-serialization-failed"))
    })
}

/// Map the closed Platform media observation to static Core transport errors.
/// No Platform string, SDK error, URL, credential, key, or Core error object
/// enters this mapping. The desktop bridge uses only the resulting category to
/// restore its established static command diagnostics.
fn media_config_transport_error(error: PlatformMediaConfigError) -> MatrixIpcError {
    match error {
        PlatformMediaConfigError::NoSession => {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-media-config-no-session")
        }
        PlatformMediaConfigError::LoadFailed => {
            MatrixIpcError::new(MatrixIpcErrorCategory::Unknown)
                .with_diagnostic("p2-media-config-load-failed")
        }
        // The source value was outside the shared JSON-safe range. Keep this
        // distinct in Core so the bridge can retain the legacy unsafe-size
        // diagnostic instead of conflating it with an SDK load failure.
        PlatformMediaConfigError::UnsafeSize => {
            MatrixIpcError::new(MatrixIpcErrorCategory::MediaTooLarge)
                .with_diagnostic("p2-media-config-unsafe-size")
        }
    }
}

fn matrix_login_flows(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixLoginFlowsRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-login-flows-invalid-payload"))?;
        let transport =
            HttpLoginFlowTransport::new_with_user_agent(state.platform().http_user_agent())
                .map_err(auth_transport_error)?;
        let result = discover_login_flows(&payload.homeserver_url, &transport)
            .await
            .map_err(auth_transport_error)?;
        let response: MatrixLoginFlowsResponse = login_flows_response(result.flows);
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-login-flows-serialization-failed"))
    })
}

fn matrix_register_flows(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRegisterFlowsRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-register-flows-invalid-payload"))?;
        let transport =
            HttpRegisterFlowTransport::new_with_user_agent(state.platform().http_user_agent())
                .map_err(auth_transport_error)?;
        let response: RegisterFlowsProbe =
            probe_register_flows(&payload.homeserver_url, &transport)
                .await
                .map_err(auth_transport_error)?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-register-flows-serialization-failed"))
    })
}

/// Convert the credential-free auth domain's static diagnostics into the
/// versioned core transport error shape. Never attach input URLs, HTTP bodies,
/// credentials, tokens, or a raw library error.
fn auth_transport_error(error: AuthError) -> MatrixIpcError {
    let mut transport =
        MatrixIpcError::new(error.category()).with_diagnostic(error.diagnostic_id());
    if let AuthError::RateLimited {
        retry_after_ms: Some(retry_after_ms),
        ..
    } = error
    {
        transport = transport.with_retry_after_ms(retry_after_ms);
    }
    transport
}

fn core_state_error(diagnostic_id: &'static str) -> MatrixIpcError {
    MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant).with_diagnostic(diagnostic_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::sync::SyncReadiness;
    use crate::dto::{SessionLifecycle, SessionSnapshot};
    use crate::platform::{PlatformStatus, SecretVault, UnavailableSecretVault};
    use crate::transport::{CommandFuture, CommandRegistry};

    const TEST_HTTP_USER_AGENT: &str = "Synara-Core-Test/1.0";

    fn unconfigured_platform_status() -> PlatformSyncStatus {
        PlatformSyncStatus::new(SyncReadiness::Unconfigured, 0, false, None, None)
            .expect("unconfigured status is a valid string-free projection")
    }

    fn unavailable_platform_crypto_status() -> PlatformCryptoStatus {
        PlatformCryptoStatus::new(0, false, PlatformCryptoCrossSigningState::Unavailable)
            .expect("unavailable is a valid string-free crypto projection")
    }

    fn available_platform_media_config() -> PlatformMediaConfig {
        PlatformMediaConfig::new(16 * 1024 * 1024)
            .expect("a normal upload limit is a valid closed media projection")
    }

    #[derive(Default)]
    struct TestPlatform;
    impl Platform for TestPlatform {
        fn emit(
            &self,
            _envelope: crate::transport::MatrixIpcEnvelope,
        ) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync> {
            Arc::new(UnavailableSecretVault)
        }
        fn http_user_agent(&self) -> String {
            TEST_HTTP_USER_AGENT.into()
        }
        fn sync_status(&self) -> crate::platform::SyncStatusFuture<'_> {
            Box::pin(async { Ok(unconfigured_platform_status()) })
        }
        fn crypto_status(&self) -> crate::platform::CryptoStatusFuture<'_> {
            Box::pin(async { Ok(unavailable_platform_crypto_status()) })
        }
        fn cross_signing_status(&self) -> crate::platform::CrossSigningStatusFuture<'_> {
            Box::pin(async { Err(crate::platform::PlatformCrossSigningStatusError::NoSession) })
        }

        fn media_config(&self) -> crate::platform::MediaConfigFuture<'_> {
            Box::pin(async { Ok(available_platform_media_config()) })
        }
        fn notify(
            &self,
            _candidate: crate::dto::NotificationCandidate,
        ) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn set_badge(&self, _count: u64) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn status(&self, _status: PlatformStatus) -> Result<(), MatrixIpcError> {
            Ok(())
        }
    }

    /// A test shell may supply only the closed platform status/error types.
    /// It has no field in which a diagnostic string can enter Core.
    struct StatusPlatform {
        status: Result<PlatformSyncStatus, crate::platform::PlatformSyncStatusError>,
    }

    impl Platform for StatusPlatform {
        fn emit(
            &self,
            _envelope: crate::transport::MatrixIpcEnvelope,
        ) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync> {
            Arc::new(UnavailableSecretVault)
        }
        fn http_user_agent(&self) -> String {
            TEST_HTTP_USER_AGENT.into()
        }
        fn sync_status(&self) -> crate::platform::SyncStatusFuture<'_> {
            Box::pin(async move { self.status })
        }
        fn crypto_status(&self) -> crate::platform::CryptoStatusFuture<'_> {
            Box::pin(async { Ok(unavailable_platform_crypto_status()) })
        }
        fn cross_signing_status(&self) -> crate::platform::CrossSigningStatusFuture<'_> {
            Box::pin(async { Err(crate::platform::PlatformCrossSigningStatusError::NoSession) })
        }

        fn media_config(&self) -> crate::platform::MediaConfigFuture<'_> {
            Box::pin(async { Ok(available_platform_media_config()) })
        }
        fn notify(
            &self,
            _candidate: crate::dto::NotificationCandidate,
        ) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn set_badge(&self, _count: u64) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn status(&self, _status: PlatformStatus) -> Result<(), MatrixIpcError> {
            Ok(())
        }
    }

    /// A crypto test shell can supply only the closed platform projection or
    /// closed static error; neither variant can carry hostile shell text.
    struct CryptoStatusPlatform {
        status: Result<PlatformCryptoStatus, crate::platform::PlatformCryptoStatusError>,
    }

    impl Platform for CryptoStatusPlatform {
        fn emit(
            &self,
            _envelope: crate::transport::MatrixIpcEnvelope,
        ) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync> {
            Arc::new(UnavailableSecretVault)
        }
        fn http_user_agent(&self) -> String {
            TEST_HTTP_USER_AGENT.into()
        }
        fn sync_status(&self) -> crate::platform::SyncStatusFuture<'_> {
            Box::pin(async { Ok(unconfigured_platform_status()) })
        }
        fn crypto_status(&self) -> crate::platform::CryptoStatusFuture<'_> {
            Box::pin(async move { self.status })
        }
        fn cross_signing_status(&self) -> crate::platform::CrossSigningStatusFuture<'_> {
            Box::pin(async { Err(crate::platform::PlatformCrossSigningStatusError::NoSession) })
        }

        fn media_config(&self) -> crate::platform::MediaConfigFuture<'_> {
            Box::pin(async { Ok(available_platform_media_config()) })
        }
        fn notify(
            &self,
            _candidate: crate::dto::NotificationCandidate,
        ) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn set_badge(&self, _count: u64) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn status(&self, _status: PlatformStatus) -> Result<(), MatrixIpcError> {
            Ok(())
        }
    }

    /// Cross-signing tests can supply only the closed private projection or
    /// four static errors. There is no identity, user id, SDK/client/store,
    /// key, secret, or raw diagnostic field in this seam.
    struct CrossSigningStatusPlatform {
        status: Result<PlatformCrossSigningStatus, PlatformCrossSigningStatusError>,
    }

    impl Platform for CrossSigningStatusPlatform {
        fn emit(
            &self,
            _envelope: crate::transport::MatrixIpcEnvelope,
        ) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync> {
            Arc::new(UnavailableSecretVault)
        }
        fn http_user_agent(&self) -> String {
            TEST_HTTP_USER_AGENT.into()
        }
        fn sync_status(&self) -> crate::platform::SyncStatusFuture<'_> {
            Box::pin(async { Ok(unconfigured_platform_status()) })
        }
        fn crypto_status(&self) -> crate::platform::CryptoStatusFuture<'_> {
            Box::pin(async { Ok(unavailable_platform_crypto_status()) })
        }
        fn cross_signing_status(&self) -> crate::platform::CrossSigningStatusFuture<'_> {
            Box::pin(async move { self.status })
        }
        fn media_config(&self) -> crate::platform::MediaConfigFuture<'_> {
            Box::pin(async { Ok(available_platform_media_config()) })
        }
        fn notify(
            &self,
            _candidate: crate::dto::NotificationCandidate,
        ) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn set_badge(&self, _count: u64) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn status(&self, _status: PlatformStatus) -> Result<(), MatrixIpcError> {
            Ok(())
        }
    }

    /// Secret-storage tests can supply only the fixed closed projection/error.
    /// There is no field in which a secret, key, identifier, SDK value, or raw
    /// diagnostic could reach Core.
    struct SecretStorageStatusPlatform {
        status: Result<PlatformSecretStorageStatus, PlatformSecretStorageStatusError>,
    }

    impl Platform for SecretStorageStatusPlatform {
        fn emit(
            &self,
            _envelope: crate::transport::MatrixIpcEnvelope,
        ) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync> {
            Arc::new(UnavailableSecretVault)
        }
        fn http_user_agent(&self) -> String {
            TEST_HTTP_USER_AGENT.into()
        }
        fn sync_status(&self) -> crate::platform::SyncStatusFuture<'_> {
            Box::pin(async { Ok(unconfigured_platform_status()) })
        }
        fn crypto_status(&self) -> crate::platform::CryptoStatusFuture<'_> {
            Box::pin(async { Ok(unavailable_platform_crypto_status()) })
        }
        fn cross_signing_status(&self) -> crate::platform::CrossSigningStatusFuture<'_> {
            Box::pin(async { Err(crate::platform::PlatformCrossSigningStatusError::NoSession) })
        }
        fn secret_storage_status(&self) -> crate::platform::SecretStorageStatusFuture<'_> {
            Box::pin(async move { self.status })
        }
        fn media_config(&self) -> crate::platform::MediaConfigFuture<'_> {
            Box::pin(async { Ok(available_platform_media_config()) })
        }
        fn notify(
            &self,
            _candidate: crate::dto::NotificationCandidate,
        ) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn set_badge(&self, _count: u64) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn status(&self, _status: PlatformStatus) -> Result<(), MatrixIpcError> {
            Ok(())
        }
    }

    /// Media tests can supply only the bounded projection or one static error.
    /// There is no field in which an SDK/client/cache/store value or raw text
    /// could reach Core.
    struct MediaConfigPlatform {
        config: Result<PlatformMediaConfig, PlatformMediaConfigError>,
    }

    impl Platform for MediaConfigPlatform {
        fn emit(
            &self,
            _envelope: crate::transport::MatrixIpcEnvelope,
        ) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync> {
            Arc::new(UnavailableSecretVault)
        }
        fn http_user_agent(&self) -> String {
            TEST_HTTP_USER_AGENT.into()
        }
        fn sync_status(&self) -> crate::platform::SyncStatusFuture<'_> {
            Box::pin(async { Ok(unconfigured_platform_status()) })
        }
        fn crypto_status(&self) -> crate::platform::CryptoStatusFuture<'_> {
            Box::pin(async { Ok(unavailable_platform_crypto_status()) })
        }
        fn cross_signing_status(&self) -> crate::platform::CrossSigningStatusFuture<'_> {
            Box::pin(async { Err(crate::platform::PlatformCrossSigningStatusError::NoSession) })
        }

        fn media_config(&self) -> crate::platform::MediaConfigFuture<'_> {
            Box::pin(async move { self.config })
        }
        fn notify(
            &self,
            _candidate: crate::dto::NotificationCandidate,
        ) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn set_badge(&self, _count: u64) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn status(&self, _status: PlatformStatus) -> Result<(), MatrixIpcError> {
            Ok(())
        }
    }

    fn session() -> SessionSnapshot {
        SessionSnapshot {
            session_generation: 1,
            user_id: "@alice:example.org".into(),
            device_id: "DEVICE".into(),
            homeserver_url: "https://example.org".into(),
            display_name: None,
            avatar_url: None,
            lifecycle: SessionLifecycle::Ready,
            crypto_ready: true,
        }
    }

    #[test]
    fn matrix_session_snapshot_response_uses_exact_desktop_wire_keys() {
        assert_eq!(
            serde_json::to_value(MatrixSessionSnapshotResponse::from(None)).unwrap(),
            serde_json::json!({"status":"logged_out"})
        );

        let response = MatrixSessionSnapshotResponse::from(Some(SessionSnapshot {
            session_generation: 7,
            user_id: "@alice:example.org".into(),
            device_id: "DEVICE".into(),
            homeserver_url: "https://example.org".into(),
            display_name: Some("Alice".into()),
            avatar_url: Some("mxc://example.org/avatar".into()),
            lifecycle: SessionLifecycle::Ready,
            crypto_ready: true,
        }));
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            serde_json::json!({
                "status":"logged_in",
                "user_id":"@alice:example.org",
                "device_id":"DEVICE",
                "homeserver_url":"https://example.org",
                "sessionGeneration":7,
            })
        );
    }

    #[tokio::test]
    async fn default_registry_dispatches_matrix_session_snapshot() {
        let core = Core::new(Arc::new(TestPlatform));
        assert_eq!(
            core.registered_commands(),
            vec![
                "matrix_backup_status",
                "matrix_composer_clear_reply_draft",
                "matrix_composer_get_reply_draft",
                "matrix_composer_set_reply_draft",
                "matrix_cross_signing_setup",
                "matrix_cross_signing_status",
                "matrix_crypto_status",
                "matrix_device_delete_cancel",
                "matrix_device_delete_start",
                "matrix_device_rename",
                "matrix_device_snapshot",
                "matrix_edit_message",
                "matrix_get_global_image_packs",
                "matrix_get_room_directory_visibility",
                "matrix_get_room_image_packs",
                "matrix_get_user_image_pack",
                "matrix_invites_accept",
                "matrix_invites_decline",
                "matrix_invites_snapshot",
                "matrix_later_clear_completed",
                "matrix_later_complete",
                "matrix_later_mark_reminded",
                "matrix_later_snapshot",
                "matrix_later_snooze",
                "matrix_later_upsert",
                "matrix_login_flows",
                "matrix_mdirect_add",
                "matrix_mdirect_remove",
                "matrix_mdirect_snapshot",
                "matrix_media_config",
                "matrix_poll_respond",
                "matrix_presence_snapshot",
                "matrix_presence_subscribe",
                "matrix_presence_unsubscribe",
                "matrix_reaction_ensure",
                "matrix_reaction_redact",
                "matrix_register_flows",
                "matrix_restricted_join_reparent",
                "matrix_room_ban",
                "matrix_room_create",
                "matrix_room_creators_snapshot",
                "matrix_room_directory_cancel",
                "matrix_room_directory_protocols",
                "matrix_room_directory_search",
                "matrix_room_invite",
                "matrix_room_join",
                "matrix_room_join_rule_snapshot",
                "matrix_room_key_transfer_status",
                "matrix_room_kick",
                "matrix_room_leave",
                "matrix_room_list_snapshot",
                "matrix_room_members_snapshot",
                "matrix_room_notes_complete_todo",
                "matrix_room_notes_delete",
                "matrix_room_notes_move_todo",
                "matrix_room_notes_snapshot",
                "matrix_room_notes_upsert",
                "matrix_room_power_level_tags_snapshot",
                "matrix_room_power_levels_snapshot",
                "matrix_room_set_power_level",
                "matrix_room_set_power_level_tags",
                "matrix_room_set_power_levels",
                "matrix_room_unban",
                "matrix_secret_storage_status",
                "matrix_send_poll",
                "matrix_send_sticker",
                "matrix_send_text",
                "matrix_session_snapshot",
                "matrix_set_global_image_packs",
                "matrix_set_own_avatar",
                "matrix_set_own_display_name",
                "matrix_set_room_avatar",
                "matrix_set_room_directory_visibility",
                "matrix_set_room_image_pack",
                "matrix_set_room_name",
                "matrix_set_room_topic",
                "matrix_set_user_image_pack",
                "matrix_space_child_remove",
                "matrix_space_child_set",
                "matrix_space_children_snapshot",
                "matrix_space_hierarchy_snapshot",
                "matrix_space_parents_snapshot",
                "matrix_sync_status",
                "matrix_timeline_call_decline",
                "matrix_timeline_close",
                "matrix_timeline_edit_text",
                "matrix_timeline_event_readback",
                "matrix_timeline_forward_media",
                "matrix_timeline_forward_text",
                "matrix_timeline_jump_latest",
                "matrix_timeline_open",
                "matrix_timeline_paginate",
                "matrix_timeline_pin",
                "matrix_timeline_poll_vote",
                "matrix_timeline_reaction_toggle",
                "matrix_timeline_redact",
                "matrix_timeline_report",
                "matrix_timeline_set_read_state",
                "matrix_timeline_unpin",
                "matrix_typing_set",
                "matrix_typing_snapshot",
                "matrix_verification_accept",
                "matrix_verification_begin_sas",
                "matrix_verification_cancel",
                "matrix_verification_confirm",
                "matrix_verification_dismiss",
                "matrix_verification_list",
                "matrix_verification_mismatch",
                "matrix_verification_start",
            ]
        );

        let request = CommandEnvelope {
            command: "matrix_session_snapshot".into(),
            session_generation: 1,
            request_id: None,
            payload: serde_json::Value::Null,
        };
        assert_eq!(
            core.command(request.clone()).await.unwrap().payload,
            serde_json::json!({"status":"logged_out"})
        );

        core.open(session()).await.unwrap();
        assert_eq!(
            core.command(request).await.unwrap().payload,
            serde_json::json!({
                "status":"logged_in",
                "user_id":"@alice:example.org",
                "device_id":"DEVICE",
                "homeserver_url":"https://example.org",
                "sessionGeneration":1,
            })
        );
    }

    #[tokio::test]
    async fn core_sync_status_uses_exact_desktop_wire_shape() {
        let core = Core::new(Arc::new(TestPlatform));
        let request = CommandEnvelope {
            command: "matrix_sync_status".into(),
            session_generation: 0,
            request_id: Some("sync-status-fixture".into()),
            payload: serde_json::Value::Null,
        };

        let response = core
            .command(request)
            .await
            .expect("status observation succeeds");
        assert_eq!(response.command, "matrix_sync_status");
        assert_eq!(response.session_generation, 0);
        assert_eq!(response.request_id.as_deref(), Some("sync-status-fixture"));
        assert_eq!(
            response.payload,
            serde_json::json!({
                "readiness": "unconfigured",
                "sessionGeneration": 0,
                "offlineModeEnabled": false,
                "failureDiagnosticId": null,
                "slidingSyncCapable": null,
            })
        );
    }

    #[tokio::test]
    async fn core_crypto_status_uses_exact_desktop_wire_shape() {
        let ready = PlatformCryptoStatus::new(9, true, PlatformCryptoCrossSigningState::Ready)
            .expect("ready is a valid closed crypto projection");
        let response = Core::new(Arc::new(CryptoStatusPlatform { status: Ok(ready) }))
            .command(CommandEnvelope {
                command: "matrix_crypto_status".into(),
                session_generation: 0,
                request_id: Some("crypto-status-fixture".into()),
                payload: serde_json::Value::Null,
            })
            .await
            .expect("crypto status observation succeeds");

        assert_eq!(response.command, "matrix_crypto_status");
        assert_eq!(response.session_generation, 0);
        assert_eq!(
            response.request_id.as_deref(),
            Some("crypto-status-fixture")
        );
        assert_eq!(
            response.payload,
            serde_json::json!({
                "sessionGeneration": 9,
                "encryptionEnabled": true,
                "crossSigningState": "ready",
            })
        );
    }

    #[tokio::test]
    async fn core_cross_signing_status_recreates_every_closed_legacy_truth_table_row() {
        for (
            private_state,
            own_identity,
            readiness,
            publication,
            private_identity,
            own_identity_verification,
            bootstrap,
        ) in [
            (
                PlatformCrossSigningPrivateState::Unavailable,
                PlatformCrossSigningOwnIdentity::Missing,
                "unavailable",
                "missing",
                "missing",
                "missing",
                "not_needed",
            ),
            (
                PlatformCrossSigningPrivateState::Unavailable,
                PlatformCrossSigningOwnIdentity::Unverified,
                "unavailable",
                "published",
                "missing",
                "unverified",
                "not_needed",
            ),
            (
                PlatformCrossSigningPrivateState::Unavailable,
                PlatformCrossSigningOwnIdentity::Verified,
                "unavailable",
                "published",
                "missing",
                "verified",
                "not_needed",
            ),
            (
                PlatformCrossSigningPrivateState::Missing,
                PlatformCrossSigningOwnIdentity::Missing,
                "setup_required",
                "missing",
                "missing",
                "missing",
                "needed",
            ),
            (
                PlatformCrossSigningPrivateState::Missing,
                PlatformCrossSigningOwnIdentity::Unverified,
                "recovery_required",
                "published",
                "missing",
                "unverified",
                "not_needed",
            ),
            (
                PlatformCrossSigningPrivateState::Missing,
                PlatformCrossSigningOwnIdentity::Verified,
                "recovery_required",
                "published",
                "missing",
                "verified",
                "not_needed",
            ),
            (
                PlatformCrossSigningPrivateState::Partial,
                PlatformCrossSigningOwnIdentity::Missing,
                "setup_required",
                "missing",
                "partial",
                "missing",
                "needed",
            ),
            (
                PlatformCrossSigningPrivateState::Partial,
                PlatformCrossSigningOwnIdentity::Unverified,
                "recovery_required",
                "published",
                "partial",
                "unverified",
                "not_needed",
            ),
            (
                PlatformCrossSigningPrivateState::Partial,
                PlatformCrossSigningOwnIdentity::Verified,
                "recovery_required",
                "published",
                "partial",
                "verified",
                "not_needed",
            ),
            (
                PlatformCrossSigningPrivateState::Complete,
                PlatformCrossSigningOwnIdentity::Missing,
                "setup_required",
                "missing",
                "complete",
                "missing",
                "needed",
            ),
            (
                PlatformCrossSigningPrivateState::Complete,
                PlatformCrossSigningOwnIdentity::Unverified,
                "verification_required",
                "published",
                "complete",
                "unverified",
                "not_needed",
            ),
            (
                PlatformCrossSigningPrivateState::Complete,
                PlatformCrossSigningOwnIdentity::Verified,
                "ready",
                "published",
                "complete",
                "verified",
                "not_needed",
            ),
        ] {
            let status = PlatformCrossSigningStatus::new(9, private_state, own_identity)
                .expect("each closed row is representable through Platform");
            let response = Core::new(Arc::new(CrossSigningStatusPlatform { status: Ok(status) }))
                .command(CommandEnvelope {
                    command: "matrix_cross_signing_status".into(),
                    session_generation: 0,
                    request_id: Some("cross-signing-status-fixture".into()),
                    payload: serde_json::Value::Null,
                })
                .await
                .expect("closed status row serializes through Core");
            assert_eq!(response.command, "matrix_cross_signing_status");
            assert_eq!(response.session_generation, 0);
            assert_eq!(
                response.request_id.as_deref(),
                Some("cross-signing-status-fixture")
            );
            assert_eq!(
                response.payload,
                serde_json::json!({
                    "sessionGeneration": 9,
                    "readiness": readiness,
                    "masterSigning": publication,
                    "selfSigning": publication,
                    "userSigning": publication,
                    "privateIdentity": private_identity,
                    "ownIdentityVerification": own_identity_verification,
                    "bootstrap": bootstrap,
                })
            );
        }
    }

    #[tokio::test]
    async fn core_cross_signing_status_rejects_payload_and_maps_every_static_platform_error() {
        let private_text = "@alice:private.example token=secret password=secret key=secret";
        let malformed = Core::new(Arc::new(TestPlatform))
            .command(CommandEnvelope {
                command: "matrix_cross_signing_status".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({ "private": private_text }),
            })
            .await
            .expect_err("cross-signing status is a zero-argument observation");
        assert_eq!(malformed.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            malformed.diagnostic_id.as_deref(),
            Some("p2-cross-signing-status-invalid-payload")
        );

        for (status, category, diagnostic_id) in [
            (
                Err(PlatformCrossSigningStatusError::NoSession),
                MatrixIpcErrorCategory::Forbidden,
                "v-crypto.2-cross-signing-requires-session",
            ),
            (
                Err(PlatformCrossSigningStatusError::UserMissing),
                MatrixIpcErrorCategory::Forbidden,
                "v-crypto.2-cross-signing-user-missing",
            ),
            (
                Err(PlatformCrossSigningStatusError::IdentityQueryFailed),
                MatrixIpcErrorCategory::Unknown,
                "v-crypto.2-cross-signing-identity-query-failed",
            ),
            (
                Err(PlatformCrossSigningStatusError::UnsafeSessionGeneration),
                MatrixIpcErrorCategory::SdkInvariant,
                "p2-cross-signing-status-unsafe-session-generation",
            ),
        ] {
            let error = Core::new(Arc::new(CrossSigningStatusPlatform { status }))
                .command(CommandEnvelope {
                    command: "matrix_cross_signing_status".into(),
                    session_generation: 0,
                    request_id: None,
                    payload: serde_json::Value::Null,
                })
                .await
                .expect_err("closed Platform failure must become a static Core error");
            assert_eq!(error.category, category);
            assert_eq!(error.diagnostic_id.as_deref(), Some(diagnostic_id));
            let serialized = serde_json::to_string(&error).expect("static error serializes");
            for forbidden in [
                "alice",
                "private.example",
                "token",
                "secret",
                "password",
                "key",
            ] {
                assert!(
                    !serialized.contains(forbidden),
                    "cross-signing Platform/Core error must not reflect hostile text: {forbidden}"
                );
            }
        }
        assert!(!serde_json::to_string(&malformed)
            .unwrap()
            .contains(private_text));
    }

    #[tokio::test]
    async fn core_secret_storage_status_recreates_every_closed_state_and_missing_secret_case() {
        let states = [
            (
                PlatformSecretStorageState::Unavailable,
                false,
                PlatformSecretStorageAction::UnlockRequired,
                "unavailable",
                "unlock_required",
            ),
            (
                PlatformSecretStorageState::NotSetUp,
                false,
                PlatformSecretStorageAction::BootstrapRequired,
                "not_set_up",
                "bootstrap_required",
            ),
            (
                PlatformSecretStorageState::Locked,
                false,
                PlatformSecretStorageAction::UnlockRequired,
                "locked",
                "unlock_required",
            ),
            (
                PlatformSecretStorageState::Ready,
                true,
                PlatformSecretStorageAction::None,
                "ready",
                "none",
            ),
        ];
        for (state, unlocked, action, state_label, action_label) in states {
            for bits in 0_u8..16 {
                let missing = crate::platform::PlatformSecretStorageMissingSecrets::new(
                    bits & 1 != 0,
                    bits & 2 != 0,
                    bits & 4 != 0,
                    bits & 8 != 0,
                );
                let status = PlatformSecretStorageStatus::new(
                    9, state, true, unlocked, true, true, true, missing, action,
                )
                .expect("all closed legacy state/missing-secret rows are representable");
                let response =
                    Core::new(Arc::new(SecretStorageStatusPlatform { status: Ok(status) }))
                        .command(CommandEnvelope {
                            command: "matrix_secret_storage_status".into(),
                            session_generation: 0,
                            request_id: Some("secret-storage-status-fixture".into()),
                            payload: serde_json::Value::Null,
                        })
                        .await
                        .expect("closed status row serializes through Core");
                let mut missing_secrets = Vec::new();
                if bits & 1 != 0 {
                    missing_secrets.push("cross_signing_master");
                }
                if bits & 2 != 0 {
                    missing_secrets.push("cross_signing_self_signing");
                }
                if bits & 4 != 0 {
                    missing_secrets.push("cross_signing_user_signing");
                }
                if bits & 8 != 0 {
                    missing_secrets.push("encryption_backup");
                }
                assert_eq!(response.command, "matrix_secret_storage_status");
                assert_eq!(response.session_generation, 0);
                assert_eq!(
                    response.request_id.as_deref(),
                    Some("secret-storage-status-fixture")
                );
                assert_eq!(
                    response.payload,
                    serde_json::json!({
                        "sessionGeneration": 9,
                        "state": state_label,
                        "exists": true,
                        "unlocked": unlocked,
                        "defaultKeySet": true,
                        "passphraseConfigured": true,
                        "bootstrapReady": true,
                        "missingSecrets": missing_secrets,
                        "action": action_label,
                    })
                );
            }
        }
    }

    #[tokio::test]
    async fn core_secret_storage_status_rejects_payload_and_maps_every_static_platform_error() {
        let private_text = "https://private.example token=secret recovery_key=secret";
        let malformed = Core::new(Arc::new(TestPlatform))
            .command(CommandEnvelope {
                command: "matrix_secret_storage_status".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({ "private": private_text }),
            })
            .await
            .expect_err("secret-storage status is a zero-argument observation");
        assert_eq!(malformed.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            malformed.diagnostic_id.as_deref(),
            Some("p2-secret-storage-status-invalid-payload")
        );

        for (status, category, diagnostic_id) in [
            (
                Err(PlatformSecretStorageStatusError::NoSession),
                MatrixIpcErrorCategory::Forbidden,
                "v-crypto.4-secret-storage-requires-session",
            ),
            (
                Err(PlatformSecretStorageStatusError::DefaultKeyLoadFailed),
                MatrixIpcErrorCategory::RecoveryFailure,
                "v-crypto.4-status-default-key-failed",
            ),
            (
                Err(PlatformSecretStorageStatusError::KeyInfoLoadFailed),
                MatrixIpcErrorCategory::RecoveryFailure,
                "v-crypto.4-status-key-info-failed",
            ),
            (
                Err(PlatformSecretStorageStatusError::SecretCheckFailed),
                MatrixIpcErrorCategory::RecoveryFailure,
                "v-crypto.4-status-secret-check-failed",
            ),
            (
                Err(PlatformSecretStorageStatusError::UnsafeSessionGeneration),
                MatrixIpcErrorCategory::SdkInvariant,
                "p2-secret-storage-status-invalid-platform-projection",
            ),
            (
                Err(PlatformSecretStorageStatusError::InvalidSnapshot),
                MatrixIpcErrorCategory::SdkInvariant,
                "p2-secret-storage-status-invalid-platform-projection",
            ),
        ] {
            let error = Core::new(Arc::new(SecretStorageStatusPlatform { status }))
                .command(CommandEnvelope {
                    command: "matrix_secret_storage_status".into(),
                    session_generation: 0,
                    request_id: None,
                    payload: serde_json::Value::Null,
                })
                .await
                .expect_err("closed Platform failure must become a static Core error");
            assert_eq!(error.category, category);
            assert_eq!(error.diagnostic_id.as_deref(), Some(diagnostic_id));
            let serialized = serde_json::to_string(&error).expect("static error serializes");
            for forbidden in ["private.example", "token=", "recovery_key="] {
                assert!(
                    !serialized.contains(forbidden),
                    "secret-storage Platform/Core error must not reflect hostile text: {forbidden}"
                );
            }
        }
        assert!(!serde_json::to_string(&malformed)
            .unwrap()
            .contains(private_text));
    }

    #[tokio::test]
    async fn core_media_config_uses_the_exact_legacy_wire_object() {
        let response = Core::new(Arc::new(MediaConfigPlatform {
            config: Ok(PlatformMediaConfig::new(MAX_WIRE_COUNTER)
                .expect("maximum safe upload size projects through Platform")),
        }))
        .command(CommandEnvelope {
            command: "matrix_media_config".into(),
            session_generation: 0,
            request_id: Some("media-config-fixture".into()),
            payload: serde_json::Value::Null,
        })
        .await
        .expect("closed media config serializes through Core");

        assert_eq!(response.command, "matrix_media_config");
        assert_eq!(response.session_generation, 0);
        assert_eq!(response.request_id.as_deref(), Some("media-config-fixture"));
        assert_eq!(
            response.payload,
            serde_json::json!({ "m.upload.size": MAX_WIRE_COUNTER })
        );
    }

    #[tokio::test]
    async fn core_media_config_rejects_payload_and_maps_each_platform_error_statically() {
        let private_text = "https://private.example token=secret password=secret key=secret";
        let malformed = Core::new(Arc::new(TestPlatform))
            .command(CommandEnvelope {
                command: "matrix_media_config".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({ "private": private_text }),
            })
            .await
            .expect_err("media config is a zero-argument command");
        assert_eq!(malformed.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            malformed.diagnostic_id.as_deref(),
            Some("p2-media-config-invalid-payload")
        );

        for (config, category, diagnostic_id) in [
            (
                Err(PlatformMediaConfigError::NoSession),
                MatrixIpcErrorCategory::Forbidden,
                "p2-media-config-no-session",
            ),
            (
                Err(PlatformMediaConfigError::LoadFailed),
                MatrixIpcErrorCategory::Unknown,
                "p2-media-config-load-failed",
            ),
            (
                Err(PlatformMediaConfigError::UnsafeSize),
                MatrixIpcErrorCategory::MediaTooLarge,
                "p2-media-config-unsafe-size",
            ),
        ] {
            let error = Core::new(Arc::new(MediaConfigPlatform { config }))
                .command(CommandEnvelope {
                    command: "matrix_media_config".into(),
                    session_generation: 0,
                    request_id: None,
                    payload: serde_json::Value::Null,
                })
                .await
                .expect_err(
                    "closed platform media failure must reach the bridge as a static error",
                );
            assert_eq!(error.category, category);
            assert_eq!(error.diagnostic_id.as_deref(), Some(diagnostic_id));
            let serialized = serde_json::to_string(&error).expect("static error serializes");
            for forbidden in ["private.example", "token", "secret", "password", "key"] {
                assert!(
                    !serialized.contains(forbidden),
                    "media Platform/Core error must not reflect hostile text: {forbidden}"
                );
            }
        }
        assert!(!serde_json::to_string(&malformed)
            .unwrap()
            .contains(private_text));
    }

    #[tokio::test]
    async fn crypto_status_projection_is_closed_and_core_errors_are_static() {
        let private_text = "https://private.example token=secret key=secret";
        let valid = PlatformCryptoStatus::new(7, true, PlatformCryptoCrossSigningState::Partial)
            .expect("partial is a valid closed projection");
        assert!(!format!("{valid:?}").contains(private_text));

        // The Platform error contains no dynamic data, and Core replaces it
        // with a fixed command error before the public transport boundary.
        let error = Core::new(Arc::new(CryptoStatusPlatform {
            status: Err(crate::platform::PlatformCryptoStatusError::InvalidSnapshot),
        }))
        .command(CommandEnvelope {
            command: "matrix_crypto_status".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("closed Platform errors have no public crypto payload");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-crypto-status-platform-unavailable")
        );

        let malformed = Core::new(Arc::new(TestPlatform))
            .command(CommandEnvelope {
                command: "matrix_crypto_status".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({ "private": private_text }),
            })
            .await
            .expect_err("crypto status accepts no payload");
        assert_eq!(malformed.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            malformed.diagnostic_id.as_deref(),
            Some("p2-crypto-status-invalid-payload")
        );

        for public_error in [error, malformed] {
            let serialized = serde_json::to_string(&public_error).expect("static error serializes");
            for forbidden in ["private.example", "token", "secret", "key"] {
                assert!(
                    !serialized.contains(forbidden),
                    "hostile crypto text must not cross the Platform/Core seam: {forbidden}"
                );
            }
        }

        // Validate the Core-owned response contract independently of the
        // Platform constructor so an accidental future mapping cannot emit an
        // impossible encryption/state pairing.
        let invalid_response = MatrixCryptoStatusResponse {
            session_generation: 7,
            encryption_enabled: false,
            cross_signing_state: MatrixCryptoCrossSigningStateResponse::Ready,
        };
        assert!(!invalid_response.is_valid());
    }

    #[tokio::test]
    async fn core_sync_status_constructs_the_only_public_failure_diagnostic() {
        let status = PlatformSyncStatus::new(
            SyncReadiness::Failed,
            9,
            true,
            Some(PlatformSyncFailure::SyncService),
            Some(true),
        )
        .expect("closed sync failure is a valid Platform projection");
        let response = Core::new(Arc::new(StatusPlatform { status: Ok(status) }))
            .command(CommandEnvelope {
                command: "matrix_sync_status".into(),
                session_generation: 9,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect("closed Platform failure serializes through Core");

        assert_eq!(
            response.payload,
            serde_json::json!({
                "readiness": "failed",
                "sessionGeneration": 9,
                "offlineModeEnabled": true,
                "failureDiagnosticId": "p4.1-sync-service-error",
                "slidingSyncCapable": true,
            })
        );
    }

    #[tokio::test]
    async fn hostile_desktop_diagnostic_is_rejected_before_platform_core_or_public_transport() {
        let private_text: &'static str = Box::leak(
            "https://private.example token=secret password=secret"
                .to_owned()
                .into_boxed_str(),
        );
        let hostile_desktop_snapshot = SyncReadinessSnapshot {
            readiness: SyncReadiness::Failed,
            session_generation: 9,
            offline_mode_enabled: true,
            failure_diagnostic_id: Some(private_text),
            sliding_sync_capable: Some(false),
        };

        // This is the desktop-side normalization step. Its typed result has no
        // diagnostic-string field, so the hostile value cannot enter Platform.
        let normalized = PlatformSyncStatus::from_desktop_snapshot(hostile_desktop_snapshot);
        assert_eq!(
            normalized,
            Err(crate::platform::PlatformSyncStatusError::InvalidSnapshot)
        );
        assert!(!format!("{normalized:?}").contains(private_text));

        let error = Core::new(Arc::new(StatusPlatform { status: normalized }))
            .command(CommandEnvelope {
                command: "matrix_sync_status".into(),
                session_generation: 9,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect_err("rejected desktop diagnostic has no public status payload");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-sync-status-platform-unavailable")
        );
        let public_error = serde_json::to_string(&error).expect("static Core error serializes");
        for forbidden in ["private.example", "token", "secret", "password"] {
            assert!(
                !public_error.contains(forbidden),
                "hostile desktop diagnostic must not cross Platform/Core or public transport: {forbidden}"
            );
        }
    }

    #[tokio::test]
    async fn core_sync_status_fails_closed_with_static_errors() {
        let malformed = Core::new(Arc::new(TestPlatform))
            .command(CommandEnvelope {
                command: "matrix_sync_status".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"private": "token=secret"}),
            })
            .await
            .expect_err("status command must accept no payload");
        assert_eq!(malformed.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            malformed.diagnostic_id.as_deref(),
            Some("p2-sync-status-invalid-payload")
        );

        let error = Core::new(Arc::new(StatusPlatform {
            status: Err(crate::platform::PlatformSyncStatusError::Unavailable),
        }))
        .command(CommandEnvelope {
            command: "matrix_sync_status".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("opaque platform errors must not cross the Core transport");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-sync-status-platform-unavailable")
        );
    }

    fn assert_test_user_agent(request: &str) {
        let user_agent = request
            .split("\r\n")
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("user-agent")
                    .then_some(value.trim())
            })
            .expect("core auth probe must send a user-agent");
        assert_eq!(user_agent, TEST_HTTP_USER_AGENT);
    }

    async fn serve_login_flows_once(listener: &tokio::net::TcpListener) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (mut socket, _) = listener.accept().await.expect("accept login-flow request");
        let mut request = [0_u8; 2048];
        let read = socket
            .read(&mut request)
            .await
            .expect("read login-flow request");
        let request = std::str::from_utf8(&request[..read]).expect("HTTP request is text");
        assert!(
            request.starts_with("GET /_matrix/client/v3/login "),
            "handler must request only the login-types endpoint"
        );
        assert_test_user_agent(request);
        let body = r#"{"flows":[{"type":"m.login.password"},{"type":"m.login.token","get_login_token":true},{"type":"m.login.application_service"},{"type":"m.login.custom"}]}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket
            .write_all(response.as_bytes())
            .await
            .expect("write login-flow response");
    }

    #[tokio::test]
    async fn core_login_flows_uses_exact_react_payload_and_response_json() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind login-flow server");
        let address = listener.local_addr().expect("login-flow address");
        let server = tokio::spawn(async move { serve_login_flows_once(&listener).await });
        let core = Core::new(Arc::new(TestPlatform));

        let response = core
            .command(CommandEnvelope {
                command: "matrix_login_flows".into(),
                session_generation: 1,
                request_id: Some("login-flows-fixture".into()),
                payload: serde_json::json!({
                    "homeserverUrl": format!("http://{address}"),
                }),
            })
            .await
            .expect("login-flow handler succeeds");

        assert_eq!(
            response.payload,
            serde_json::json!({
                "flows": [
                    {"kind":"password","matrixType":"m.login.password"},
                    {"kind":"token","matrixType":"m.login.token","getLoginToken":true},
                    {"kind":"application_service","matrixType":"m.login.application_service"},
                    {"kind":"unknown","matrixType":"m.login.custom"},
                ]
            })
        );
        server.await.expect("login-flow server task");
    }

    #[tokio::test]
    async fn core_login_flows_rejects_malformed_missing_and_unsafe_input_privately() {
        let core = Core::new(Arc::new(TestPlatform));
        for payload in [
            serde_json::Value::Null,
            serde_json::json!({"homeserver_url":"https://not-the-react-key.invalid"}),
        ] {
            let error = core
                .command(CommandEnvelope {
                    command: "matrix_login_flows".into(),
                    session_generation: 1,
                    request_id: None,
                    payload,
                })
                .await
                .expect_err("malformed or missing payload must fail closed");
            assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
            assert_eq!(
                error.diagnostic_id.as_deref(),
                Some("p2-login-flows-invalid-payload")
            );
        }

        let unsafe_url = "https://private.example.invalid/../must-not-appear";
        let error = core
            .command(CommandEnvelope {
                command: "matrix_login_flows".into(),
                session_generation: 1,
                request_id: None,
                payload: serde_json::json!({"homeserverUrl": unsafe_url}),
            })
            .await
            .expect_err("unsafe homeserver must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p3.1-invalid-homeserver-url")
        );
        assert!(!format!("{error:?}").contains(unsafe_url));
    }

    async fn serve_register_flows_once(
        listener: &tokio::net::TcpListener,
        status: u16,
        body: &'static str,
    ) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (mut socket, _) = listener
            .accept()
            .await
            .expect("accept registration-flow request");
        let mut request = [0_u8; 4096];
        let read = socket
            .read(&mut request)
            .await
            .expect("read registration-flow request");
        let request = std::str::from_utf8(&request[..read]).expect("HTTP request is text");
        assert_test_user_agent(request);
        let (headers, request_body) = request
            .split_once("\r\n\r\n")
            .expect("registration request has headers and body");
        assert!(
            headers.starts_with("POST /_matrix/client/v3/register "),
            "handler must request only the empty registration-probe endpoint"
        );
        let headers_lower = headers.to_ascii_lowercase();
        assert!(headers_lower.contains("content-type: application/json"));
        assert_eq!(request_body, "{}", "probe must use only an empty JSON body");
        for forbidden in [
            "authorization:",
            "access_token",
            "refresh_token",
            "password",
            "registration_token",
            "client_secret",
            "captcha",
            "threepid",
            "session",
        ] {
            assert!(
                !request.to_ascii_lowercase().contains(forbidden),
                "registration probe request must not contain {forbidden}"
            );
        }

        let reason = match status {
            200 => "OK",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            429 => "Too Many Requests",
            _ => "Error",
        };
        let response = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket
            .write_all(response.as_bytes())
            .await
            .expect("write registration-flow response");
    }

    async fn core_register_flows_request(
        address: std::net::SocketAddr,
    ) -> Result<CommandResponseEnvelope, MatrixIpcError> {
        Core::new(Arc::new(TestPlatform))
            .command(CommandEnvelope {
                command: "matrix_register_flows".into(),
                session_generation: 1,
                request_id: Some("register-flows-fixture".into()),
                payload: serde_json::json!({
                    "homeserverUrl": format!("http://{address}"),
                }),
            })
            .await
    }

    #[tokio::test]
    async fn core_register_flows_uses_exact_react_wire_fixtures_and_empty_post() {
        const FLOW_REQUIRED_UIAA: &str = r#"{
            "flows":[
                {"stages":["m.login.terms","m.login.dummy"]},
                {"stages":["m.login.registration_token"]}
            ],
            "completed":["m.login.terms"],
            "params":{"m.login.terms":{"policies":[]}},
            "session":"opaque-uia-session"
        }"#;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind registration-flow server");
        let address = listener.local_addr().expect("registration-flow address");
        let server = tokio::spawn(async move {
            serve_register_flows_once(&listener, 401, FLOW_REQUIRED_UIAA).await;
        });

        let response = core_register_flows_request(address)
            .await
            .expect("registration UIAA probe succeeds");
        assert_eq!(
            response.payload,
            serde_json::json!({
                "status":"flow_required",
                "session":"opaque-uia-session",
                "flows":[
                    {"stages":["m.login.terms","m.login.dummy"]},
                    {"stages":["m.login.registration_token"]}
                ],
                "completed":["m.login.terms"],
                "params":{"m.login.terms":{"policies":[]}},
            })
        );
        assert_eq!(response.command, "matrix_register_flows");
        assert_eq!(
            response.request_id.as_deref(),
            Some("register-flows-fixture")
        );
        server.await.expect("registration-flow server task");
    }

    #[tokio::test]
    async fn core_register_flows_preserves_all_non_uia_probe_wire_variants() {
        for (status, expected) in [
            (200, serde_json::json!({"status":"invalid_request"})),
            (400, serde_json::json!({"status":"invalid_request"})),
            (403, serde_json::json!({"status":"registration_disabled"})),
            (429, serde_json::json!({"status":"rate_limited"})),
        ] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind registration-flow server");
            let address = listener.local_addr().expect("registration-flow address");
            let server = tokio::spawn(async move {
                serve_register_flows_once(&listener, status, "{").await;
            });

            let response = core_register_flows_request(address)
                .await
                .expect("known registration-probe status has a safe wire outcome");
            assert_eq!(response.payload, expected, "status {status}");
            server.await.expect("registration-flow server task");
        }
    }

    #[tokio::test]
    async fn core_register_flows_rejects_non_react_or_sensitive_payloads_privately() {
        let core = Core::new(Arc::new(TestPlatform));
        for payload in [
            serde_json::Value::Null,
            serde_json::json!({"homeserver_url":"https://not-the-react-key.invalid"}),
            serde_json::json!({
                "homeserverUrl":"https://not-the-react-key.invalid",
                "password":"must-not-cross-core",
            }),
            serde_json::json!({
                "homeserverUrl":"https://not-the-react-key.invalid",
                "session":"must-not-continue-uia",
            }),
        ] {
            let error = core
                .command(CommandEnvelope {
                    command: "matrix_register_flows".into(),
                    session_generation: 1,
                    request_id: None,
                    payload,
                })
                .await
                .expect_err("malformed or sensitive probe payload must fail closed");
            assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
            assert_eq!(
                error.diagnostic_id.as_deref(),
                Some("p2-register-flows-invalid-payload")
            );
            assert!(!format!("{error:?}").contains("must-not"));
        }

        let unsafe_url = "https://private.example.invalid/../must-not-appear";
        let error = core
            .command(CommandEnvelope {
                command: "matrix_register_flows".into(),
                session_generation: 1,
                request_id: None,
                payload: serde_json::json!({"homeserverUrl": unsafe_url}),
            })
            .await
            .expect_err("unsafe homeserver must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p3.1-invalid-homeserver-url")
        );
        assert!(!format!("{error:?}").contains(unsafe_url));
    }

    #[tokio::test]
    async fn core_register_flows_malformed_uiaa_fails_closed_without_raw_body() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind registration-flow server");
        let address = listener.local_addr().expect("registration-flow address");
        let raw_body = r#"{"flows":"not-an-array","error":"private remote body"}"#;
        let server = tokio::spawn(async move {
            serve_register_flows_once(&listener, 401, raw_body).await;
        });

        let error = core_register_flows_request(address)
            .await
            .expect_err("malformed UIAA response must fail closed");
        assert_eq!(
            error.category,
            MatrixIpcErrorCategory::UnsupportedCapability
        );
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-register-flows-uiaa-response-invalid")
        );
        assert!(!format!("{error:?}").contains("private remote body"));
        server.await.expect("registration-flow server task");
    }

    #[tokio::test]
    async fn core_open_and_close_only_manage_safe_session_projection() {
        let core = Core::new(Arc::new(TestPlatform));
        assert!(core.session_snapshot().unwrap().is_none());
        core.open(session()).await.unwrap();
        assert_eq!(core.session_snapshot().unwrap(), Some(session()));
        core.close().await.unwrap();
        assert!(core.session_snapshot().unwrap().is_none());
    }

    #[tokio::test]
    async fn command_registry_dispatches_one_typed_envelope() {
        let mut registry = CommandRegistry::new();
        registry
            .register(
                "matrix_login_flows",
                |_state: Arc<CoreState>, request: CommandEnvelope| -> CommandFuture {
                    Box::pin(async move { Ok(request.payload) })
                },
            )
            .unwrap();
        let core = Core::with_registry(Arc::new(TestPlatform), registry);
        let response = core
            .command(CommandEnvelope {
                command: "matrix_login_flows".into(),
                session_generation: 1,
                request_id: Some("r1".into()),
                payload: serde_json::json!({"safe":true}),
            })
            .await
            .unwrap();
        assert_eq!(response.payload, serde_json::json!({"safe":true}));
        assert_eq!(core.registered_commands(), vec!["matrix_login_flows"]);
    }

    #[tokio::test]
    async fn matrix_typing_snapshot_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_typing_snapshot".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect_err("typing snapshot without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-typing-snapshot-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_presence_snapshot_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_presence_snapshot".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"userId":"@alice:example.org"}),
            })
            .await
            .expect_err("presence snapshot without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-presence-snapshot-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_presence_snapshot_rejects_unknown_payload_fields() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_presence_snapshot".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"userId":"@alice:example.org","token":"no"}),
            })
            .await
            .expect_err("presence snapshot must reject unknown payload fields");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-presence-snapshot-invalid-payload")
        );
    }

    #[tokio::test]
    async fn matrix_presence_subscribe_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_presence_subscribe".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"userId":"@alice:example.org"}),
            })
            .await
            .expect_err("presence subscribe without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-presence-subscribe-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_presence_subscribe_rejects_unknown_payload_fields() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_presence_subscribe".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"userId":"@alice:example.org","token":"no"}),
            })
            .await
            .expect_err("presence subscribe must reject unknown payload fields");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-presence-subscribe-invalid-payload")
        );
    }

    #[tokio::test]
    async fn matrix_presence_unsubscribe_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_presence_unsubscribe".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"subscriptionId":"presence-1-0"}),
            })
            .await
            .expect_err("presence unsubscribe without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-presence-unsubscribe-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_presence_unsubscribe_rejects_unknown_payload_fields() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_presence_unsubscribe".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"subscriptionId":"presence-1-0","token":"no"}),
            })
            .await
            .expect_err("presence unsubscribe must reject unknown payload fields");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-presence-unsubscribe-invalid-payload")
        );
    }

    #[tokio::test]
    async fn matrix_verification_accept_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_verification_accept".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"flowId":"flow-1"}),
            })
            .await
            .expect_err("verification accept without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-verification-accept-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_verification_accept_rejects_unknown_payload_fields() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_verification_accept".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"flowId":"flow-1","token":"no"}),
            })
            .await
            .expect_err("verification accept must reject unknown payload fields");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-verification-accept-invalid-payload")
        );
    }

    #[tokio::test]
    async fn matrix_verification_begin_sas_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_verification_begin_sas".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"flowId":"flow-1"}),
            })
            .await
            .expect_err("verification begin_sas without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-verification-begin-sas-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_verification_begin_sas_rejects_unknown_payload_fields() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_verification_begin_sas".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"flowId":"flow-1","token":"no"}),
            })
            .await
            .expect_err("verification begin_sas must reject unknown payload fields");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-verification-begin-sas-invalid-payload")
        );
    }

    #[tokio::test]
    async fn matrix_verification_cancel_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_verification_cancel".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"flowId":"flow-1"}),
            })
            .await
            .expect_err("verification cancel without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-verification-cancel-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_verification_cancel_rejects_unknown_payload_fields() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_verification_cancel".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"flowId":"flow-1","token":"no"}),
            })
            .await
            .expect_err("verification cancel must reject unknown payload fields");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-verification-cancel-invalid-payload")
        );
    }

    #[tokio::test]
    async fn matrix_verification_confirm_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_verification_confirm".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"flowId":"flow-1"}),
            })
            .await
            .expect_err("verification confirm without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-verification-confirm-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_verification_confirm_rejects_unknown_payload_fields() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_verification_confirm".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"flowId":"flow-1","token":"no"}),
            })
            .await
            .expect_err("verification confirm must reject unknown payload fields");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-verification-confirm-invalid-payload")
        );
    }

    #[tokio::test]
    async fn matrix_verification_dismiss_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_verification_dismiss".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"flowId":"flow-1"}),
            })
            .await
            .expect_err("verification dismiss without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-verification-dismiss-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_verification_dismiss_rejects_unknown_payload_fields() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_verification_dismiss".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"flowId":"flow-1","token":"no"}),
            })
            .await
            .expect_err("verification dismiss must reject unknown payload fields");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-verification-dismiss-invalid-payload")
        );
    }

    #[tokio::test]
    async fn matrix_verification_mismatch_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_verification_mismatch".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"flowId":"flow-1"}),
            })
            .await
            .expect_err("verification mismatch without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-verification-mismatch-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_verification_mismatch_rejects_unknown_payload_fields() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_verification_mismatch".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"flowId":"flow-1","token":"no"}),
            })
            .await
            .expect_err("verification mismatch must reject unknown payload fields");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-verification-mismatch-invalid-payload")
        );
    }

    #[tokio::test]
    async fn matrix_verification_start_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_verification_start".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"deviceId":"DEVICE"}),
            })
            .await
            .expect_err("verification start without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-verification-start-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_verification_start_rejects_unknown_payload_fields() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_verification_start".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"deviceId":"DEVICE","token":"no"}),
            })
            .await
            .expect_err("verification start must reject unknown payload fields");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-verification-start-invalid-payload")
        );
    }

    #[tokio::test]
    async fn matrix_verification_list_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_verification_list".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect_err("verification list without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-verification-list-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_backup_status_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_backup_status".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect_err("backup status without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-backup-status-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_room_key_transfer_status_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_room_key_transfer_status".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect_err("room-key transfer status without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-room-key-transfer-status-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_cross_signing_setup_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_cross_signing_setup".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect_err("cross-signing setup without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-cross-signing-setup-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_room_list_snapshot_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_room_list_snapshot".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect_err("room-list snapshot without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-room-list-snapshot-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_invites_snapshot_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_invites_snapshot".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect_err("invites snapshot without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-invites-snapshot-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_invites_accept_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_invites_accept".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org"}),
            })
            .await
            .expect_err("invite accept without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-invites-accept-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_invites_decline_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_invites_decline".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org"}),
            })
            .await
            .expect_err("invite decline without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-invites-decline-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_room_directory_protocols_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_room_directory_protocols".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect_err("directory protocols without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-room-directory-protocols-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_room_directory_search_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_room_directory_search".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "sessionGeneration":1,
                    "requestId":1,
                    "limit":20
                }),
            })
            .await
            .expect_err("directory search without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-room-directory-search-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_room_directory_cancel_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_room_directory_cancel".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "sessionGeneration":1,
                    "requestId":1
                }),
            })
            .await
            .expect_err("directory cancel without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-room-directory-cancel-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_device_snapshot_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_device_snapshot".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect_err("device snapshot without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-device-snapshot-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_device_rename_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_device_rename".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"deviceId":"DEVICE","displayName":"laptop"}),
            })
            .await
            .expect_err("device rename without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-device-rename-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_device_rename_rejects_unknown_payload_fields() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_device_rename".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "deviceId":"DEVICE",
                    "displayName":"laptop",
                    "token":"no"
                }),
            })
            .await
            .expect_err("device rename must reject unknown payload fields");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-device-rename-invalid-payload")
        );
    }

    #[tokio::test]
    async fn matrix_device_delete_start_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_device_delete_start".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"deviceIds":["OTHER"]}),
            })
            .await
            .expect_err("device delete start without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-device-delete-start-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_device_delete_start_rejects_unknown_payload_fields() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_device_delete_start".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"deviceIds":["OTHER"],"token":"no"}),
            })
            .await
            .expect_err("device delete start must reject unknown payload fields");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-device-delete-start-invalid-payload")
        );
    }

    #[tokio::test]
    async fn matrix_device_delete_cancel_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_device_delete_cancel".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"operationId":1,"sessionGeneration":1}),
            })
            .await
            .expect_err("device delete cancel without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-device-delete-cancel-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_device_delete_cancel_rejects_unknown_payload_fields() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_device_delete_cancel".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "operationId":1,
                    "sessionGeneration":1,
                    "token":"no"
                }),
            })
            .await
            .expect_err("device delete cancel must reject unknown payload fields");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-device-delete-cancel-invalid-payload")
        );
    }

    #[tokio::test]
    async fn matrix_set_room_name_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_set_room_name".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org","name":"Room"}),
            })
            .await
            .expect_err("set room name without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-set-room-name-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_set_room_topic_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_set_room_topic".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org","topic":"Hello"}),
            })
            .await
            .expect_err("set room topic without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-set-room-topic-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_set_room_avatar_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_set_room_avatar".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org","mxc":""}),
            })
            .await
            .expect_err("set room avatar without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-set-room-avatar-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_set_own_display_name_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_set_own_display_name".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"displayName":"Alice"}),
            })
            .await
            .expect_err("set own display name without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-set-own-display-name-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_set_own_avatar_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_set_own_avatar".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"mxc":"mxc://example.org/abc"}),
            })
            .await
            .expect_err("set own avatar without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-set-own-avatar-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_get_room_directory_visibility_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_get_room_directory_visibility".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org","sessionGeneration":1}),
            })
            .await
            .expect_err("directory visibility get without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-get-room-directory-visibility-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_set_room_directory_visibility_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_set_room_directory_visibility".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "sessionGeneration":1,
                    "visibility":"public"
                }),
            })
            .await
            .expect_err("directory visibility set without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-set-room-directory-visibility-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_later_snapshot_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_later_snapshot".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect_err("later snapshot without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-later-snapshot-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_later_upsert_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_later_upsert".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "item": {
                        "id": "i",
                        "kind": "saved",
                        "roomId": "!r:example.org",
                        "eventId": "$e",
                        "createdAt": 1.0
                    }
                }),
            })
            .await
            .expect_err("later upsert without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-later-upsert-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_room_notes_snapshot_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_room_notes_snapshot".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect_err("room notes snapshot without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-room-notes-snapshot-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_mdirect_snapshot_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_mdirect_snapshot".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect_err("mdirect snapshot without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-mdirect-snapshot-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_mdirect_add_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_mdirect_add".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "userId":"@alice:example.org"
                }),
            })
            .await
            .expect_err("mdirect add without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-mdirect-add-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_mdirect_remove_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_mdirect_remove".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org"}),
            })
            .await
            .expect_err("mdirect remove without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-mdirect-remove-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_room_join_rule_snapshot_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_room_join_rule_snapshot".into(),
                session_generation: 1,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org","sessionGeneration":1}),
            })
            .await
            .expect_err("join-rule snapshot without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-join-rule-snapshot-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_room_leave_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_room_leave".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org"}),
            })
            .await
            .expect_err("room leave without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-room-leave-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_room_join_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_room_join".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomIdOrAlias":"!r:example.org"}),
            })
            .await
            .expect_err("room join without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-room-join-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_room_invite_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_room_invite".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "userId":"@alice:example.org"
                }),
            })
            .await
            .expect_err("room invite without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-room-invite-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_room_kick_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_room_kick".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "userId":"@alice:example.org"
                }),
            })
            .await
            .expect_err("room kick without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-room-kick-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_room_ban_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_room_ban".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "userId":"@alice:example.org"
                }),
            })
            .await
            .expect_err("room ban without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-room-ban-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_room_unban_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_room_unban".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "userId":"@alice:example.org"
                }),
            })
            .await
            .expect_err("room unban without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-room-unban-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_room_set_power_level_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_room_set_power_level".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "userId":"@alice:example.org",
                    "powerLevel":50
                }),
            })
            .await
            .expect_err("set power level without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-room-set-power-level-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_room_set_power_levels_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_room_set_power_levels".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "content":{"users":{}}
                }),
            })
            .await
            .expect_err("set power levels without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-room-set-power-levels-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_room_set_power_level_tags_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_room_set_power_level_tags".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "content":{}
                }),
            })
            .await
            .expect_err("set power-level tags without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-room-set-power-level-tags-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_room_create_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_room_create".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "name":"Room",
                    "encryption":false,
                    "isDirect":false,
                    "invite":[],
                    "knock":false
                }),
            })
            .await
            .expect_err("room create without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-room-create-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_room_members_snapshot_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_room_members_snapshot".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org"}),
            })
            .await
            .expect_err("members snapshot without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-room-members-snapshot-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_room_power_levels_snapshot_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_room_power_levels_snapshot".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org"}),
            })
            .await
            .expect_err("power-levels snapshot without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-room-power-levels-snapshot-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_room_creators_snapshot_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_room_creators_snapshot".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org"}),
            })
            .await
            .expect_err("creators snapshot without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-room-creators-snapshot-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_room_power_level_tags_snapshot_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_room_power_level_tags_snapshot".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org"}),
            })
            .await
            .expect_err("power-level tags snapshot without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-room-power-level-tags-snapshot-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_room_join_rule_snapshot_rejects_unknown_payload_fields() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_room_join_rule_snapshot".into(),
                session_generation: 1,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "sessionGeneration":1,
                    "token":"no"
                }),
            })
            .await
            .expect_err("join-rule snapshot must reject unknown payload fields");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-join-rule-snapshot-invalid-payload")
        );
    }

    #[tokio::test]
    async fn matrix_get_global_image_packs_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_get_global_image_packs".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect_err("global image packs without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-global-image-packs-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_get_user_image_pack_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_get_user_image_pack".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect_err("user image pack without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-user-image-pack-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_get_room_image_packs_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_get_room_image_packs".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org"}),
            })
            .await
            .expect_err("room image packs without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-room-image-packs-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_set_user_image_pack_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_set_user_image_pack".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"content":{}}),
            })
            .await
            .expect_err("set user image pack without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-set-user-image-pack-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_typing_set_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_typing_set".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org","typing":true}),
            })
            .await
            .expect_err("typing set without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-typing-set-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_close_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_close".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"streamId":"view-1"}),
            })
            .await
            .expect_err("timeline close without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-close-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_close_rejects_unknown_payload_fields() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_close".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"streamId":"view-1","token":"no"}),
            })
            .await
            .expect_err("timeline close must reject unknown payload fields");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-close-invalid-payload")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_open_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_open".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org","position":{"kind":"live_bottom"}}),
            })
            .await
            .expect_err("timeline open without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-open-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_open_rejects_unknown_payload_fields() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_open".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "position":{"kind":"live_bottom"},
                    "token":"no"
                }),
            })
            .await
            .expect_err("timeline open must reject unknown payload fields");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-open-invalid-payload")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_jump_latest_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_jump_latest".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"streamId":"view-1"}),
            })
            .await
            .expect_err("timeline jump_latest without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-jump-latest-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_event_readback_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_event_readback".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org","eventId":"$e"}),
            })
            .await
            .expect_err("timeline event readback without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-event-readback-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_event_readback_rejects_unknown_payload_fields() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_event_readback".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "eventId":"$e",
                    "token":"no"
                }),
            })
            .await
            .expect_err("timeline event readback must reject unknown payload fields");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-event-readback-invalid-payload")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_paginate_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_paginate".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"streamId":"view-1","direction":"backwards"}),
            })
            .await
            .expect_err("timeline paginate without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-paginate-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_paginate_rejects_unknown_payload_fields() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_paginate".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "streamId":"view-1",
                    "direction":"backwards",
                    "token":"no"
                }),
            })
            .await
            .expect_err("timeline paginate must reject unknown payload fields");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-paginate-invalid-payload")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_set_read_state_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_set_read_state".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"streamId":"view-1","action":"mark_read"}),
            })
            .await
            .expect_err("timeline set_read_state without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-set-read-state-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_set_read_state_rejects_unknown_payload_fields() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_set_read_state".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "streamId":"view-1",
                    "action":"mark_read",
                    "token":"no"
                }),
            })
            .await
            .expect_err("timeline set_read_state must reject unknown payload fields");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-set-read-state-invalid-payload")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_reaction_toggle_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_reaction_toggle".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org","eventId":"$e","key":"✅"}),
            })
            .await
            .expect_err("reaction toggle without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-reaction-toggle-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_reaction_toggle_rejects_unknown_payload_fields() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_reaction_toggle".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "eventId":"$e",
                    "key":"✅",
                    "token":"no"
                }),
            })
            .await
            .expect_err("reaction toggle must reject unknown payload fields");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-reaction-toggle-invalid-payload")
        );
    }

    #[tokio::test]
    async fn matrix_reaction_ensure_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_reaction_ensure".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org","eventId":"$e","key":"✅"}),
            })
            .await
            .expect_err("reaction ensure without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-reaction-ensure-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_reaction_redact_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_reaction_redact".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "targetEventId":"$e",
                    "reactionEventId":"$r",
                    "key":"✅"
                }),
            })
            .await
            .expect_err("reaction redact without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-reaction-redact-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_send_text_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_send_text".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "body":"hello"
                }),
            })
            .await
            .expect_err("send text without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-send-text-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_send_sticker_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_send_sticker".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "body":"sticker",
                    "mxc":"mxc://example.org/s"
                }),
            })
            .await
            .expect_err("send sticker without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-send-sticker-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_send_poll_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_send_poll".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "question":"Q?",
                    "answers":["A","B"],
                    "maxSelections":1
                }),
            })
            .await
            .expect_err("send poll without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-send-poll-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_poll_respond_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_poll_respond".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "pollEventId":"$e:example.org",
                    "answerIds":["a1"]
                }),
            })
            .await
            .expect_err("poll respond without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-poll-respond-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_space_parents_snapshot_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_space_parents_snapshot".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect_err("space parents snapshot without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-space-parents-snapshot-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_space_hierarchy_snapshot_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_space_hierarchy_snapshot".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!space:example.org"}),
            })
            .await
            .expect_err("space hierarchy snapshot without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-space-hierarchy-snapshot-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_space_children_snapshot_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_space_children_snapshot".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect_err("space children snapshot without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-space-children-snapshot-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_space_child_set_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_space_child_set".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "parentId":"!space:example.org",
                    "childId":"!room:example.org",
                    "via":["example.org"]
                }),
            })
            .await
            .expect_err("space child set without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-space-child-set-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_space_child_remove_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_space_child_remove".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "parentId":"!space:example.org",
                    "childId":"!room:example.org"
                }),
            })
            .await
            .expect_err("space child remove without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-space-child-remove-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_restricted_join_reparent_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_restricted_join_reparent".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!room:example.org",
                    "addParentId":"!space:example.org"
                }),
            })
            .await
            .expect_err("restricted join reparent without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-restricted-join-reparent-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_edit_message_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_edit_message".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "eventId":"$e:example.org",
                    "body":"hello"
                }),
            })
            .await
            .expect_err("edit message without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-edit-message-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_edit_text_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_edit_text".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "eventId":"$e",
                    "body":"updated"
                }),
            })
            .await
            .expect_err("timeline edit without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-edit-text-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_edit_text_rejects_unknown_payload_fields() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_edit_text".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "eventId":"$e",
                    "body":"updated",
                    "token":"no"
                }),
            })
            .await
            .expect_err("timeline edit must reject unknown payload fields");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-edit-text-invalid-payload")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_redact_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_redact".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "eventId":"$e"
                }),
            })
            .await
            .expect_err("timeline redact without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-redact-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_report_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_report".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "eventId":"$e"
                }),
            })
            .await
            .expect_err("timeline report without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-report-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_pin_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_pin".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "eventId":"$e"
                }),
            })
            .await
            .expect_err("timeline pin without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-pin-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_unpin_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_unpin".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "eventId":"$e"
                }),
            })
            .await
            .expect_err("timeline unpin without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-unpin-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_poll_vote_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_poll_vote".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "eventId":"$e",
                    "answerIds":["yes"]
                }),
            })
            .await
            .expect_err("timeline poll vote without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-poll-vote-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_call_decline_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_call_decline".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "eventId":"$e"
                }),
            })
            .await
            .expect_err("timeline call decline without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-call-decline-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_forward_text_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_forward_text".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "sourceRoomId":"!s:example.org",
                    "eventId":"$e",
                    "targetRoomId":"!t:example.org"
                }),
            })
            .await
            .expect_err("timeline forward text without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-forward-text-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_timeline_forward_media_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_forward_media".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "sourceRoomId":"!s:example.org",
                    "eventId":"$e",
                    "targetRoomId":"!t:example.org"
                }),
            })
            .await
            .expect_err("timeline forward media without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-timeline-forward-media-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_composer_set_reply_draft_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_composer_set_reply_draft".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({
                    "roomId":"!r:example.org",
                    "eventId":"$e"
                }),
            })
            .await
            .expect_err("composer set reply draft without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-composer-set-reply-draft-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_composer_clear_reply_draft_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_composer_clear_reply_draft".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org"}),
            })
            .await
            .expect_err("composer clear reply draft without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-composer-clear-reply-draft-no-session")
        );
    }

    #[tokio::test]
    async fn matrix_composer_get_reply_draft_without_owner_fails_closed() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_composer_get_reply_draft".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org"}),
            })
            .await
            .expect_err("composer get reply draft without an attached owner must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-composer-get-reply-draft-no-session")
        );
    }

    #[tokio::test]
    async fn known_but_unregistered_commands_fail_closed_with_static_diagnostic() {
        let core = Core::new(Arc::new(TestPlatform));
        for command in [
            "matrix_login_password",
            "matrix_register",
            "matrix_register_request_email_token",
        ] {
            let error = core
                .command(CommandEnvelope {
                    command: command.into(),
                    session_generation: 1,
                    request_id: None,
                    payload: serde_json::Value::Null,
                })
                .await
                .expect_err("known but unregistered command must fail closed");
            assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
            assert_eq!(
                error.diagnostic_id.as_deref(),
                Some("p2-command-unregistered")
            );
        }
    }
}
