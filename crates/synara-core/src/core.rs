//! Shared native-core entry points (P2 foundation).
//!
//! `Core` owns safe session projection/lifecycle plus the transport command
//! registry. It intentionally has no Tauri dependency; P2 command groups add
//! handlers, P3 makes the desktop shell a thin `Core::command` registrar.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::app::account_data::{
    NativeAgentApprovalHistorySnapshot, NativeGlobalImagePacksSnapshot, NativeImagePackOwner,
    NativeLaterSnapshot, NativeMDirectMutationResult, NativeMDirectSnapshot,
    NativeRoomImagePacksSnapshot, NativeRoomNotesSnapshot, NativeUserImagePackSnapshot,
    RoomNoteMoveDirection, SynaraLaterItem, SynaraRoomNoteItem,
};
use crate::app::auth::{
    discover_login_flows, login_flows_response, probe_register_flows, AuthError,
    HttpLoginFlowTransport, HttpRegisterFlowTransport, MatrixLoginFlowsResponse,
    RegisterFlowsProbe,
};
use crate::app::backup::{MatrixRestoreBackupResult, NativeBackupStatus};
use crate::app::cross_signing::NativeCrossSigningSetupResult;
use crate::app::dehydrated_devices::NativeDehydratedDevicesOwner;
use crate::app::devices::{NativeDeviceDeleteResult, NativeDeviceOwner, NativeDeviceSnapshot};
use crate::app::media::{MatrixMediaPreviewSnapshot, MatrixUploadMediaResult};
use crate::app::members::{
    NativePowerLevelWriteResult, NativeRoomCreatorsSnapshot, NativeRoomMembersSnapshot,
    NativeRoomPowerLevelTagsSnapshot, NativeRoomPowerLevelsSnapshot, ROOM_POWER_LEVELS_EVENT_TYPE,
    ROOM_POWER_LEVEL_TAGS_EVENT_TYPE,
};
use crate::app::notifications::{
    MatrixHttpPusherWriteResult, MatrixPushRulesSnapshot, MatrixPushRulesWriteResult,
    MatrixRoomNotificationSnapshot, MatrixRoomNotificationWriteResult,
    MatrixRoomNotificationsSnapshot, NativeHttpPusherOwner, NativeNotificationDecideRequest,
    NativeNotificationDecisionOwner, NativeNotificationDismissRequest,
    NativeNotificationFocusSetRequest, NotificationDecisionReadback,
};
use crate::app::presence::{
    NativePresenceOwner, NativePresenceSnapshotResult, NativePresenceSubscription,
    NativePresenceWriteResult,
};
use crate::app::room_directory::{
    DirectoryRoomTypeFilter, DirectorySearchInput, NativeRoomDirectoryProtocols,
    NativeRoomDirectorySearchResponse,
};
use crate::app::room_keys::NativeRoomKeyTransferStatus;
use crate::app::room_list::{
    snapshot_from_sync_owner, NativeInviteSnapshot, NativeRoomListSnapshot,
};
use crate::app::room_ops::{set_encrypted_state_events_setting_enabled, MatrixRoomCreateRequest};
use crate::app::room_profile::{
    MatrixRoomDirectoryVisibilityResult, MatrixRoomDirectoryVisibilityWriteResult,
    MatrixRoomJoinRuleSnapshot, MatrixRoomRetentionSnapshot, NativeRoomJoinRuleOwner,
};
use crate::app::rtc_transports::{NativeRtcTransportsOwner, NativeRtcTransportsSnapshot};
use crate::app::search::MatrixMessageSearchResult;
use crate::app::send::{
    MatrixPollRespondResult, MatrixSendPollResult, MatrixSendRoomAttachmentResult,
    MatrixSendTextResult, SendRoomAttachmentRequest,
};
use crate::app::spaces::{
    NativeRestrictedJoinReparentResult, NativeSpaceChildMutationResult,
    NativeSpaceChildrenSnapshot, NativeSpaceHierarchySnapshot, NativeSpaceParentsSnapshot,
};
use crate::app::sync::{
    SyncReadiness, SyncReadinessSnapshot, SyncServiceOwner, SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID,
};
use crate::app::threads::NativeThreadListSnapshot;
use crate::app::timeline::{
    NativeAgentApprovalDecisionRequest, NativeAgentApprovalDecisionResult,
    NativeComposerReplyDraftReadback, NativeReactionMutationResult, NativeTimelineActionReadback,
    NativeTimelineCloseRequest, NativeTimelineDirection, NativeTimelineEventReadback,
    NativeTimelineFollowLiveRequest, NativeTimelineJumpLatestRequest, NativeTimelineOpenPosition,
    NativeTimelineOpenReadback, NativeTimelineOpenRequest, NativeTimelineOwner,
    NativeTimelineReadAction, NativeTimelineReadIntent, NativeTimelineReadStateReadback,
    NativeTimelineReadStateRequest, NativeTimelineTimestampToEventReadback,
    NativeTimelineViewPaginationRequest, TimelineViewSnapshot,
};
use crate::app::typing::{NativeTypingOwner, NativeTypingSnapshot};
use crate::app::user_profile::{
    MatrixIgnoredUsersSnapshot, MatrixIgnoredUsersWriteResult, MatrixOwnProfile,
    MatrixProfileWriteResult, MatrixThreepidAddResult, MatrixThreepidEmailTokenResult,
    MatrixThreepidSnapshot, MatrixThreepidWriteResult, MatrixUploadAvatarResult,
    MatrixUserDirectorySearchResult,
};
use crate::app::user_status::{
    NativeUserStatusOwner, NativeUserStatusSnapshot, NativeUserStatusWriteResult,
};
use crate::app::verification::{
    NativeVerificationInbox, NativeVerificationOwner, NativeVerificationRequest,
};
use crate::app::widgets::{
    AgentWidgetEntry, NativeWidgetOwner, WidgetGrantPolicy, WidgetKind, WidgetListSnapshot,
    WidgetOpenResult, WidgetSessionRecord,
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

// Domain command implementations keep request validation, closed projections,
// and owner error mapping together. Only the parent registry imports them.
mod account_data;
use account_data::*;
mod messaging;
use messaging::*;
mod notifications;
use notifications::*;
mod profile_media;
use profile_media::*;
mod realtime;
use realtime::*;
mod room_administration;
use room_administration::*;
mod session_crypto;
use session_crypto::*;

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
    rtc_transports: Mutex<Option<Arc<NativeRtcTransportsOwner>>>,
    user_status: Mutex<Option<Arc<NativeUserStatusOwner>>>,
    verification: Mutex<Option<Arc<NativeVerificationOwner>>>,
    devices: Mutex<Option<Arc<NativeDeviceOwner>>>,
    dehydrated_devices: Mutex<Option<Arc<NativeDehydratedDevicesOwner>>>,
    join_rules: Mutex<Option<Arc<NativeRoomJoinRuleOwner>>>,
    image_packs: Mutex<Option<Arc<NativeImagePackOwner>>>,
    http_pusher: Mutex<Option<Arc<NativeHttpPusherOwner>>>,
    notification_decisions: Mutex<Option<Arc<NativeNotificationDecisionOwner>>>,
    timelines: Mutex<Option<Arc<NativeTimelineOwner>>>,
    sync: Mutex<Option<Arc<SyncServiceOwner>>>,
    widgets: Mutex<Option<Arc<NativeWidgetOwner>>>,
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

    fn rtc_transports_owner(
        &self,
    ) -> Result<Option<Arc<NativeRtcTransportsOwner>>, MatrixIpcError> {
        self.rtc_transports
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| core_state_error("p2-core-state-poisoned"))
    }

    fn user_status_owner(&self) -> Result<Option<Arc<NativeUserStatusOwner>>, MatrixIpcError> {
        self.user_status
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

    fn dehydrated_devices_owner(
        &self,
    ) -> Result<Option<Arc<NativeDehydratedDevicesOwner>>, MatrixIpcError> {
        self.dehydrated_devices
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

    fn http_pusher_owner(&self) -> Result<Option<Arc<NativeHttpPusherOwner>>, MatrixIpcError> {
        self.http_pusher
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| core_state_error("p2-core-state-poisoned"))
    }

    fn notification_decision_owner(
        &self,
    ) -> Result<Option<Arc<NativeNotificationDecisionOwner>>, MatrixIpcError> {
        self.notification_decisions
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

    fn widget_owner(&self) -> Result<Option<Arc<NativeWidgetOwner>>, MatrixIpcError> {
        self.widgets
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
                rtc_transports: Mutex::new(None),
                user_status: Mutex::new(None),
                verification: Mutex::new(None),
                devices: Mutex::new(None),
                dehydrated_devices: Mutex::new(None),
                join_rules: Mutex::new(None),
                image_packs: Mutex::new(None),
                http_pusher: Mutex::new(None),
                notification_decisions: Mutex::new(None),
                timelines: Mutex::new(None),
                sync: Mutex::new(None),
                widgets: Mutex::new(None),
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
        let mut rtc_transports = self
            .state
            .rtc_transports
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *rtc_transports = None;
        drop(rtc_transports);
        let mut user_status = self
            .state
            .user_status
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *user_status = None;
        drop(user_status);
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
        let mut dehydrated_devices = self
            .state
            .dehydrated_devices
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *dehydrated_devices = None;
        drop(dehydrated_devices);
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
        let mut http_pusher = self
            .state
            .http_pusher
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *http_pusher = None;
        drop(http_pusher);
        let mut notification_decisions = self
            .state
            .notification_decisions
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *notification_decisions = None;
        drop(notification_decisions);
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
        drop(sync);
        let mut widgets = self
            .state
            .widgets
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *widgets = None;
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
    /// Core snapshots it for `matrix_presence_snapshot` and writes through
    /// `matrix_presence_set`; the shell keeps an Arc for subscribe/unsubscribe
    /// and event-handler lifetime.
    pub fn attach_presence(&self, owner: Arc<NativePresenceOwner>) -> Result<(), MatrixIpcError> {
        let mut presence = self
            .state
            .presence
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *presence = Some(owner);
        Ok(())
    }

    /// Install the live MatrixRTC transport owner created after login/restore.
    /// Core snapshots it for `matrix_rtc_transports_snapshot` / refresh.
    pub fn attach_rtc_transports(
        &self,
        owner: Arc<NativeRtcTransportsOwner>,
    ) -> Result<(), MatrixIpcError> {
        let mut rtc_transports = self
            .state
            .rtc_transports
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *rtc_transports = Some(owner);
        Ok(())
    }

    /// Install the live MSC4426 status owner created after login/restore.
    /// Core snapshots it for `matrix_user_status_snapshot` and writes through
    /// `matrix_user_status_set` / `clear`. This never calls `set_call`.
    pub fn attach_user_status(
        &self,
        owner: Arc<NativeUserStatusOwner>,
    ) -> Result<(), MatrixIpcError> {
        let mut user_status = self
            .state
            .user_status
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *user_status = Some(owner);
        Ok(())
    }

    /// Install the live experimental widget owner created by the shell after
    /// login/restore. Runtime enablement stays on the command payloads
    /// (`experimentalWidgetsEnabled`); attaching the owner is not enablement.
    pub fn attach_widgets(&self, owner: Arc<NativeWidgetOwner>) -> Result<(), MatrixIpcError> {
        let mut widgets = self
            .state
            .widgets
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *widgets = Some(owner);
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

    /// Install the live dehydrated-device manager. Start failures stay on
    /// the owner; attach itself is infallible from the shell's point of view.
    pub fn attach_dehydrated_devices(
        &self,
        owner: Arc<NativeDehydratedDevicesOwner>,
    ) -> Result<(), MatrixIpcError> {
        let mut dehydrated_devices = self
            .state
            .dehydrated_devices
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *dehydrated_devices = Some(owner);
        Ok(())
    }

    /// Password UIAA for a pending device delete. The password is a method
    /// argument, never a `Core::command` JSON field.
    pub async fn device_delete_password(
        &self,
        operation_id: u64,
        session_generation: u64,
        password: &str,
    ) -> Result<crate::app::devices::NativeDeviceDeleteResult, MatrixIpcError> {
        let owner = self.state.device_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-device-delete-password-no-session")
        })?;
        owner
            .authenticate_delete_password(operation_id, session_generation, password)
            .await
            .map_err(device_snapshot_owner_error)
    }

    /// Restore encryption backup. Recovery secret is a method argument,
    /// never a `Core::command` JSON field.
    pub async fn restore_backup(
        &self,
        recovery_secret: &str,
    ) -> Result<MatrixRestoreBackupResult, MatrixIpcError> {
        if recovery_secret.is_empty() {
            return Err(restore_backup_owner_error(
                "v-crypto.3-recovery-secret-empty",
            ));
        }
        let owner = self.state.device_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-restore-backup-no-session")
        })?;
        let result = owner
            .restore_backup(recovery_secret)
            .await
            .map_err(restore_backup_owner_error)?;
        if let Some(dehydrated) = self.state.dehydrated_devices_owner()? {
            let _ = dehydrated.try_start_with_secret(recovery_secret).await;
        }
        Ok(result)
    }

    /// Recovery mutations accept secrets only as typed arguments. Core's
    /// managed owner is shared by desktop and full-app Apple bindings.
    pub async fn backup_setup(
        &self,
        passphrase: &str,
    ) -> Result<crate::app::backup::NativeBackupOperationResult, MatrixIpcError> {
        if passphrase.is_empty() || passphrase.len() > 100_000 {
            return Err(MatrixIpcError::new(MatrixIpcErrorCategory::RecoveryFailure)
                .with_diagnostic("recovery-secret-invalid"));
        }
        let owner = self.state.device_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("recovery-no-session")
        })?;
        owner.backup_setup(passphrase).await.map_err(|diagnostic| {
            MatrixIpcError::new(MatrixIpcErrorCategory::RecoveryFailure).with_diagnostic(diagnostic)
        })
    }

    pub async fn backup_repair(
        &self,
        recovery_secret: &str,
    ) -> Result<crate::app::backup::NativeBackupOperationResult, MatrixIpcError> {
        if recovery_secret.is_empty() || recovery_secret.len() > 100_000 {
            return Err(MatrixIpcError::new(MatrixIpcErrorCategory::RecoveryFailure)
                .with_diagnostic("recovery-secret-invalid"));
        }
        let owner = self.state.device_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("recovery-no-session")
        })?;
        owner
            .backup_repair(recovery_secret)
            .await
            .map_err(|diagnostic| {
                MatrixIpcError::new(MatrixIpcErrorCategory::RecoveryFailure)
                    .with_diagnostic(diagnostic)
            })
    }

    pub async fn secret_storage_bootstrap(
        &self,
        passphrase: &str,
    ) -> Result<crate::app::secret_storage::SecretStorageSetup, MatrixIpcError> {
        if passphrase.is_empty() || passphrase.len() > 100_000 {
            return Err(MatrixIpcError::new(MatrixIpcErrorCategory::RecoveryFailure)
                .with_diagnostic("recovery-secret-invalid"));
        }
        let owner = self.state.device_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("recovery-no-session")
        })?;
        owner
            .secret_storage_bootstrap(passphrase)
            .await
            .map_err(|diagnostic| {
                MatrixIpcError::new(MatrixIpcErrorCategory::RecoveryFailure)
                    .with_diagnostic(diagnostic)
            })
    }

    pub async fn secret_storage_unlock(
        &self,
        recovery_secret: &str,
    ) -> Result<crate::app::secret_storage::NativeSecretStorageOperationResult, MatrixIpcError>
    {
        if recovery_secret.is_empty() || recovery_secret.len() > 100_000 {
            return Err(MatrixIpcError::new(MatrixIpcErrorCategory::RecoveryFailure)
                .with_diagnostic("recovery-secret-invalid"));
        }
        let owner = self.state.device_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("recovery-no-session")
        })?;
        owner
            .secret_storage_unlock(recovery_secret)
            .await
            .map_err(|diagnostic| {
                MatrixIpcError::new(MatrixIpcErrorCategory::RecoveryFailure)
                    .with_diagnostic(diagnostic)
            })
    }

    pub async fn secret_storage_reset(
        &self,
        passphrase: &str,
    ) -> Result<crate::app::secret_storage::SecretStorageSetup, MatrixIpcError> {
        if passphrase.is_empty() || passphrase.len() > 100_000 {
            return Err(MatrixIpcError::new(MatrixIpcErrorCategory::RecoveryFailure)
                .with_diagnostic("recovery-secret-invalid"));
        }
        let owner = self.state.device_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("recovery-no-session")
        })?;
        owner
            .secret_storage_reset(passphrase)
            .await
            .map_err(|diagnostic| {
                MatrixIpcError::new(MatrixIpcErrorCategory::RecoveryFailure)
                    .with_diagnostic(diagnostic)
            })
    }

    /// Password UIAA for a pending email 3PID attach. Password is a method
    /// argument, never a `Core::command` JSON field.
    pub async fn threepid_add_email_password(
        &self,
        password: &str,
    ) -> Result<MatrixThreepidAddResult, MatrixIpcError> {
        let owner = self.state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-threepid-add-email-password-no-session")
        })?;
        owner
            .add_threepid_email_password(password)
            .await
            .map_err(threepid_owner_error)
    }

    /// Own-avatar bytes upload. Bytes are a method argument, never a
    /// `Core::command` JSON field. Returns an `mxc://` URI only.
    pub async fn upload_avatar(
        &self,
        payload: Vec<u8>,
        mime_type: &str,
    ) -> Result<MatrixUploadAvatarResult, MatrixIpcError> {
        let owner = self.state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-upload-avatar-no-session")
        })?;
        owner
            .upload_avatar(payload, mime_type)
            .await
            .map_err(own_profile_owner_error)
    }

    /// Generic content upload. Bytes are a method argument, never a
    /// `Core::command` JSON field. Returns an `mxc://` URI only.
    pub async fn upload_content(
        &self,
        payload: Vec<u8>,
        mime_type: &str,
        filename: Option<&str>,
    ) -> Result<MatrixUploadMediaResult, MatrixIpcError> {
        let owner = self.state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-upload-content-no-session")
        })?;
        owner
            .upload_content(payload, mime_type, filename)
            .await
            .map_err(content_upload_owner_error)
    }

    /// Send a room attachment. Bytes are a method argument, never a
    /// `Core::command` JSON field.
    pub async fn send_room_attachment(
        &self,
        request: SendRoomAttachmentRequest,
    ) -> Result<MatrixSendRoomAttachmentResult, MatrixIpcError> {
        let owner = self.state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-send-room-attachment-no-session")
        })?;
        owner
            .send_room_attachment(request)
            .await
            .map_err(send_room_attachment_owner_error)
    }

    /// Original-file download for a plain `mxc://`. Bytes are a method
    /// return, never a `Core::command` JSON field. Timeline-media handles
    /// stay on `timeline_media_bytes`. Encrypted sources are not this API.
    pub async fn download_plain_media(&self, content_uri: &str) -> Result<Vec<u8>, MatrixIpcError> {
        let owner = self.state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-download-plain-media-no-session")
        })?;
        owner
            .download_plain_media(content_uri)
            .await
            .map_err(plain_media_owner_error)
    }

    /// Thumbnail download for a plain `mxc://`. Bytes are a method return,
    /// never a `Core::command` JSON field. Timeline-media handles stay on
    /// `timeline_media_bytes`. Encrypted sources are not this API.
    pub async fn thumbnail_plain_media(
        &self,
        content_uri: &str,
        width: u64,
        height: u64,
    ) -> Result<Vec<u8>, MatrixIpcError> {
        let owner = self.state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-thumbnail-plain-media-no-session")
        })?;
        owner
            .thumbnail_plain_media(content_uri, width, height)
            .await
            .map_err(plain_media_owner_error)
    }

    /// Register an HTTP pusher. Push key and gateway stay method arguments,
    /// never `Core::command` JSON fields.
    pub async fn register_http_pusher(
        &self,
        push_key: &str,
        app_id: &str,
        gateway_url: &str,
        app_display_name: &str,
        lang: &str,
    ) -> Result<MatrixHttpPusherWriteResult, MatrixIpcError> {
        let owner = self.state.http_pusher_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-register-http-pusher-no-session")
        })?;
        owner
            .register(push_key, app_id, gateway_url, app_display_name, lang)
            .await
            .map_err(http_pusher_owner_error)
    }

    /// Delete an HTTP pusher. Push key stays a method argument, never a
    /// `Core::command` JSON field.
    pub async fn delete_http_pusher(
        &self,
        push_key: &str,
        app_id: &str,
    ) -> Result<MatrixHttpPusherWriteResult, MatrixIpcError> {
        let owner = self.state.http_pusher_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-delete-http-pusher-no-session")
        })?;
        owner
            .delete(push_key, app_id)
            .await
            .map_err(http_pusher_owner_error)
    }

    /// Capture the currently authenticated owner for an account-bound HTTP
    /// pusher handle. The returned `Arc` remains bound to that owner's Matrix
    /// client even if a later session attach replaces Core's current owner.
    pub(crate) fn http_pusher_owner(&self) -> Result<Arc<NativeHttpPusherOwner>, MatrixIpcError> {
        self.state.http_pusher_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-bind-http-pusher-no-session")
        })
    }

    /// Install the live account-bound HTTP-pusher owner created by the shell
    /// after login or restore.
    pub fn attach_http_pusher(
        &self,
        owner: Arc<NativeHttpPusherOwner>,
    ) -> Result<(), MatrixIpcError> {
        let mut pusher = self
            .state
            .http_pusher
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *pusher = Some(owner);
        Ok(())
    }

    /// Install the live account-bound notification-decision owner created by
    /// the shell after login or restore. The shell keeps its own Arc for the
    /// session lifetime; Core drops its handle on logout.
    pub fn attach_notification_decisions(
        &self,
        owner: Arc<NativeNotificationDecisionOwner>,
    ) -> Result<(), MatrixIpcError> {
        let mut decisions = self
            .state
            .notification_decisions
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *decisions = Some(owner);
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

    /// Whether a SyncService owner is attached. Does not start sync.
    pub fn sync_service_attached(&self) -> bool {
        self.state.sync_owner().ok().flatten().is_some()
    }

    /// Whether the attached SyncService has been started. Idle/unconfigured
    /// owners count as not started. Does not start sync.
    pub fn sync_service_started(&self) -> bool {
        match self.state.sync_owner() {
            Ok(Some(owner)) => matches!(
                owner.observe().readiness,
                SyncReadiness::Running | SyncReadiness::Offline
            ),
            _ => false,
        }
    }

    /// Attached SyncService owner, if any. Does not start sync.
    pub(crate) fn attached_sync_owner(&self) -> Option<Arc<SyncServiceOwner>> {
        self.state.sync_owner().ok().flatten()
    }

    /// Attached timeline owner, if any. Does not open a view.
    pub(crate) fn attached_timeline_owner(&self) -> Option<Arc<NativeTimelineOwner>> {
        self.state.timeline_owner().ok().flatten()
    }

    /// Start the already-attached SyncService. Does not attach owners.
    /// Missing owner is `p4-s12-sync-not-attached`. Start failures stay
    /// `p4-s12-sync-start-failed` and never echo SDK text.
    pub async fn start_attached_sync(&self) -> Result<SyncReadinessSnapshot, &'static str> {
        let owner = self
            .state
            .sync_owner()
            .map_err(|_| "p4-s12-sync-start-failed")?
            .ok_or("p4-s12-sync-not-attached")?;
        owner.start().await.map_err(|_| "p4-s12-sync-start-failed")
    }

    /// Stop the already-attached SyncService and wait for the SDK stop call.
    /// The owner and retained Client remain attached for foreground restart.
    pub async fn stop_attached_sync(&self) -> Result<SyncReadinessSnapshot, &'static str> {
        let owner = self
            .state
            .sync_owner()
            .map_err(|_| "p4-s12-sync-stop-failed")?
            .ok_or("p4-s12-sync-not-attached")?;
        owner.stop().await.map_err(|_| "p4-s12-sync-stop-failed")
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
        .register("matrix_invites_block_sender", matrix_invites_block_sender)
        .expect("built-in matrix_invites_block_sender must remain in the command census");
    registry
        .register("matrix_invites_decline", matrix_invites_decline)
        .expect("built-in matrix_invites_decline must remain in the command census");
    registry
        .register("matrix_invites_report_spam", matrix_invites_report_spam)
        .expect("built-in matrix_invites_report_spam must remain in the command census");
    registry
        .register("matrix_inbox_notifications", matrix_inbox_notifications)
        .expect("built-in matrix_inbox_notifications must remain in the command census");
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
        .register(
            "matrix_enable_room_encrypted_state",
            matrix_enable_room_encrypted_state,
        )
        .expect("built-in matrix_enable_room_encrypted_state must remain in the command census");
    registry
        .register("matrix_media_config", matrix_media_config)
        .expect("built-in matrix_media_config must remain in the command census");
    registry
        .register("matrix_media_preview", matrix_media_preview)
        .expect("built-in matrix_media_preview must remain in the command census");
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
        .register("matrix_presence_set", matrix_presence_set)
        .expect("built-in matrix_presence_set must remain in the command census");
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
        .register(
            "matrix_rtc_transports_refresh",
            matrix_rtc_transports_refresh,
        )
        .expect("built-in matrix_rtc_transports_refresh must remain in the command census");
    registry
        .register(
            "matrix_rtc_transports_snapshot",
            matrix_rtc_transports_snapshot,
        )
        .expect("built-in matrix_rtc_transports_snapshot must remain in the command census");
    registry
        .register("matrix_widgets_list", matrix_widgets_list)
        .expect("built-in matrix_widgets_list must remain in the command census");
    registry
        .register("matrix_widget_open", matrix_widget_open)
        .expect("built-in matrix_widget_open must remain in the command census");
    registry
        .register("matrix_widget_close", matrix_widget_close)
        .expect("built-in matrix_widget_close must remain in the command census");
    registry
        .register("matrix_widget_post", matrix_widget_post)
        .expect("built-in matrix_widget_post must remain in the command census");
    registry
        .register("matrix_widget_subscribe", matrix_widget_subscribe)
        .expect("built-in matrix_widget_subscribe must remain in the command census");
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
        .register("matrix_room_set_join_rule", matrix_room_set_join_rule)
        .expect("built-in matrix_room_set_join_rule must remain in the command census");
    registry
        .register("matrix_room_leave", matrix_room_leave)
        .expect("built-in matrix_room_leave must remain in the command census");
    registry
        .register("matrix_room_join", matrix_room_join)
        .expect("built-in matrix_room_join must remain in the command census");
    registry
        .register("matrix_room_set_favorite", matrix_room_set_favorite)
        .expect("built-in matrix_room_set_favorite must remain in the command census");
    registry
        .register("matrix_room_set_read_state", matrix_room_set_read_state)
        .expect("built-in matrix_room_set_read_state must remain in the command census");
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
        .register("matrix_room_retention", matrix_room_retention)
        .expect("built-in matrix_room_retention must remain in the command census");
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
        .register("matrix_send_state_event", matrix_send_state_event)
        .expect("built-in matrix_send_state_event must remain in the command census");
    registry
        .register(
            "matrix_set_encrypted_state_events_setting",
            matrix_set_encrypted_state_events_setting,
        )
        .expect(
            "built-in matrix_set_encrypted_state_events_setting must remain in the command census",
        );
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
        .register("matrix_get_own_profile", matrix_get_own_profile)
        .expect("built-in matrix_get_own_profile must remain in the command census");
    registry
        .register(
            "matrix_ignored_users_snapshot",
            matrix_ignored_users_snapshot,
        )
        .expect("built-in matrix_ignored_users_snapshot must remain in the command census");
    registry
        .register("matrix_ignored_users_ignore", matrix_ignored_users_ignore)
        .expect("built-in matrix_ignored_users_ignore must remain in the command census");
    registry
        .register(
            "matrix_ignored_users_unignore",
            matrix_ignored_users_unignore,
        )
        .expect("built-in matrix_ignored_users_unignore must remain in the command census");
    registry
        .register("matrix_user_directory_search", matrix_user_directory_search)
        .expect("built-in matrix_user_directory_search must remain in the command census");
    registry
        .register("matrix_user_status_clear", matrix_user_status_clear)
        .expect("built-in matrix_user_status_clear must remain in the command census");
    registry
        .register("matrix_user_status_set", matrix_user_status_set)
        .expect("built-in matrix_user_status_set must remain in the command census");
    registry
        .register("matrix_user_status_snapshot", matrix_user_status_snapshot)
        .expect("built-in matrix_user_status_snapshot must remain in the command census");
    registry
        .register("matrix_message_search", matrix_message_search)
        .expect("built-in matrix_message_search must remain in the command census");
    registry
        .register("matrix_push_rules_snapshot", matrix_push_rules_snapshot)
        .expect("built-in matrix_push_rules_snapshot must remain in the command census");
    registry
        .register(
            "matrix_push_rules_set_default",
            matrix_push_rules_set_default,
        )
        .expect("built-in matrix_push_rules_set_default must remain in the command census");
    registry
        .register(
            "matrix_push_rules_set_mention",
            matrix_push_rules_set_mention,
        )
        .expect("built-in matrix_push_rules_set_mention must remain in the command census");
    registry
        .register(
            "matrix_push_rules_add_keyword",
            matrix_push_rules_add_keyword,
        )
        .expect("built-in matrix_push_rules_add_keyword must remain in the command census");
    registry
        .register(
            "matrix_push_rules_remove_keyword",
            matrix_push_rules_remove_keyword,
        )
        .expect("built-in matrix_push_rules_remove_keyword must remain in the command census");
    registry
        .register(
            "matrix_room_notification_snapshot",
            matrix_room_notification_snapshot,
        )
        .expect("built-in matrix_room_notification_snapshot must remain in the command census");
    registry
        .register("matrix_room_notification_set", matrix_room_notification_set)
        .expect("built-in matrix_room_notification_set must remain in the command census");
    registry
        .register(
            "matrix_room_notifications_snapshot",
            matrix_room_notifications_snapshot,
        )
        .expect("built-in matrix_room_notifications_snapshot must remain in the command census");
    registry
        .register("matrix_notification_decide", matrix_notification_decide)
        .expect("built-in matrix_notification_decide must remain in the command census");
    registry
        .register("matrix_notification_dismiss", matrix_notification_dismiss)
        .expect("built-in matrix_notification_dismiss must remain in the command census");
    registry
        .register(
            "matrix_notification_focus_set",
            matrix_notification_focus_set,
        )
        .expect("built-in matrix_notification_focus_set must remain in the command census");
    registry
        .register(
            "matrix_notification_pending_snapshot",
            matrix_notification_pending_snapshot,
        )
        .expect("built-in matrix_notification_pending_snapshot must remain in the command census");
    registry
        .register("matrix_threepid_snapshot", matrix_threepid_snapshot)
        .expect("built-in matrix_threepid_snapshot must remain in the command census");
    registry
        .register("matrix_threepid_delete", matrix_threepid_delete)
        .expect("built-in matrix_threepid_delete must remain in the command census");
    registry
        .register(
            "matrix_threepid_request_email_token",
            matrix_threepid_request_email_token,
        )
        .expect("built-in matrix_threepid_request_email_token must remain in the command census");
    registry
        .register("matrix_threepid_add_email", matrix_threepid_add_email)
        .expect("built-in matrix_threepid_add_email must remain in the command census");
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
        .register("matrix_agent_approval_decide", matrix_agent_approval_decide)
        .expect("built-in matrix_agent_approval_decide must remain in the command census");
    registry
        .register(
            "matrix_agent_approval_history_snapshot",
            matrix_agent_approval_history_snapshot,
        )
        .expect(
            "built-in matrix_agent_approval_history_snapshot must remain in the command census",
        );
    registry
        .register("matrix_agent_approvals_list", matrix_agent_approvals_list)
        .expect("built-in matrix_agent_approvals_list must remain in the command census");
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
        .register("matrix_timeline_snapshot", matrix_timeline_snapshot)
        .expect("built-in matrix_timeline_snapshot must remain in the command census");
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
        .register(
            "matrix_timeline_timestamp_to_event",
            matrix_timeline_timestamp_to_event,
        )
        .expect("built-in matrix_timeline_timestamp_to_event must remain in the command census");
    registry
        .register("matrix_timeline_paginate", matrix_timeline_paginate)
        .expect("built-in matrix_timeline_paginate must remain in the command census");
    registry
        .register("matrix_timeline_follow_live", matrix_timeline_follow_live)
        .expect("built-in matrix_timeline_follow_live must remain in the command census");
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
        .register("matrix_pinned_events", matrix_pinned_events)
        .expect("built-in matrix_pinned_events must remain in the command census");
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
        .register("matrix_thread_list", matrix_thread_list)
        .expect("built-in matrix_thread_list must remain in the command census");
    registry
}

fn core_state_error(diagnostic_id: &'static str) -> MatrixIpcError {
    MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant).with_diagnostic(diagnostic_id)
}

#[cfg(test)]
mod tests;
