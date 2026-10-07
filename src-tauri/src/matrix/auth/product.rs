//! D0.1–D0.3 product password-login, native session, sync, and timeline ownership.
//!
//! This is the only desktop product boundary for password login. The live
//! `matrix_sdk::Client` and all access/refresh tokens remain in the Rust host.

use std::fs;

use std::path::{Path, PathBuf};

use std::sync::atomic::{AtomicU64, Ordering};

use std::sync::Arc;

use std::time::{Duration, Instant, SystemTime};

#[cfg(test)]
use matrix_sdk::ruma::{
    api::client::room::{create_room, Visibility},
    Int, OwnedRoomOrAliasId, OwnedServerName, OwnedUserId,
};

use matrix_sdk::{
    media::{MediaFormat, MediaRequestParameters},
    ruma::{
        api::client::uiaa, events::room::MediaSource, OwnedEventId, OwnedMxcUri, OwnedRoomId,
        OwnedTransactionId,
    },
    Client,
};

#[cfg(test)]
use super::LoginFlow;

use mime::Mime;

use serde::{Deserialize, Serialize};

use synara_core::platform::{
    PlatformCrossSigningOwnIdentity, PlatformCrossSigningPrivateState, PlatformCrossSigningStatus,
    PlatformCryptoCrossSigningState, PlatformCryptoStatus, PlatformMediaConfig,
    PlatformMediaConfigError, PlatformSecretStorageAction, PlatformSecretStorageMissingSecrets,
    PlatformSecretStorageState, PlatformSecretStorageStatus, PlatformSecretStorageStatusError,
};

use tauri::{AppHandle, Emitter, Manager, State};

use tokio::sync::Mutex;

use zeroize::Zeroize;

use super::{
    complete_password_reset, existing_sqlite_crypto_device_id, login_with_password,
    normalize_homeserver_url, password_reset_ephemeral_user_id, register_ephemeral_user_id,
    register_submit, request_password_email_token, request_register_email_token, AuthError,
    LoginOptions, PasswordEmailTokenResult, PasswordResetOutcome, RegisterAuthStage,
    RegisterFlowsProbe, RegisterSubmitOutcome, RegisterUiaFlow,
};

use crate::matrix::account_data::{
    NativeAgentApprovalHistorySnapshot, NativeGlobalImagePacksSnapshot, NativeImagePackOwner,
    NativeLaterSnapshot, NativeMDirectMutationResult, NativeMDirectSnapshot,
    NativeRoomImagePacksSnapshot, NativeRoomNotesSnapshot, NativeUserImagePackSnapshot,
    RoomNoteMoveDirection, SynaraLaterItem, SynaraRoomNoteItem,
};

use crate::matrix::backup::live::{
    self as live_backup, NativeBackupOperationResult, NativeBackupStatus,
};

use crate::matrix::client_builder::{
    build_unauthenticated_client, ClientBuildConfig, ClientBuilderError,
};

use crate::matrix::cross_signing::live::{NativeCrossSigningSetupResult, NativeCrossSigningStatus};

use crate::matrix::dehydrated_devices::NativeDehydratedDevicesOwner;

use crate::matrix::devices::{NativeDeviceDeleteResult, NativeDeviceOwner, NativeDeviceSnapshot};

use crate::matrix::lifecycle::{
    clear_session_material, load_session_material, matrix_session_from_host_secrets,
    persist_session_after_login, restore_session_from_vault, restore_session_onto_client,
    KeyringSessionMaterialVault, SessionMaterial,
};

use crate::matrix::notifications::NativeNotificationObservationOwner;

use crate::matrix::presence::NativePresenceOwner;

use crate::matrix::room_keys::{
    live::{
        self as live_room_keys, NativeRoomKeyFileSelection, NativeRoomKeyTransferResult,
        NativeRoomKeyTransferStatus, SelectedRoomKeyImport,
    },
    RoomKeyTransferFlow,
};

use crate::matrix::room_list::{InviteAvatarHandles, NativeInviteSnapshot, NativeRoomListSnapshot};

use crate::matrix::room_profile::NativeRoomJoinRuleOwner;

use crate::matrix::rtc_transports::NativeRtcTransportsOwner;

use crate::matrix::secret_storage::live::{
    self as live_secret_storage, NativeMissingSecret, NativeSecretStorageAction,
    NativeSecretStorageOperationResult, NativeSecretStorageState, NativeSecretStorageStatus,
};

use crate::matrix::send::{AttachmentEnqueue, AttachmentKind, AttachmentSendQueue};

use crate::matrix::spaces::{
    NativeRestrictedJoinReparentResult, NativeSpaceChildMutationResult,
    NativeSpaceChildrenSnapshot, NativeSpaceHierarchySnapshot, NativeSpaceParentsSnapshot,
};

use crate::matrix::store::{
    get_or_migrate_store_key, migrate_store_to_current, reset_store_for_recovery, AccountIdentity,
    KeyringStoreKeyVault, StoreKeyMaterial, StoreKeyVaultError, StoreMigrationError, StorePaths,
};

use crate::matrix::sync::{
    build_sync_service, recover_cooldown_active, suspend_detected, unconfigured_snapshot,
    SyncError, SyncIntent, SyncReadinessSnapshot, SyncServiceConfig, SyncServiceOwner,
    RECOVER_COOLDOWN, SUSPEND_WALL_SKEW,
};

use crate::matrix::timeline::{
    NativeComposerClearReplyDraftRequest, NativeComposerReplyDraftReadback,
    NativeComposerReplyDraftRoomRequest, NativeComposerSetReplyDraftRequest,
    NativePinnedEventsRequest, NativeReactionMutationResult, NativeTimelineActionReadback,
    NativeTimelineCallDeclineRequest, NativeTimelineCloseRequest, NativeTimelineEditTextRequest,
    NativeTimelineEventReadback, NativeTimelineForwardMediaRequest,
    NativeTimelineForwardTextRequest, NativeTimelineJumpLatestRequest, NativeTimelineOpenReadback,
    NativeTimelineOpenRequest, NativeTimelineOwner, NativeTimelinePinRequest,
    NativeTimelinePollVoteRequest, NativeTimelineReadAction, NativeTimelineReadStateReadback,
    NativeTimelineReadStateRequest, NativeTimelineRedactRequest, NativeTimelineReportRequest,
    NativeTimelineViewPaginationRequest, PinnedEventsSnapshot, TimelineMediaSource,
};

use crate::matrix::typing::{NativeTypingOwner, NativeTypingSnapshot};

use crate::matrix::user_profile::NativeOwnProfileOwner;

use crate::matrix::user_status::NativeUserStatusOwner;

use crate::matrix::verification::live::{
    NativeVerificationInbox, NativeVerificationOwner, NativeVerificationRequest,
};

use crate::matrix::widgets::NativeWidgetOwner;

use synara_core::app::media_cache::NativeMediaRetentionOwner;

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
    MatrixMediaConfigResult, MatrixMediaPreviewSnapshot, MatrixUploadMediaResult,
};

#[cfg(test)]
pub use synara_core::app::media::{MatrixMediaDownloadRequest, MatrixMediaDownloadResult};

pub use synara_core::app::members::NativeRoomMembersSnapshot;

pub use synara_core::app::send::{
    MatrixPollRespondResult, MatrixSendAttachmentResult, MatrixSendPollResult, MatrixSendTextResult,
};

pub use synara_core::app::user_profile::MatrixProfileWriteResult;

pub use synara_core::app::members::NativePowerLevelWriteResult;

pub use synara_core::app::room_ops::MatrixRoomCreateRequest;

#[cfg(test)]
pub use synara_core::app::room_ops::{
    MatrixRoomCreateContent, MatrixRoomCreatePowerLevels, MatrixRoomCreatePreset,
    MatrixRoomCreateVisibility,
};

/// Soft IPC/body cap for one-shot composer attachment transfer (bytes).
const MAX_ATTACHMENT_IPC_BYTES: usize = 32 * 1024 * 1024;

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
    session_persistence: SessionPersistenceOwner,
    identity: MatrixLoginIdentity,
    sync: Arc<SyncServiceOwner>,
    invite_avatars: Arc<tokio::sync::Mutex<InviteAvatarHandles>>,
    timelines: Arc<NativeTimelineOwner>,
    attachments: AttachmentSendQueue,
    verification: Arc<NativeVerificationOwner>,
    devices: Arc<NativeDeviceOwner>,
    dehydrated_devices: Arc<NativeDehydratedDevicesOwner>,
    _image_packs: Arc<NativeImagePackOwner>,
    typing: Arc<NativeTypingOwner>,
    presence: Arc<NativePresenceOwner>,
    rtc_transports: Arc<NativeRtcTransportsOwner>,
    user_status: Arc<NativeUserStatusOwner>,
    widgets: Arc<NativeWidgetOwner>,
    join_rules: Arc<NativeRoomJoinRuleOwner>,
    _own_profile: NativeOwnProfileOwner,
    _media_retention: NativeMediaRetentionOwner,
    /// Emits `matrix-room-list-updated`; dropping the session aborts it.
    _room_list_live: synara_core::app::room_list::NativeRoomListOwner,
    /// Core→renderer observation stream; retired on logout, dropped with
    /// the session.
    notification_observations: Arc<NativeNotificationObservationOwner>,
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
struct RecoverGate {
    in_flight: bool,
    last_success_wall: Option<SystemTime>,
}

/// Serialize suspend recovery with logout/replacement for the installed owner.
/// Recovery observers acquire recovery then session; completion must release
/// session before reacquiring recovery to avoid reversing that lock order.
async fn recover_installed_session_owner<Session, Owner, Snapshot, Error, ResumeFuture>(
    session: &Mutex<Option<Session>>,
    recover_gate: &Mutex<RecoverGate>,
    ignore_cooldown: bool,
    owner: impl FnOnce(&Session) -> Owner,
    observe: impl FnOnce(Option<&Session>) -> Snapshot,
    resume: impl FnOnce(Owner) -> ResumeFuture,
) -> Result<Snapshot, Error>
where
    ResumeFuture: std::future::Future<Output = Result<Snapshot, Error>>,
{
    let mut gate = recover_gate.lock().await;
    if gate.in_flight
        || (!ignore_cooldown
            && recover_cooldown_active(gate.last_success_wall, SystemTime::now(), RECOVER_COOLDOWN))
    {
        drop(gate);
        let guard = session.lock().await;
        return Ok(observe(guard.as_ref()));
    }
    let guard = session.lock().await;
    let Some(active) = guard.as_ref() else {
        return Ok(observe(None));
    };
    let owner = owner(active);
    gate.in_flight = true;
    drop(gate);
    let result = resume(owner).await;
    drop(guard);
    let mut gate = recover_gate.lock().await;
    gate.in_flight = false;
    if result.is_ok() {
        gate.last_success_wall = Some(SystemTime::now());
    }
    result
}

#[derive(Default)]
pub struct MatrixAuthState {
    session: Mutex<Option<ManagedMatrixSession>>,
    /// Session transition gate. Logout holds it through the whole teardown,
    /// including the bounded remote `/logout` and Core close; login, register,
    /// restore, and store recovery take it before `session`. Ordinary commands
    /// never take it, so they fail closed on the empty slot instead of waiting.
    transition: Mutex<()>,
    /// Identity whose credential cleanup failed after its live session left the
    /// slot. Restore refuses to reinstall it, and the watcher retries cleanup.
    pending_logout_cleanup: std::sync::Mutex<Option<MatrixLoginIdentity>>,
    store_recovery: Mutex<StoreRecoveryState>,
    next_session_generation: AtomicU64,
    recover_gate: Mutex<RecoverGate>,
}

impl MatrixAuthState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Accept a bound native notification only for the currently installed
    /// generation, retaining the auth transition gate through OS acceptance.
    /// The callback must not call back into this session gate.
    pub(crate) async fn with_session_generation<T, Accept, AcceptFuture>(
        &self,
        expected_generation: u64,
        accept: Accept,
    ) -> Option<T>
    where
        Accept: FnOnce() -> AcceptFuture,
        AcceptFuture: std::future::Future<Output = T>,
    {
        with_generation_bound_acceptance(
            &self.session,
            expected_generation,
            |active| active.sync.session_generation(),
            accept,
        )
        .await
    }

    /// Retry a failed save of the current in-memory tokens.
    ///
    /// Returns `None` when nothing needs saving, otherwise whether the write
    /// succeeded. The keyring write is synchronous D-Bus/Keychain work, so it
    /// runs on a blocking thread with a timeout and without the session mutex.
    /// The persistence lease still fences it: logout revokes the lease first.
    async fn retry_failed_session_save(&self, app: &AppHandle) -> Option<bool> {
        let (lease, client, identity, generation) = {
            let session = self.session.lock().await;
            let active = session.as_ref()?;
            let lease = active.session_persistence.callback_lease();
            if !lease.save_failed() {
                return None;
            }
            (
                lease,
                active.client.clone(),
                active.identity.clone(),
                active.sync.session_generation(),
            )
        };
        let root = app_data_root(app);
        let write = tauri::async_runtime::spawn_blocking(move || {
            root.and_then(|root| {
                let account = account_identity(&identity)?;
                let saved = lease.save_credentials(
                    || ensure_logout_retry_locator(&root, &identity),
                    || {
                        persist_session_after_login(
                            &client,
                            &account,
                            &KeyringSessionMaterialVault::new(),
                        )
                        .map(|_| ())
                        .map_err(map_session_rotation_persist_error)
                    },
                );
                record_session_rotation_outcome(&root, &saved);
                saved
            })
        });
        let saved = matches!(
            tokio::time::timeout(KEYRING_RETRY_TIMEOUT, write).await,
            Ok(Ok(Ok(())))
        );
        let _ = app.emit(
            "matrix-session-persistence",
            serde_json::json!({ "sessionGeneration": generation, "saved": saved }),
        );
        Some(saved)
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

    /// Restart the live SyncService after OS suspend while retaining the
    /// session transition gate through stop/start. Release the recovery gate
    /// during SDK work, and release the session gate before recovery bookkeeping.
    /// Concurrent renderer and watchdog calls share an in-flight flag so we
    /// do not stop/start twice on the same wake. Renderer IPC keeps a
    /// wall-clock cooldown; the native watchdog skips that cooldown after a
    /// proven suspend because monotonic time does not advance during sleep.
    pub(crate) async fn recover_sync_after_wake(&self) -> Result<SyncReadinessSnapshot, SyncError> {
        self.recover_live_sync(false).await
    }

    pub(crate) async fn recover_sync_after_detected_suspend(
        &self,
    ) -> Result<SyncReadinessSnapshot, SyncError> {
        self.recover_live_sync(true).await
    }

    async fn recover_live_sync(
        &self,
        ignore_cooldown: bool,
    ) -> Result<SyncReadinessSnapshot, SyncError> {
        recover_installed_session_owner(
            &self.session,
            &self.recover_gate,
            ignore_cooldown,
            |active| active.sync.clone(),
            |active| match active {
                Some(active) => active.sync.observe(),
                None => unconfigured_snapshot(self.current_generation()),
            },
            |owner| async move { owner.apply_intent(SyncIntent::Resume).await },
        )
        .await
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

    /// Serialize session installs and logout teardown. Always taken before
    /// `session`, never while holding it.
    pub(super) async fn lock_transition(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.transition.lock().await
    }

    pub(super) fn record_pending_logout_cleanup(&self, identity: MatrixLoginIdentity) {
        if let Ok(mut pending) = self.pending_logout_cleanup.lock() {
            *pending = Some(identity);
        }
    }

    pub(super) fn has_pending_logout_cleanup(&self) -> bool {
        self.pending_logout_cleanup
            .lock()
            .map(|pending| pending.is_some())
            .unwrap_or(true)
    }

    /// Retry credential cleanup for a session that already left the slot.
    /// Caller holds the transition gate so a new install cannot interleave.
    pub(super) fn retry_pending_logout_cleanup(
        &self,
        cleanup: impl FnOnce(&MatrixLoginIdentity) -> Result<(), MatrixAuthCommandError>,
    ) -> Result<(), MatrixAuthCommandError> {
        retry_pending_logout_cleanup(&self.pending_logout_cleanup, cleanup)
    }

    /// A new install replaces whatever the failed cleanup was retrying. Try
    /// once more, then forget it so a later retry cannot erase the new session.
    pub(super) fn settle_pending_logout_cleanup_before_install(
        &self,
        cleanup: impl FnOnce(&MatrixLoginIdentity) -> Result<(), MatrixAuthCommandError>,
    ) {
        let _ = self.retry_pending_logout_cleanup(cleanup);
        if let Ok(mut pending) = self.pending_logout_cleanup.lock() {
            *pending = None;
        }
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
        // Clone the owners and release the session mutex before awaiting the
        // timeline registry. A registry operation can be on the network, and
        // holding the session here would stall every status poll behind it.
        let (client, timelines) = {
            let session = self.session.lock().await;
            let active = session.as_ref()?;
            (active.client.clone(), Arc::clone(&active.timelines))
        };
        let source = timelines.lock().await.resolve_media(handle).await?;
        Some((client, source))
    }

    /// Live session client for plain `mxc://` display through the media protocol.
    pub(crate) async fn media_client(&self) -> Option<Client> {
        let session = self.session.lock().await;
        session.as_ref().map(|active| active.client.clone())
    }
}

pub(super) fn retry_pending_logout_cleanup(
    pending: &std::sync::Mutex<Option<MatrixLoginIdentity>>,
    cleanup: impl FnOnce(&MatrixLoginIdentity) -> Result<(), MatrixAuthCommandError>,
) -> Result<(), MatrixAuthCommandError> {
    let mut pending = pending
        .lock()
        .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-session-clear-failed"))?;
    let Some(identity) = pending.as_ref() else {
        return Ok(());
    };
    cleanup(identity)?;
    *pending = None;
    Ok(())
}

pub(super) const SESSION_REJECTION_NO_CORE_DIAGNOSTIC_ID: &str = "d0.1-session-rejection-no-core";

/// Per-generation log de-duplication for the rejection watcher. Concurrency
/// with a user logout is the transition gate's job, not this struct's: every
/// tick that still sees the rejected generation retries `matrix_logout`.
#[derive(Debug, Default)]
pub(super) struct AuthenticationRejectionWatch {
    logged_rejection: Option<u64>,
    logged_missing_core: Option<u64>,
    logged_failure: Option<(u64, String)>,
}

impl AuthenticationRejectionWatch {
    pub(super) fn should_log_rejection(&mut self, generation: u64) -> bool {
        if self.logged_rejection == Some(generation) {
            return false;
        }
        self.logged_rejection = Some(generation);
        true
    }

    pub(super) fn should_log_missing_core(&mut self, generation: u64) -> bool {
        if self.logged_missing_core == Some(generation) {
            return false;
        }
        self.logged_missing_core = Some(generation);
        true
    }

    fn should_log_failure(&mut self, generation: u64, diagnostic_id: &str) -> bool {
        if self
            .logged_failure
            .as_ref()
            .is_some_and(|(logged, id)| *logged == generation && id == diagnostic_id)
        {
            return false;
        }
        self.logged_failure = Some((generation, diagnostic_id.to_owned()));
        true
    }
}

/// One watcher tick for a sync snapshot that reports a rejected refresh.
///
/// `retire` is the local, generation-fenced `matrix_logout`; it is called only
/// when Core is present. Every line passed to `log` is a fixed lifecycle word or
/// a static `d0.1-*` / `p4.1-*` id. Returns whether retirement succeeded.
pub(super) async fn handle_authentication_rejection_tick<Retire, RetireFuture>(
    watch: &mut AuthenticationRejectionWatch,
    generation: u64,
    core_present: bool,
    mut log: impl FnMut(&str),
    retire: Retire,
) -> bool
where
    Retire: FnOnce(u64) -> RetireFuture,
    RetireFuture: std::future::Future<Output = Result<(), MatrixAuthCommandError>>,
{
    if watch.should_log_rejection(generation) {
        log("session-authentication-rejected");
    }
    if !core_present {
        if watch.should_log_missing_core(generation) {
            log(SESSION_REJECTION_NO_CORE_DIAGNOSTIC_ID);
        }
        return false;
    }
    match retire(generation).await {
        Ok(()) => true,
        Err(error) => {
            if let Some(diagnostic_id) = static_rejection_logout_diagnostic(&error.diagnostic_id) {
                if watch.should_log_failure(generation, diagnostic_id) {
                    log(diagnostic_id);
                }
            }
            false
        }
    }
}

/// Log a logout failure id only when it is already a static session diagnostic.
/// Anything else, including tokens and URLs, is dropped.
pub(super) fn static_rejection_logout_diagnostic(diagnostic_id: &str) -> Option<&str> {
    let static_id = diagnostic_id.starts_with("d0.1-") || diagnostic_id.starts_with("p4.1-");
    let closed_alphabet = diagnostic_id
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '.')
        && diagnostic_id.len() <= 80;
    (static_id && closed_alphabet).then_some(diagnostic_id)
}

/// Upper bound for one watchdog keyring write before the tick moves on.
const KEYRING_RETRY_TIMEOUT: Duration = Duration::from_secs(10);

/// Exponential backoff for watchdog retries of synchronous keyring work:
/// 5s after the first failure, doubling to at most 60s, reset on success.
#[derive(Debug)]
pub(super) struct RetryBackoff {
    delay: Duration,
    next_attempt: Option<Instant>,
}

impl RetryBackoff {
    const INITIAL: Duration = Duration::from_secs(5);
    const MAX: Duration = Duration::from_secs(60);

    pub(super) fn new() -> Self {
        Self {
            delay: Self::INITIAL,
            next_attempt: None,
        }
    }

    pub(super) fn ready(&self, now: Instant) -> bool {
        self.next_attempt.is_none_or(|at| now >= at)
    }

    pub(super) fn record(&mut self, now: Instant, succeeded: bool) {
        if succeeded {
            *self = Self::new();
        } else {
            self.next_attempt = Some(now + self.delay);
            self.delay = (self.delay * 2).min(Self::MAX);
        }
    }
}

/// Restart SyncService when wall time jumps ahead of monotonic time (OS sleep).
/// Linux sleep often leaves the webview visible, so renderer hooks never run.
pub fn spawn_suspend_resume_watch(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut previous_wall = SystemTime::now();
        let mut previous_mono = Instant::now();
        let mut rejection_watch = AuthenticationRejectionWatch::default();
        let mut save_backoff = RetryBackoff::new();
        let mut cleanup_backoff = RetryBackoff::new();
        loop {
            interval.tick().await;
            let now_wall = SystemTime::now();
            let now_mono = Instant::now();
            let slept = suspend_detected(
                previous_wall,
                previous_mono,
                now_wall,
                now_mono,
                SUSPEND_WALL_SKEW,
            );
            previous_wall = now_wall;
            previous_mono = now_mono;
            let Some(state) = app.try_state::<MatrixAuthState>() else {
                continue;
            };
            // A server refresh can succeed while its synchronous save callback
            // fails. Retry only the local write of the current in-memory tokens;
            // never replay the old refresh token or erase encryption data.
            // A broken keyring backs off instead of blocking a worker every tick.
            if save_backoff.ready(now_mono) {
                if let Some(saved) = state.retry_failed_session_save(&app).await {
                    save_backoff.record(Instant::now(), saved);
                }
            }
            // A retired session whose credential delete failed must not stay
            // restorable. Retry under the transition gate so a new login
            // cannot interleave with the delete.
            if state.has_pending_logout_cleanup() && cleanup_backoff.ready(now_mono) {
                let _transition = state.lock_transition().await;
                let cleaned = tokio::task::block_in_place(|| {
                    state.retry_pending_logout_cleanup(|identity| {
                        app_data_root(&app).and_then(|root| {
                            clear_native_logout_material(
                                &KeyringSessionMaterialVault::new(),
                                identity,
                                &root,
                            )
                        })
                    })
                })
                .is_ok();
                cleanup_backoff.record(Instant::now(), cleaned);
            }
            let snapshot = state.sync_status_snapshot().await;
            if snapshot.failure_diagnostic_id
                == Some(synara_core::app::sync::SYNC_AUTHENTICATION_FAILURE_DIAGNOSTIC_ID)
            {
                // A terminal rejected refresh is not an offline server. Retire
                // native work and invalid credentials through the existing
                // fenced logout owner. This does not erase the crypto store,
                // POST /logout, or send the rejected refresh token.
                let core = app.try_state::<Arc<synara_core::Core>>();
                let log_app = app.clone();
                let retire_app = app.clone();
                handle_authentication_rejection_tick(
                    &mut rejection_watch,
                    snapshot.session_generation,
                    core.is_some(),
                    |line| {
                        crate::desktop_logging::desktop_append_log(
                            log_app.clone(),
                            "native".into(),
                            line.to_owned(),
                        )
                    },
                    |generation| async move {
                        let core = core.expect("retire runs only when Core is present");
                        matrix_logout(retire_app, state, core, Some(generation))
                            .await
                            .map(|_| ())
                    },
                )
                .await;
                continue;
            }
            if !slept {
                continue;
            }
            if let Err(error) = state.recover_sync_after_detected_suspend().await {
                eprintln!(
                    "[synara] sync resume after suspend failed: {}",
                    error.diagnostic_id()
                );
            }
        }
    });
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
#[path = "../rtc_transports/product_commands.rs"]
mod rtc_transports;
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
#[path = "../user_status/product_commands.rs"]
mod user_status;
#[path = "../verification/product_commands.rs"]
mod verification;
#[path = "../widgets/product_commands.rs"]
mod widgets;
#[path = "../x509/product_commands.rs"]
mod x509_identity;
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

pub use rtc_transports::*;

pub use search::*;

pub use secret_storage::*;

pub use send::*;

pub use spaces::*;

pub use timeline::*;

pub use typing::*;

pub use user_profile::*;

pub use user_status::*;

pub use verification::*;

pub use widgets::*;

pub use x509_identity::*;

#[cfg(test)]
#[path = "product_tests.rs"]
mod tests;

#[cfg(test)]
pub(crate) use send::message_content;

#[cfg(test)]
use send::{edit_message_content, parse_edit_event_id};

#[cfg(test)]
use user_profile::{parse_avatar_mxc, parse_display_name};

#[cfg(test)]
use media::validate_media_download_size;

#[cfg(test)]
mod retry_backoff_tests {
    use super::RetryBackoff;
    use std::time::{Duration, Instant};

    #[test]
    fn keyring_retries_back_off_to_a_minute_and_reset_on_success() {
        let start = Instant::now();
        let mut backoff = RetryBackoff::new();
        assert!(backoff.ready(start), "the first attempt runs immediately");

        let mut now = start;
        for expected in [5, 10, 20, 40, 60, 60] {
            backoff.record(now, false);
            assert!(!backoff.ready(now + Duration::from_secs(expected) - Duration::from_millis(1)));
            now += Duration::from_secs(expected);
            assert!(backoff.ready(now), "retry after {expected}s");
        }

        backoff.record(now, true);
        assert!(backoff.ready(now), "success clears the wait");
        backoff.record(now, false);
        assert!(!backoff.ready(now + Duration::from_secs(4)));
        assert!(
            backoff.ready(now + Duration::from_secs(5)),
            "success resets to 5s"
        );
    }
}
