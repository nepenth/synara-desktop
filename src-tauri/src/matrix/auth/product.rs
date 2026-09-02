//! D0.1–D0.3 product password-login, native session, sync, and timeline ownership.
//!
//! This is the only desktop product boundary for password login. The live
//! `matrix_sdk::Client` and all access/refresh tokens remain in the Rust host.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use matrix_sdk::{
    authentication::matrix::MatrixSession,
    media::{MediaFormat, MediaRequestParameters},
    ruma::{
        api::client::{
            room::{create_room, Visibility},
            uiaa,
        },
        events::{
            relation::{Reply, Thread},
            room::{
                message::{
                    AddMentions, MessageFormat, MessageType, Relation, RelationWithoutReplacement,
                    ReplyWithinThread, RoomMessageEventContent,
                },
                ImageInfo, MediaSource,
            },
            AnyMessageLikeEventContent, AnySyncMessageLikeEvent, AnySyncTimelineEvent, Mentions,
            StateEventType,
        },
        EventId, Int, MxcUri, OwnedEventId, OwnedMxcUri, OwnedRoomId, OwnedRoomOrAliasId,
        OwnedServerName, OwnedTransactionId, OwnedUserId, UInt,
    },
    Client, Room, SessionMeta, SessionTokens,
};
use mime::Mime;
use serde::{Deserialize, Serialize};
use synara_core::platform::{
    PlatformCrossSigningOwnIdentity, PlatformCrossSigningPrivateState, PlatformCrossSigningStatus,
    PlatformCryptoCrossSigningState, PlatformCryptoStatus, PlatformMediaConfig,
    PlatformMediaConfigError, PlatformSecretStorageAction, PlatformSecretStorageMissingSecrets,
    PlatformSecretStorageState, PlatformSecretStorageStatus, PlatformSecretStorageStatusError,
};
use tauri::{AppHandle, Manager, State};
use tokio::sync::Mutex;
use zeroize::Zeroize;

use super::{
    complete_password_reset, existing_sqlite_crypto_device_id, login_with_password,
    normalize_homeserver_url, password_reset_ephemeral_user_id, register_ephemeral_user_id,
    register_submit, request_password_email_token, request_register_email_token, AuthError,
    LoginFlow, LoginOptions, PasswordEmailTokenResult, PasswordResetOutcome, RegisterAuthStage,
    RegisterFlowsProbe, RegisterSubmitOutcome, RegisterUiaFlow,
};
use crate::matrix::account_data::{
    add_room_to_mdirect, clear_completed_later_live, complete_later_item_live,
    complete_room_todo_item_live, delete_room_note_item_live, mark_later_reminded_live,
    move_room_todo_item_live, remove_room_from_mdirect, set_global_image_packs,
    set_room_image_pack, set_user_image_pack, snapshot_global_image_packs, snapshot_later,
    snapshot_mdirect, snapshot_room_image_packs, snapshot_room_notes, snapshot_user_image_pack,
    snooze_later_item_live, upsert_later_item, upsert_room_note_item,
    NativeGlobalImagePacksSnapshot, NativeImagePackOwner, NativeLaterSnapshot,
    NativeMDirectMutationResult, NativeMDirectSnapshot, NativeRoomImagePacksSnapshot,
    NativeRoomNotesSnapshot, NativeUserImagePackSnapshot, RoomNoteMoveDirection, SynaraLaterItem,
    SynaraRoomNoteItem,
};
use crate::matrix::backup::live::{
    self as live_backup, NativeBackupOperationResult, NativeBackupStatus,
};
use crate::matrix::client_builder::{
    build_unauthenticated_client, ClientBuildConfig, ClientBuilderError,
};
use crate::matrix::cross_signing::live::{NativeCrossSigningSetupResult, NativeCrossSigningStatus};
use crate::matrix::devices::{NativeDeviceDeleteResult, NativeDeviceOwner, NativeDeviceSnapshot};
use crate::matrix::lifecycle::{
    clear_session_material, load_session_material, matrix_session_from_host_secrets,
    persist_session_after_login, restore_session_from_vault, restore_session_onto_client,
    KeyringSessionMaterialVault, SessionMaterial,
};
use crate::matrix::presence::NativePresenceOwner;
use crate::matrix::room_keys::{
    live::{
        self as live_room_keys, NativeRoomKeyFileSelection, NativeRoomKeyTransferResult,
        NativeRoomKeyTransferStatus, SelectedRoomKeyImport,
    },
    RoomKeyTransferFlow,
};
use crate::matrix::room_list::{
    snapshot_invites, InviteAvatarHandles, NativeInvite, NativeInviteSnapshot,
    NativeRoomListSnapshot,
};
use crate::matrix::room_profile::NativeRoomJoinRuleOwner;
use crate::matrix::secret_storage::live::{
    self as live_secret_storage, NativeMissingSecret, NativeSecretStorageAction,
    NativeSecretStorageOperationResult, NativeSecretStorageState, NativeSecretStorageStatus,
};
use crate::matrix::send::{
    normalize_poll, poll_response_content, poll_start_content, AttachmentEnqueue, AttachmentKind,
    AttachmentSendQueue, SendQueue,
};
use crate::matrix::spaces::{
    NativeRestrictedJoinReparentResult, NativeSpaceChildMutationResult,
    NativeSpaceChildrenSnapshot, NativeSpaceHierarchySnapshot, NativeSpaceParentsSnapshot,
};
use crate::matrix::store::{
    get_or_migrate_store_key, migrate_store_to_current, reset_store_for_recovery, AccountIdentity,
    KeyringStoreKeyVault, StoreKeyMaterial, StoreKeyVaultError, StoreMigrationError, StorePaths,
};
use crate::matrix::sync::{
    build_sync_service, unconfigured_snapshot, SyncReadinessSnapshot, SyncServiceConfig,
    SyncServiceOwner,
};
use crate::matrix::timeline::{
    format_forwarded_media_body, format_forwarded_plain_body, should_attach_formatted_body,
    NativeComposerClearReplyDraftRequest, NativeComposerReplyDraftReadback,
    NativeComposerReplyDraftRoomRequest, NativeComposerSetReplyDraftRequest,
    NativeReactionMutationResult, NativeTimelineActionKind, NativeTimelineActionReadback,
    NativeTimelineCallDeclineRequest, NativeTimelineCloseRequest, NativeTimelineDirection,
    NativeTimelineEditTextRequest, NativeTimelineEventReadback, NativeTimelineForwardMediaRequest,
    NativeTimelineForwardTextRequest, NativeTimelineJumpLatestRequest, NativeTimelineOpenReadback,
    NativeTimelineOpenRequest, NativeTimelineOwner, NativeTimelinePinRequest,
    NativeTimelinePollVoteRequest, NativeTimelineReadAction, NativeTimelineReadIntent,
    NativeTimelineReadStateReadback, NativeTimelineReadStateRequest, NativeTimelineRedactRequest,
    NativeTimelineReportRequest, NativeTimelineSnapshot, NativeTimelineViewPaginationRequest,
    TimelineMediaSource, NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
};
use crate::matrix::typing::{set_typing_notice, NativeTypingOwner, NativeTypingSnapshot};
use crate::matrix::verification::live::{
    NativeVerificationInbox, NativeVerificationOwner, NativeVerificationRequest,
};

const ACTIVE_SESSION_FILE: &str = "active-session.json";
const MATRIX_DATA_DIR: &str = "matrix";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixLoginIdentity {
    pub user_id: String,
    pub device_id: String,
    pub homeserver_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum MatrixSessionSnapshot {
    LoggedOut,
    LoggedIn {
        user_id: String,
        device_id: String,
        homeserver_url: String,
        #[serde(rename = "sessionGeneration")]
        session_generation: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatrixCrossSigningState {
    Unavailable,
    NotSetUp,
    Partial,
    Ready,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixCryptoStatus {
    pub session_generation: u64,
    pub encryption_enabled: bool,
    pub cross_signing_state: MatrixCrossSigningState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixAuthCommandError {
    pub code: &'static str,
    pub message: &'static str,
    pub diagnostic_id: String,
}

pub use synara_core::app::media::{
    MatrixMediaConfigResult, MatrixMediaDownloadRequest, MatrixMediaDownloadResult,
    MatrixUploadMediaResult,
};
pub use synara_core::app::members::NativeRoomMembersSnapshot;
pub use synara_core::app::send::{
    MatrixPollRespondResult, MatrixSendAttachmentResult, MatrixSendPollResult, MatrixSendTextResult,
};
pub use synara_core::app::user_profile::MatrixProfileWriteResult;

pub use synara_core::app::members::{
    NativePowerLevelWriteResult, MAX_POWER_LEVEL_CONTENT_JSON_BYTES,
};
pub use synara_core::app::room_ops::{
    MatrixRoomCreateContent, MatrixRoomCreatePowerLevels, MatrixRoomCreatePreset,
    MatrixRoomCreateRequest, MatrixRoomCreateVisibility,
};

/// Soft IPC/body cap for one-shot composer attachment transfer (bytes).
const MAX_ATTACHMENT_IPC_BYTES: usize = 32 * 1024 * 1024;

/// Maximum bounded identifier accepted by the direct CallWidget media path.
const MAX_CALL_WIDGET_MEDIA_URI_BYTES: usize = 2 * 1024;

/// V-SEND.R-CALL-MEDIA response ceiling. Never truncate over-limit content.
const MAX_CALL_WIDGET_MEDIA_DOWNLOAD_BYTES: usize = MAX_ATTACHMENT_IPC_BYTES;

/// Soft IPC/body cap for one-shot user-avatar media transfer (bytes).
/// Avatars are small images; 8 MiB is generous and keeps the webview buffer
/// bounded (well under the 32 MiB attachment cap).
const MAX_AVATAR_IPC_BYTES: usize = 8 * 1024 * 1024;

impl MatrixAuthCommandError {
    pub(crate) fn new(
        code: &'static str,
        message: &'static str,
        diagnostic_id: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message,
            diagnostic_id: diagnostic_id.into(),
        }
    }

    fn invalid_input(diagnostic_id: &'static str) -> Self {
        Self::new(
            "InvalidRequest",
            "The native Matrix login request is invalid.",
            diagnostic_id,
        )
    }

    fn unavailable(diagnostic_id: &'static str) -> Self {
        Self::new(
            "Unknown",
            "Native Matrix session storage is unavailable.",
            diagnostic_id,
        )
    }
}

struct ManagedMatrixSession {
    client: Client,
    identity: MatrixLoginIdentity,
    sync: Arc<SyncServiceOwner>,
    invite_avatars: Arc<tokio::sync::Mutex<InviteAvatarHandles>>,
    timelines: Arc<NativeTimelineOwner>,
    sends: SendQueue,
    attachments: AttachmentSendQueue,
    verification: Arc<NativeVerificationOwner>,
    devices: Arc<NativeDeviceOwner>,
    _image_packs: Arc<NativeImagePackOwner>,
    typing: Arc<NativeTypingOwner>,
    presence: Arc<NativePresenceOwner>,
    join_rules: Arc<NativeRoomJoinRuleOwner>,
    room_key_transfer: Arc<Mutex<RoomKeyTransferFlow>>,
    selected_room_key_import: Option<SelectedRoomKeyImport>,
    next_room_key_import_selection_id: u64,
}

/// A locally held recovery target is armed only by a failed native login.
/// It never crosses IPC; the renderer receives only an opaque, one-use
/// confirmation capability after the user opens the recovery confirmation.
#[derive(Default)]
enum StoreRecoveryState {
    #[default]
    Idle,
    Pending {
        identity: AccountIdentity,
    },
    AwaitingConfirmation {
        identity: AccountIdentity,
        confirmation_id: String,
    },
}

#[derive(Default)]
pub struct MatrixAuthState {
    session: Mutex<Option<ManagedMatrixSession>>,
    store_recovery: Mutex<StoreRecoveryState>,
    next_session_generation: AtomicU64,
}

impl MatrixAuthState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read the current SDK sync owner as the existing safe readiness DTO.
    ///
    /// This remains desktop-owned: no client, credential, store handle, or raw
    /// SDK diagnostic leaves `MatrixAuthState`. The desktop `Platform` adapter
    /// normalizes this legacy DTO into its string-free Core projection locally.
    pub(crate) async fn sync_status_snapshot(&self) -> SyncReadinessSnapshot {
        let session = self.session.lock().await;
        match session.as_ref() {
            Some(active) => active.sync.observe(),
            None => unconfigured_snapshot(self.current_generation()),
        }
    }

    /// Read the existing crypto-status observation as a closed Core projection.
    ///
    /// This intentionally keeps the auth mutex held while the live SDK crypto
    /// owner is sampled, matching the pre-Core command behavior. The desktop
    /// remains the sole Client/crypto/store owner: only a generation, boolean,
    /// and fixed coarse state leave this method.
    pub(crate) async fn crypto_status_projection(&self) -> PlatformCryptoStatus {
        let session = self.session.lock().await;
        let Some(active) = session.as_ref() else {
            return PlatformCryptoStatus::new(
                self.current_generation(),
                false,
                PlatformCryptoCrossSigningState::Unavailable,
            )
            .expect("unavailable is a valid closed crypto projection");
        };

        let cross_signing = active.client.encryption().cross_signing_status().await;
        let (encryption_enabled, cross_signing_state) = match cross_signing.as_ref() {
            None => (false, PlatformCryptoCrossSigningState::Unavailable),
            Some(status) => (
                true,
                crypto_cross_signing_state(
                    status.is_complete(),
                    status.has_master,
                    status.has_self_signing,
                    status.has_user_signing,
                ),
            ),
        };
        PlatformCryptoStatus::new(
            active.sync.session_generation(),
            encryption_enabled,
            cross_signing_state,
        )
        .expect("desktop crypto observation must map to a valid closed projection")
    }

    /// Read the exact legacy cross-signing observation as a closed Core projection.
    ///
    /// Clone the live SDK client under the auth mutex, then release that mutex
    /// before identity lookup. Holding the mutex across `request_user_identity`
    /// (`/keys/query`) stalled Settings → Devices on a spinner because sync
    /// could not progress. Prefer the local crypto-store identity so the page
    /// can render without a network round trip; bound the homeserver fetch.
    pub(crate) async fn cross_signing_status_projection(
        &self,
    ) -> Result<PlatformCrossSigningStatus, synara_core::platform::PlatformCrossSigningStatusError>
    {
        let (client, session_generation) = {
            let session = self.session.lock().await;
            let active = session
                .as_ref()
                .ok_or(synara_core::platform::PlatformCrossSigningStatusError::NoSession)?;
            (active.client.clone(), active.sync.session_generation())
        };

        let encryption = client.encryption();
        let private_status = encryption.cross_signing_status().await;
        let Some(user_id) = client.user_id() else {
            return Err(synara_core::platform::PlatformCrossSigningStatusError::UserMissing);
        };
        let local_identity = encryption.get_user_identity(user_id).await.map_err(|_| {
            synara_core::platform::PlatformCrossSigningStatusError::IdentityQueryFailed
        })?;
        let own_identity = match local_identity {
            Some(identity) => Some(identity),
            None => match tokio::time::timeout(
                Duration::from_secs(8),
                encryption.request_user_identity(user_id),
            )
            .await
            {
                Ok(Ok(identity)) => identity,
                Ok(Err(_)) => {
                    return Err(
                        synara_core::platform::PlatformCrossSigningStatusError::IdentityQueryFailed,
                    );
                }
                Err(_) => None,
            },
        };

        let private_state = match private_status.as_ref() {
            None => PlatformCrossSigningPrivateState::Unavailable,
            Some(status) => cross_signing_private_state(
                status.is_complete(),
                status.has_master,
                status.has_self_signing,
                status.has_user_signing,
            ),
        };
        let own_identity = match own_identity.as_ref() {
            Some(identity) if identity.is_verified() => PlatformCrossSigningOwnIdentity::Verified,
            Some(_) => PlatformCrossSigningOwnIdentity::Unverified,
            None if matches!(private_state, PlatformCrossSigningPrivateState::Complete) => {
                // Local private keys exist but the identity query did not
                // return. Offer verification instead of hanging the Devices
                // page on a spinner.
                PlatformCrossSigningOwnIdentity::Unverified
            }
            None => PlatformCrossSigningOwnIdentity::Missing,
        };
        PlatformCrossSigningStatus::new(session_generation, private_state, own_identity)
    }

    /// Read secret-storage status through the desktop-owned Matrix session.
    ///
    /// This retains the pre-Core status command's auth mutex across every
    /// existing SDK observation and reduces its legacy DTO locally to fixed
    /// booleans/enums. No recovery material, key id, account-data value, SDK
    /// object, or raw diagnostic reaches the Platform/Core seam.
    pub(crate) async fn secret_storage_status_projection(
        &self,
    ) -> Result<PlatformSecretStorageStatus, PlatformSecretStorageStatusError> {
        let session = self.session.lock().await;
        let active = session
            .as_ref()
            .ok_or(PlatformSecretStorageStatusError::NoSession)?;
        let status = live_secret_storage::status(&active.client, active.sync.session_generation())
            .await
            .map_err(map_secret_storage_status_error)?;
        platform_secret_storage_status(status)
    }

    /// Read the upload-size config through the desktop-owned SDK client.
    ///
    /// This preserves the pre-Core `matrix_media_config` concurrency contract
    /// exactly: clone the SDK Client while holding the auth mutex, release that
    /// mutex, then allow `load_or_fetch_max_upload_size` to use its cache or
    /// network. `Client` is reference-counted and the old command already did
    /// this, so logout/session replacement may proceed without invalidating the
    /// in-flight client/cache load. Core receives only the closed scalar result.
    pub(crate) async fn media_config_projection(
        &self,
    ) -> Result<PlatformMediaConfig, PlatformMediaConfigError> {
        let client = {
            let session = self.session.lock().await;
            let active = session
                .as_ref()
                .ok_or(PlatformMediaConfigError::NoSession)?;
            active.client.clone()
        };
        let upload_size = client
            .load_or_fetch_max_upload_size()
            .await
            .map_err(|_| PlatformMediaConfigError::LoadFailed)?;
        let upload_size = u64::try_from(i64::from(upload_size))
            .map_err(|_| PlatformMediaConfigError::UnsafeSize)?;
        PlatformMediaConfig::new(upload_size)
    }

    /// A normal login supersedes any abandoned recovery affordance. This only
    /// clears a process-local capability; it does not touch files or Keychain.
    pub(super) async fn clear_store_recovery(&self) {
        *self.store_recovery.lock().await = StoreRecoveryState::Idle;
    }

    /// Remember an account only after an allowlisted failed store-open path.
    /// The identity remains in the host and is never returned by recovery IPC.
    pub(super) async fn arm_store_recovery(&self, identity: AccountIdentity) {
        *self.store_recovery.lock().await = StoreRecoveryState::Pending { identity };
    }

    pub(super) async fn prepare_store_recovery_confirmation(
        &self,
    ) -> Result<String, MatrixAuthCommandError> {
        let mut recovery = self.store_recovery.lock().await;
        let identity = match std::mem::replace(&mut *recovery, StoreRecoveryState::Idle) {
            StoreRecoveryState::Pending { identity } => identity,
            StoreRecoveryState::Idle | StoreRecoveryState::AwaitingConfirmation { .. } => {
                return Err(MatrixAuthCommandError::new(
                    "InvalidRequest",
                    "Local Matrix store recovery must be requested from a failed login.",
                    "p3.2-login-store-recovery-not-pending",
                ));
            }
        };
        let confirmation_id = new_store_recovery_confirmation_id()?;
        *recovery = StoreRecoveryState::AwaitingConfirmation {
            identity,
            confirmation_id: confirmation_id.clone(),
        };
        Ok(confirmation_id)
    }

    /// Consume the CSPRNG confirmation capability before filesystem work so it
    /// cannot be replayed after either success or failure. The fixed typed
    /// acknowledgement is a second independent host-side requirement; neither
    /// a renderer button state nor a valid opaque ID alone can authorize an
    /// archive. Wrong input leaves a pending capability untouched so the user
    /// can correct a transport/UI error without rearming recovery from a new
    /// login failure.
    pub(super) async fn take_confirmed_store_recovery(
        &self,
        confirmation_id: &str,
        confirmation_text: &str,
    ) -> Result<AccountIdentity, MatrixAuthCommandError> {
        if confirmation_text != STORE_RECOVERY_TYPED_CONFIRMATION_TEXT
            || !is_store_recovery_confirmation_id(confirmation_id)
        {
            return Err(store_recovery_confirmation_error());
        }
        let mut recovery = self.store_recovery.lock().await;
        let valid = matches!(
            &*recovery,
            StoreRecoveryState::AwaitingConfirmation {
                confirmation_id: expected,
                ..
            } if expected == confirmation_id
        );
        if !valid {
            return Err(store_recovery_confirmation_error());
        }
        match std::mem::replace(&mut *recovery, StoreRecoveryState::Idle) {
            StoreRecoveryState::AwaitingConfirmation { identity, .. } => Ok(identity),
            StoreRecoveryState::Idle | StoreRecoveryState::Pending { .. } => {
                Err(store_recovery_confirmation_error())
            }
        }
    }

    /// Resolve an opaque V-ROOMS invite-avatar capability for the native URI
    /// protocol. The handle is valid only for the live session generation and
    /// never reveals its MXC source to the webview or command IPC.
    pub async fn resolve_invite_avatar(
        &self,
        handle: &str,
    ) -> Option<(Client, crate::matrix::room_list::InviteAvatarSource)> {
        let session = self.session.lock().await;
        let active = session.as_ref()?;
        let source = active
            .invite_avatars
            .lock()
            .await
            .resolve(active.sync.session_generation(), handle)?;
        Some((active.client.clone(), source))
    }

    /// Resolve a stream/session-bound V-TIMELINE media capability. Neither the
    /// SDK media source nor downloaded bytes cross command IPC.
    pub async fn resolve_timeline_media(
        &self,
        handle: &str,
    ) -> Option<(Client, TimelineMediaSource)> {
        let session = self.session.lock().await;
        let active = session.as_ref()?;
        let source = active.timelines.lock().await.resolve_media(handle).await?;
        Some((active.client.clone(), source))
    }
}

/// Reduce the existing desktop-only secret-storage DTO before it reaches Core.
///
/// `NativeSecretStorageStatus` can contain a dynamic public list locally; this
/// conversion collapses its four known labels into fixed bits before returning.
fn platform_secret_storage_status(
    status: NativeSecretStorageStatus,
) -> Result<PlatformSecretStorageStatus, PlatformSecretStorageStatusError> {
    let state = match status.state {
        NativeSecretStorageState::Unavailable => PlatformSecretStorageState::Unavailable,
        NativeSecretStorageState::NotSetUp => PlatformSecretStorageState::NotSetUp,
        NativeSecretStorageState::Locked => PlatformSecretStorageState::Locked,
        NativeSecretStorageState::Ready => PlatformSecretStorageState::Ready,
    };
    let action = match status.action {
        NativeSecretStorageAction::BootstrapRequired => {
            PlatformSecretStorageAction::BootstrapRequired
        }
        NativeSecretStorageAction::UnlockRequired => PlatformSecretStorageAction::UnlockRequired,
        NativeSecretStorageAction::None => PlatformSecretStorageAction::None,
    };
    let missing_secrets = PlatformSecretStorageMissingSecrets::new(
        status
            .missing_secrets
            .contains(&NativeMissingSecret::CrossSigningMaster),
        status
            .missing_secrets
            .contains(&NativeMissingSecret::CrossSigningSelfSigning),
        status
            .missing_secrets
            .contains(&NativeMissingSecret::CrossSigningUserSigning),
        status
            .missing_secrets
            .contains(&NativeMissingSecret::EncryptionBackup),
    );
    PlatformSecretStorageStatus::new(
        status.session_generation,
        state,
        status.exists,
        status.unlocked,
        status.default_key_set,
        status.passphrase_configured,
        status.bootstrap_ready,
        missing_secrets,
        action,
    )
}

/// Map only the three exact legacy status failures to closed Platform errors.
/// Any unexpected local result fails closed without moving a diagnostic string.
fn map_secret_storage_status_error(
    error: MatrixAuthCommandError,
) -> PlatformSecretStorageStatusError {
    match error.diagnostic_id.as_str() {
        "v-crypto.4-status-default-key-failed" => {
            PlatformSecretStorageStatusError::DefaultKeyLoadFailed
        }
        "v-crypto.4-status-key-info-failed" => PlatformSecretStorageStatusError::KeyInfoLoadFailed,
        "v-crypto.4-status-secret-check-failed" => {
            PlatformSecretStorageStatusError::SecretCheckFailed
        }
        _ => PlatformSecretStorageStatusError::InvalidSnapshot,
    }
}

/// Reduce the current desktop SDK observation to only the existing coarse
/// cross-signing vocabulary. The inputs are booleans so no SDK type can cross
/// the Platform/Core seam.
fn crypto_cross_signing_state(
    is_complete: bool,
    has_master: bool,
    has_self_signing: bool,
    has_user_signing: bool,
) -> PlatformCryptoCrossSigningState {
    if is_complete {
        PlatformCryptoCrossSigningState::Ready
    } else if has_master || has_self_signing || has_user_signing {
        PlatformCryptoCrossSigningState::Partial
    } else {
        PlatformCryptoCrossSigningState::NotSetUp
    }
}

/// Reduce the desktop SDK's private cross-signing result locally, before the
/// closed projection enters the Platform/Core seam. This keeps all SDK status
/// types and key details in the desktop process.
fn cross_signing_private_state(
    is_complete: bool,
    has_master: bool,
    has_self_signing: bool,
    has_user_signing: bool,
) -> PlatformCrossSigningPrivateState {
    if is_complete {
        PlatformCrossSigningPrivateState::Complete
    } else if has_master || has_self_signing || has_user_signing {
        PlatformCrossSigningPrivateState::Partial
    } else {
        PlatformCrossSigningPrivateState::Missing
    }
}

const STORE_RECOVERY_CONFIRMATION_ID_BYTES: usize = 32;
/// Exact acknowledgement that the host requires in addition to the opaque
/// CSPRNG confirmation capability. This is intentionally validated only in
/// the native process; renderer-side button state is not an authorization
/// boundary.
pub(super) const STORE_RECOVERY_TYPED_CONFIRMATION_TEXT: &str = "ARCHIVE";

/// Produce an opaque, CSPRNG-backed, one-use confirmation capability. It is
/// neither a Matrix credential nor a store key, and it is never logged.
fn new_store_recovery_confirmation_id() -> Result<String, MatrixAuthCommandError> {
    let mut bytes = [0_u8; STORE_RECOVERY_CONFIRMATION_ID_BYTES];
    getrandom::fill(&mut bytes).map_err(|_| {
        MatrixAuthCommandError::new(
            "Unknown",
            "Local Matrix store recovery confirmation is unavailable.",
            "p3.2-login-store-recovery-confirmation-unavailable",
        )
    })?;
    let mut id = String::with_capacity(STORE_RECOVERY_CONFIRMATION_ID_BYTES * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(id, "{byte:02x}");
    }
    Ok(id)
}

fn is_store_recovery_confirmation_id(value: &str) -> bool {
    value.len() == STORE_RECOVERY_CONFIRMATION_ID_BYTES * 2
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn store_recovery_confirmation_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "InvalidRequest",
        "Local Matrix store recovery confirmation is invalid or has expired.",
        "p3.2-login-store-recovery-confirmation-required",
    )
}

// Shared fail-closed session guards used by the domain command modules.
fn require_session(
    session: Option<&ManagedMatrixSession>,
) -> Result<&ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.3-timeline-requires-session",
        )
    })
}

fn require_verification_session(
    session: Option<&ManagedMatrixSession>,
) -> Result<&ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.1-verification-requires-session",
        )
    })
}

fn require_device_session(
    session: Option<&ManagedMatrixSession>,
) -> Result<&ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.7-device-requires-session",
        )
    })
}

fn require_device_session_mut(
    session: Option<&mut ManagedMatrixSession>,
) -> Result<&mut ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.7-device-requires-session",
        )
    })
}

fn require_cross_signing_session(
    session: Option<&ManagedMatrixSession>,
) -> Result<&ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.2-cross-signing-requires-session",
        )
    })
}

fn require_cross_signing_session_mut(
    session: Option<&mut ManagedMatrixSession>,
) -> Result<&mut ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.2-cross-signing-requires-session",
        )
    })
}

fn require_backup_session(
    session: Option<&ManagedMatrixSession>,
) -> Result<&ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.3-backup-requires-session",
        )
    })
}

fn require_secret_storage_session(
    session: Option<&ManagedMatrixSession>,
) -> Result<&ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.4-secret-storage-requires-session",
        )
    })
}

fn require_room_key_session(
    session: Option<&ManagedMatrixSession>,
) -> Result<&ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.5-room-keys-requires-session",
        )
    })
}

fn require_room_key_session_mut(
    session: Option<&mut ManagedMatrixSession>,
) -> Result<&mut ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.5-room-keys-requires-session",
        )
    })
}

async fn require_current_room_key_generation(
    state: &State<'_, MatrixAuthState>,
    generation: u64,
) -> Result<(), MatrixAuthCommandError> {
    let session = state.session.lock().await;
    if require_room_key_session(session.as_ref())?
        .sync
        .session_generation()
        != generation
    {
        return Err(stale_room_key_generation_error());
    }
    Ok(())
}

fn stale_room_key_generation_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "StaleSessionGeneration",
        "The native Matrix session changed during room-key transfer.",
        "v-crypto.5-stale-session-generation",
    )
}

fn require_session_mut(
    session: Option<&mut ManagedMatrixSession>,
) -> Result<&mut ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.3-timeline-requires-session",
        )
    })
}

#[path = "../account_data/product_commands.rs"]
mod account_data;
#[path = "product_commands.rs"]
mod auth_commands;
#[path = "../backup/product_commands.rs"]
mod backup;
#[path = "../cross_signing/product_commands.rs"]
mod cross_signing;
#[path = "../devices/product_commands.rs"]
mod devices;
#[path = "../media/product_commands.rs"]
mod media;
#[path = "../members/product_commands.rs"]
mod members;
#[path = "../presence/product_commands.rs"]
mod presence;
#[path = "../room_directory/product_commands.rs"]
mod room_directory;
#[path = "../room_keys/product_commands.rs"]
mod room_keys;
#[path = "../room_list/product_commands.rs"]
mod room_list;
#[path = "../room_ops/product_commands.rs"]
mod room_ops;
#[path = "../room_profile/product_commands.rs"]
mod room_profile;
#[path = "../search/product_commands.rs"]
mod search;
#[path = "../secret_storage/product_commands.rs"]
mod secret_storage;
#[path = "../send/product_commands.rs"]
mod send;
#[path = "../spaces/product_commands.rs"]
mod spaces;
#[path = "../timeline/product_commands.rs"]
mod timeline;
#[path = "../typing/product_commands.rs"]
mod typing;
#[path = "../user_profile/product_commands.rs"]
mod user_profile;
#[path = "../verification/product_commands.rs"]
mod verification;
pub use account_data::*;
pub use auth_commands::*;
pub use backup::*;
pub use cross_signing::*;
pub use devices::*;
pub use media::*;
pub use members::*;
pub use presence::*;
pub use room_directory::*;
pub use room_keys::*;
pub use room_list::*;
pub use room_ops::*;
pub use room_profile::*;
pub use search::*;
pub use secret_storage::*;
pub use send::*;
pub use spaces::*;
pub use timeline::*;
pub use typing::*;
pub use user_profile::*;
pub use verification::*;

#[cfg(test)]
#[path = "product_tests.rs"]
mod tests;
