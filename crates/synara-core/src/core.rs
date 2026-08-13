//! Shared native-core entry points (P2 foundation).
//!
//! `Core` owns safe session projection/lifecycle plus the transport command
//! registry. It intentionally has no Tauri dependency; P2 command groups add
//! handlers, P3 makes the desktop shell a thin `Core::command` registrar.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::app::account_data::{
    NativeGlobalImagePacksSnapshot, NativeImagePackOwner, NativeRoomImagePacksSnapshot,
    NativeUserImagePackSnapshot,
};
use crate::app::auth::{
    discover_login_flows, login_flows_response, probe_register_flows, AuthError,
    HttpLoginFlowTransport, HttpRegisterFlowTransport, MatrixLoginFlowsResponse,
    RegisterFlowsProbe,
};
use crate::app::devices::{NativeDeviceOwner, NativeDeviceSnapshot};
use crate::app::presence::{
    NativePresenceOwner, NativePresenceSnapshotResult, NativePresenceSubscription,
};
use crate::app::room_profile::{MatrixRoomJoinRuleSnapshot, NativeRoomJoinRuleOwner};
use crate::app::sync::{SyncReadinessSnapshot, SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID};
use crate::app::typing::{NativeTypingOwner, NativeTypingSnapshot};
use crate::app::verification::{NativeVerificationInbox, NativeVerificationOwner};
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixTypingSetRequest {
    room_id: String,
    typing: bool,
}

/// Exact React/Tauri envelope payload for `matrix_room_join_rule_snapshot`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixRoomJoinRuleSnapshotRequest {
    room_id: String,
    session_generation: u64,
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
        .register("matrix_secret_storage_status", matrix_secret_storage_status)
        .expect("built-in matrix_secret_storage_status must remain in the command census");
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
        .register("matrix_verification_list", matrix_verification_list)
        .expect("built-in matrix_verification_list must remain in the command census");
    registry
        .register("matrix_device_snapshot", matrix_device_snapshot)
        .expect("built-in matrix_device_snapshot must remain in the command census");
    registry
        .register(
            "matrix_room_join_rule_snapshot",
            matrix_room_join_rule_snapshot,
        )
        .expect("built-in matrix_room_join_rule_snapshot must remain in the command census");
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
        .register("matrix_set_room_image_pack", matrix_set_room_image_pack)
        .expect("built-in matrix_set_room_image_pack must remain in the command census");
    registry
        .register("matrix_typing_set", matrix_typing_set)
        .expect("built-in matrix_typing_set must remain in the command census");
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
        "v-crypto.7-device-owner-user-missing"
        | "v-crypto.7-device-snapshot-current-missing"
        | "v-crypto.7-device-snapshot-user-missing" => MatrixIpcErrorCategory::Forbidden,
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
                "matrix_cross_signing_status",
                "matrix_crypto_status",
                "matrix_device_snapshot",
                "matrix_get_global_image_packs",
                "matrix_get_room_image_packs",
                "matrix_get_user_image_pack",
                "matrix_login_flows",
                "matrix_media_config",
                "matrix_presence_snapshot",
                "matrix_presence_subscribe",
                "matrix_presence_unsubscribe",
                "matrix_register_flows",
                "matrix_room_join_rule_snapshot",
                "matrix_secret_storage_status",
                "matrix_session_snapshot",
                "matrix_set_global_image_packs",
                "matrix_set_room_image_pack",
                "matrix_set_user_image_pack",
                "matrix_sync_status",
                "matrix_typing_set",
                "matrix_typing_snapshot",
                "matrix_verification_list",
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
