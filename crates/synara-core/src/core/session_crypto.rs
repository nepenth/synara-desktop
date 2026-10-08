//! Core command adapters for session crypto.

use super::*;

/// React-compatible payload for `matrix_session_snapshot`.
///
/// This deliberately selects only the fields returned by the desktop command,
/// rather than serializing the broader safe session projection wholesale.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
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

impl From<Option<SessionSnapshot>> for MatrixSessionSnapshot {
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
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatrixCrossSigningState {
    Unavailable,
    NotSetUp,
    Partial,
    Ready,
}

/// Exact React/Tauri payload for `matrix_crypto_status`.
///
/// Keep this separate from the Platform projection: this type owns the wire
/// field names and is constructed only after Core validates the closed input.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixCryptoStatus {
    pub session_generation: u64,
    pub encryption_enabled: bool,
    pub cross_signing_state: MatrixCrossSigningState,
}

impl MatrixCryptoStatus {
    pub(super) fn from_platform(status: PlatformCryptoStatus) -> Result<Self, MatrixIpcError> {
        let cross_signing_state = match status.cross_signing_state() {
            PlatformCryptoCrossSigningState::Unavailable => MatrixCrossSigningState::Unavailable,
            PlatformCryptoCrossSigningState::NotSetUp => MatrixCrossSigningState::NotSetUp,
            PlatformCryptoCrossSigningState::Partial => MatrixCrossSigningState::Partial,
            PlatformCryptoCrossSigningState::Ready => MatrixCrossSigningState::Ready,
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

    pub fn is_valid(&self) -> bool {
        matches!(
            (self.encryption_enabled, self.cross_signing_state),
            (false, MatrixCrossSigningState::Unavailable)
                | (true, MatrixCrossSigningState::NotSetUp)
                | (true, MatrixCrossSigningState::Partial)
                | (true, MatrixCrossSigningState::Ready)
        )
    }
}

/// Fixed public readiness vocabulary for `matrix_cross_signing_status`.
///
/// `recovery_required` remains a read-only legacy status label. This transport
/// command performs no setup, recovery, or verification action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatrixCrossSigningReadinessResponse {
    Unavailable,
    SetupRequired,
    RecoveryRequired,
    VerificationRequired,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatrixCrossSigningKeyPublicationResponse {
    Missing,
    Published,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatrixCrossSigningPrivateIdentityResponse {
    Missing,
    Partial,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatrixOwnIdentityVerificationResponse {
    Missing,
    Unverified,
    Verified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatrixCrossSigningBootstrapResponse {
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
pub struct MatrixCrossSigningStatusResponse {
    pub session_generation: u64,
    pub readiness: MatrixCrossSigningReadinessResponse,
    pub master_signing: MatrixCrossSigningKeyPublicationResponse,
    pub self_signing: MatrixCrossSigningKeyPublicationResponse,
    pub user_signing: MatrixCrossSigningKeyPublicationResponse,
    pub private_identity: MatrixCrossSigningPrivateIdentityResponse,
    pub own_identity_verification: MatrixOwnIdentityVerificationResponse,
    pub bootstrap: MatrixCrossSigningBootstrapResponse,
}

impl MatrixCrossSigningStatusResponse {
    pub(super) fn from_platform(
        status: PlatformCrossSigningStatus,
    ) -> Result<Self, MatrixIpcError> {
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
    pub(super) fn is_valid(&self) -> bool {
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
pub enum MatrixSecretStorageStateResponse {
    Unavailable,
    NotSetUp,
    Locked,
    Ready,
}

/// Fixed public action vocabulary for `matrix_secret_storage_status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatrixSecretStorageActionResponse {
    BootstrapRequired,
    UnlockRequired,
    None,
}

/// Exact React/Tauri DTO for `matrix_secret_storage_status`.
///
/// Core reconstructs this legacy object only from the platform's closed,
/// scalar projection. The `missingSecrets` list is public wire shape; its
/// canonical labels and ordering are owned here, not supplied by the shell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixSecretStorageStatusResponse {
    pub session_generation: u64,
    pub state: MatrixSecretStorageStateResponse,
    pub exists: bool,
    pub unlocked: bool,
    pub default_key_set: bool,
    pub passphrase_configured: bool,
    pub bootstrap_ready: bool,
    pub missing_secrets: Vec<MatrixMissingSecretResponse>,
    pub action: MatrixSecretStorageActionResponse,
}

impl From<crate::app::secret_storage::NativeSecretStorageStatus>
    for MatrixSecretStorageStatusResponse
{
    /// The attached owner's projection has the same closed vocabulary.
    fn from(status: crate::app::secret_storage::NativeSecretStorageStatus) -> Self {
        use crate::app::secret_storage::{
            NativeMissingSecret, NativeSecretStorageAction, NativeSecretStorageState,
        };
        Self {
            session_generation: status.session_generation,
            state: match status.state {
                NativeSecretStorageState::Unavailable => {
                    MatrixSecretStorageStateResponse::Unavailable
                }
                NativeSecretStorageState::NotSetUp => MatrixSecretStorageStateResponse::NotSetUp,
                NativeSecretStorageState::Locked => MatrixSecretStorageStateResponse::Locked,
                NativeSecretStorageState::Ready => MatrixSecretStorageStateResponse::Ready,
            },
            exists: status.exists,
            unlocked: status.unlocked,
            default_key_set: status.default_key_set,
            passphrase_configured: status.passphrase_configured,
            bootstrap_ready: status.bootstrap_ready,
            missing_secrets: status
                .missing_secrets
                .into_iter()
                .map(|secret| match secret {
                    NativeMissingSecret::CrossSigningMaster => {
                        MatrixMissingSecretResponse::CrossSigningMaster
                    }
                    NativeMissingSecret::CrossSigningSelfSigning => {
                        MatrixMissingSecretResponse::CrossSigningSelfSigning
                    }
                    NativeMissingSecret::CrossSigningUserSigning => {
                        MatrixMissingSecretResponse::CrossSigningUserSigning
                    }
                    NativeMissingSecret::EncryptionBackup => {
                        MatrixMissingSecretResponse::EncryptionBackup
                    }
                })
                .collect(),
            action: match status.action {
                NativeSecretStorageAction::BootstrapRequired => {
                    MatrixSecretStorageActionResponse::BootstrapRequired
                }
                NativeSecretStorageAction::UnlockRequired => {
                    MatrixSecretStorageActionResponse::UnlockRequired
                }
                NativeSecretStorageAction::None => MatrixSecretStorageActionResponse::None,
            },
        }
    }
}

impl MatrixSecretStorageStatusResponse {
    pub(super) fn from_platform(
        status: PlatformSecretStorageStatus,
    ) -> Result<Self, MatrixIpcError> {
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

    pub(super) fn is_valid(&self) -> bool {
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

/// Exact React/Tauri envelope payload for `matrix_login_flows`.
///
/// The renderer sends the camel-case `homeserverUrl` key; unknown keys are
/// rejected so accidental credential fields do not cross this boundary.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixLoginFlowsRequest {
    pub homeserver_url: String,
}

/// Exact React/Tauri envelope payload for `matrix_device_rename`.
///
/// The renderer sends camel-case `deviceId` and `displayName`; unknown keys
/// are rejected so this write cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixDeviceRenameRequest {
    pub device_id: String,
    pub display_name: String,
}

/// Exact React/Tauri envelope payload for `matrix_device_delete_start`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixDeviceDeleteStartRequest {
    pub device_ids: Vec<String>,
}

/// Exact React/Tauri envelope payload for `matrix_device_delete_cancel`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixDeviceDeleteCancelRequest {
    pub operation_id: u64,
    pub session_generation: u64,
}

/// Exact React/Tauri envelope payload for `matrix_verification_accept`.
///
/// The renderer sends the camel-case `flowId` key; unknown keys are rejected
/// so this write cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixVerificationAcceptRequest {
    pub flow_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_verification_begin_sas`.
///
/// Shares accept's camel-case `flowId` key. Unknown keys are rejected so
/// this write cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixVerificationBeginSasRequest {
    pub flow_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_verification_cancel`.
///
/// Shares accept's camel-case `flowId` key. Unknown keys are rejected so
/// this write cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixVerificationCancelRequest {
    pub flow_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_verification_confirm`.
///
/// Shares accept's camel-case `flowId` key. Unknown keys are rejected so
/// this write cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixVerificationConfirmRequest {
    pub flow_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_verification_dismiss`.
///
/// Shares accept's camel-case `flowId` key. Unknown keys are rejected so
/// this write cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixVerificationDismissRequest {
    pub flow_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_verification_mismatch`.
///
/// Shares accept's camel-case `flowId` key. Unknown keys are rejected so
/// this write cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixVerificationMismatchRequest {
    pub flow_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_verification_start`.
///
/// The renderer sends optional camel-case `deviceId`. Unknown keys are
/// rejected so this write cannot grow extra identity or session fields.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixVerificationStartRequest {
    #[serde(default)]
    pub device_id: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_register_flows`.
///
/// This read-only probe accepts exactly the existing camel-case homeserver
/// field and rejects all credential or UIAA-continuation fields at the core
/// boundary.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRegisterFlowsRequest {
    pub homeserver_url: String,
}

/// Map live presence-owner diagnostics onto closed Core transport categories.
/// Preserve the owner diagnostic id so the desktop bridge can restore the
/// established Tauri error shape without leaking user ids or status text.
/// Typed `matrix_verification_list`.
pub(super) async fn verification_list(
    state: &Arc<CoreState>,
) -> Result<NativeVerificationInbox, MatrixIpcError> {
    let owner = state.verification_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-verification-list-no-session")
    })?;
    let inbox: NativeVerificationInbox = owner.list().await;
    Ok(inbox)
}

#[cfg(test)]
pub(super) fn matrix_verification_list(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-verification-list-invalid-payload"));
        }
        let response = verification_list(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-verification-list-serialization-failed"))
    })
}

/// Typed `matrix_verification_accept`.
pub(super) async fn verification_accept(
    state: &Arc<CoreState>,
    payload: MatrixVerificationAcceptRequest,
) -> Result<NativeVerificationRequest, MatrixIpcError> {
    let owner = state.verification_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-verification-accept-no-session")
    })?;
    let request: NativeVerificationRequest = owner
        .accept(&payload.flow_id)
        .await
        .map_err(verification_accept_owner_error)?;
    Ok(request)
}

#[cfg(test)]
pub(super) fn matrix_verification_accept(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixVerificationAcceptRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-verification-accept-invalid-payload"))?;
        let response = verification_accept(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-verification-accept-serialization-failed"))
    })
}

pub(super) fn verification_accept_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
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

/// Typed `matrix_verification_begin_sas`.
pub(super) async fn verification_begin_sas(
    state: &Arc<CoreState>,
    payload: MatrixVerificationBeginSasRequest,
) -> Result<NativeVerificationRequest, MatrixIpcError> {
    let owner = state.verification_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-verification-begin-sas-no-session")
    })?;
    let request: NativeVerificationRequest = owner
        .begin_sas(&payload.flow_id)
        .await
        .map_err(verification_accept_owner_error)?;
    Ok(request)
}

#[cfg(test)]
pub(super) fn matrix_verification_begin_sas(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixVerificationBeginSasRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-verification-begin-sas-invalid-payload"))?;
        let response = verification_begin_sas(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-verification-begin-sas-serialization-failed"))
    })
}

/// Typed `matrix_verification_cancel`.
pub(super) async fn verification_cancel(
    state: &Arc<CoreState>,
    payload: MatrixVerificationCancelRequest,
) -> Result<NativeVerificationRequest, MatrixIpcError> {
    let owner = state.verification_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-verification-cancel-no-session")
    })?;
    let request: NativeVerificationRequest = owner
        .cancel(&payload.flow_id)
        .await
        .map_err(verification_accept_owner_error)?;
    Ok(request)
}

#[cfg(test)]
pub(super) fn matrix_verification_cancel(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixVerificationCancelRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-verification-cancel-invalid-payload"))?;
        let response = verification_cancel(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-verification-cancel-serialization-failed"))
    })
}

/// Typed `matrix_verification_confirm`.
pub(super) async fn verification_confirm(
    state: &Arc<CoreState>,
    payload: MatrixVerificationConfirmRequest,
) -> Result<NativeVerificationRequest, MatrixIpcError> {
    let owner = state.verification_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-verification-confirm-no-session")
    })?;
    let request: NativeVerificationRequest = owner
        .confirm(&payload.flow_id)
        .await
        .map_err(verification_accept_owner_error)?;
    Ok(request)
}

#[cfg(test)]
pub(super) fn matrix_verification_confirm(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixVerificationConfirmRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-verification-confirm-invalid-payload"))?;
        let response = verification_confirm(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-verification-confirm-serialization-failed"))
    })
}

/// Typed `matrix_verification_dismiss`.
pub(super) async fn verification_dismiss(
    state: &Arc<CoreState>,
    payload: MatrixVerificationDismissRequest,
) -> Result<(), MatrixIpcError> {
    let owner = state.verification_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-verification-dismiss-no-session")
    })?;
    owner
        .dismiss(&payload.flow_id)
        .await
        .map_err(verification_accept_owner_error)?;
    Ok(())
}

#[cfg(test)]
pub(super) fn matrix_verification_dismiss(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixVerificationDismissRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-verification-dismiss-invalid-payload"))?;
        verification_dismiss(&state, payload).await?;
        Ok(serde_json::Value::Null)
    })
}

/// Typed `matrix_verification_mismatch`.
pub(super) async fn verification_mismatch(
    state: &Arc<CoreState>,
    payload: MatrixVerificationMismatchRequest,
) -> Result<NativeVerificationRequest, MatrixIpcError> {
    let owner = state.verification_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-verification-mismatch-no-session")
    })?;
    let request: NativeVerificationRequest = owner
        .mismatch(&payload.flow_id)
        .await
        .map_err(verification_accept_owner_error)?;
    Ok(request)
}

#[cfg(test)]
pub(super) fn matrix_verification_mismatch(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixVerificationMismatchRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-verification-mismatch-invalid-payload"))?;
        let response = verification_mismatch(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-verification-mismatch-serialization-failed"))
    })
}

/// Typed `matrix_verification_start`.
pub(super) async fn verification_start(
    state: &Arc<CoreState>,
    payload: MatrixVerificationStartRequest,
) -> Result<NativeVerificationRequest, MatrixIpcError> {
    let owner = state.verification_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-verification-start-no-session")
    })?;
    let request: NativeVerificationRequest = owner
        .start(payload.device_id)
        .await
        .map_err(verification_accept_owner_error)?;
    Ok(request)
}

#[cfg(test)]
pub(super) fn matrix_verification_start(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixVerificationStartRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-verification-start-invalid-payload"))?;
        let response = verification_start(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-verification-start-serialization-failed"))
    })
}

/// Typed `matrix_backup_status`.
pub(super) async fn backup_status(
    state: &Arc<CoreState>,
) -> Result<NativeBackupStatus, MatrixIpcError> {
    let owner = state.device_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-backup-status-no-session")
    })?;
    let snapshot: NativeBackupStatus = owner
        .backup_status()
        .await
        .map_err(backup_status_owner_error)?;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_backup_status(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-backup-status-invalid-payload"));
        }
        let response = backup_status(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-backup-status-serialization-failed"))
    })
}

pub(super) fn backup_status_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    MatrixIpcError::new(MatrixIpcErrorCategory::Unknown).with_diagnostic(diagnostic_id)
}

pub(super) fn restore_backup_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-crypto.3-recovery-secret-empty" => MatrixIpcErrorCategory::SdkInvariant,
        "v-crypto.3-restore-rejected" | "v-crypto.3-restore-incomplete" => {
            MatrixIpcErrorCategory::RecoveryFailure
        }
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

/// Typed `matrix_room_key_transfer_status`.
pub(super) async fn room_key_transfer_status(
    state: &Arc<CoreState>,
) -> Result<NativeRoomKeyTransferStatus, MatrixIpcError> {
    let owner = state.device_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-key-transfer-status-no-session")
    })?;
    let snapshot: NativeRoomKeyTransferStatus = owner.room_key_status().await;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_room_key_transfer_status(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error(
                "p2-room-key-transfer-status-invalid-payload",
            ));
        }
        let response = room_key_transfer_status(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-key-transfer-status-serialization-failed"))
    })
}

/// Typed `matrix_cross_signing_setup`.
pub(super) async fn cross_signing_setup(
    state: &Arc<CoreState>,
) -> Result<NativeCrossSigningSetupResult, MatrixIpcError> {
    let owner = state.device_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-cross-signing-setup-no-session")
    })?;
    let result: NativeCrossSigningSetupResult = owner
        .cross_signing_setup()
        .await
        .map_err(cross_signing_setup_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_cross_signing_setup(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-cross-signing-setup-invalid-payload"));
        }
        let response = cross_signing_setup(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-cross-signing-setup-serialization-failed"))
    })
}

pub(super) fn cross_signing_setup_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-crypto.2-cross-signing-auth-unsupported" => MatrixIpcErrorCategory::Forbidden,
        "v-crypto.2-cross-signing-user-missing" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

/// Typed `matrix_device_snapshot`.
pub(super) async fn device_snapshot(
    state: &Arc<CoreState>,
) -> Result<NativeDeviceSnapshot, MatrixIpcError> {
    let owner = state.device_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-device-snapshot-no-session")
    })?;
    let snapshot: NativeDeviceSnapshot = owner
        .snapshot()
        .await
        .map_err(device_snapshot_owner_error)?;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_device_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-device-snapshot-invalid-payload"));
        }
        let response = device_snapshot(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-device-snapshot-serialization-failed"))
    })
}

/// Typed `matrix_device_rename`.
pub(super) async fn device_rename(
    state: &Arc<CoreState>,
    payload: MatrixDeviceRenameRequest,
) -> Result<NativeDeviceSnapshot, MatrixIpcError> {
    let owner = state.device_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-device-rename-no-session")
    })?;
    let snapshot: NativeDeviceSnapshot = owner
        .rename(&payload.device_id, &payload.display_name)
        .await
        .map_err(device_snapshot_owner_error)?;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_device_rename(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixDeviceRenameRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-device-rename-invalid-payload"))?;
        let response = device_rename(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-device-rename-serialization-failed"))
    })
}

/// Typed `matrix_device_delete_start`.
pub(super) async fn device_delete_start(
    state: &Arc<CoreState>,
    payload: MatrixDeviceDeleteStartRequest,
) -> Result<NativeDeviceDeleteResult, MatrixIpcError> {
    let owner = state.device_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-device-delete-start-no-session")
    })?;
    let result: NativeDeviceDeleteResult = owner
        .delete_start(payload.device_ids)
        .await
        .map_err(device_snapshot_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_device_delete_start(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixDeviceDeleteStartRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-device-delete-start-invalid-payload"))?;
        let response = device_delete_start(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-device-delete-start-serialization-failed"))
    })
}

/// Typed `matrix_device_delete_cancel`.
pub(super) async fn device_delete_cancel(
    state: &Arc<CoreState>,
    payload: MatrixDeviceDeleteCancelRequest,
) -> Result<(), MatrixIpcError> {
    let owner = state.device_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-device-delete-cancel-no-session")
    })?;
    owner
        .delete_cancel(payload.operation_id, payload.session_generation)
        .map_err(device_snapshot_owner_error)?;
    Ok(())
}

#[cfg(test)]
pub(super) fn matrix_device_delete_cancel(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixDeviceDeleteCancelRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-device-delete-cancel-invalid-payload"))?;
        device_delete_cancel(&state, payload).await?;
        Ok(serde_json::Value::Null)
    })
}

pub(super) fn device_snapshot_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-crypto.7-device-rename-empty"
        | "v-crypto.7-device-delete-selection-empty"
        | "v-crypto.7-device-delete-selection-invalid"
        | "v-crypto.7-device-delete-not-pending"
        | "v-crypto.7-device-delete-operation-mismatch"
        | "v-crypto.7-device-delete-password-empty" => MatrixIpcErrorCategory::SdkInvariant,
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

/// Typed `matrix_session_snapshot`.
#[cfg(test)]
pub(super) async fn session_snapshot(
    state: &Arc<CoreState>,
) -> Result<MatrixSessionSnapshot, MatrixIpcError> {
    let response = state.public_session_snapshot()?;
    Ok(response)
}

#[cfg(test)]
pub(super) fn matrix_session_snapshot(
    state: Arc<CoreState>,
    _request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let response = session_snapshot(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-session-snapshot-serialization-failed"))
    })
}

impl CoreState {
    /// Typed `matrix_session_snapshot`: the exact public session observation.
    pub(super) fn public_session_snapshot(&self) -> Result<MatrixSessionSnapshot, MatrixIpcError> {
        Ok(MatrixSessionSnapshot::from(self.session_snapshot()?))
    }

    /// Typed `matrix_sync_status`: Platform sync readiness plus Core's
    /// session-level command gate.
    pub(super) async fn public_sync_status(&self) -> Result<SyncReadinessSnapshot, MatrixIpcError> {
        let status = self
            .platform()
            .sync_status()
            .await
            // Platform status errors are closed enums, and Core still exposes
            // only its static command error through this public observation.
            .map_err(|_| core_state_error("p2-sync-status-platform-unavailable"))?;
        let mut snapshot = public_sync_status(status)?;
        // Session-level Core owner, not "a room timeline view is open".
        // Commands such as matrix_timeline_snapshot consult this same slot.
        let timeline_owner_attached = matches!(self.timeline_owner(), Ok(Some(_)));
        snapshot.command_gate = crate::app::sync::CommandGate::for_installed_session(
            timeline_owner_attached,
            snapshot.failure_diagnostic_id,
        );
        Ok(snapshot)
    }

    /// Typed `matrix_crypto_status`: the validated public crypto observation.
    pub(super) async fn public_crypto_status(&self) -> Result<MatrixCryptoStatus, MatrixIpcError> {
        let status = self
            .platform()
            .crypto_status()
            .await
            // A Platform crypto error is a closed enum. Never attach a shell
            // error, SDK diagnostic, identity, or key to the public command.
            .map_err(|_| core_state_error("p2-crypto-status-platform-unavailable"))?;
        MatrixCryptoStatus::from_platform(status)
    }
}

/// Reconstruct the public status DTO from the string-free Platform projection.
///
/// This is the only Platform-to-public mapping: Core constructs one of two
/// fixed diagnostics from the closed failure enum, then validates
/// the full DTO contract before it can be serialized.
pub(super) fn public_sync_status(
    status: PlatformSyncStatus,
) -> Result<SyncReadinessSnapshot, MatrixIpcError> {
    let failure_diagnostic_id = status.failure().map(|failure| match failure {
        PlatformSyncFailure::SyncService => SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID,
        PlatformSyncFailure::AuthenticationRejected => {
            crate::app::sync::SYNC_AUTHENTICATION_FAILURE_DIAGNOSTIC_ID
        }
    });
    let snapshot = SyncReadinessSnapshot {
        readiness: status.readiness(),
        session_generation: status.session_generation(),
        offline_mode_enabled: status.offline_mode_enabled(),
        failure_diagnostic_id,
        sliding_sync_capable: status.sliding_sync_capable(),
        command_gate: crate::app::sync::CommandGate::Open,
    };
    snapshot
        .is_valid_public_sync_status()
        .then_some(snapshot)
        .ok_or_else(|| core_state_error("p2-sync-status-invalid-platform-projection"))
}

/// `matrix_sync_status` is deliberately a payload-free observation. Core owns
/// its registry entry and exact wire serialization; the Platform remains the
/// sole owner of the live SDK client from which it reads the safe projection.
/// Typed `matrix_sync_status`.
#[cfg(test)]
pub(super) async fn sync_status(
    state: &Arc<CoreState>,
) -> Result<SyncReadinessSnapshot, MatrixIpcError> {
    let snapshot = state.public_sync_status().await?;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_sync_status(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-sync-status-invalid-payload"));
        }
        let response = sync_status(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-sync-status-serialization-failed"))
    })
}

/// `matrix_crypto_status` is deliberately a payload-free observation. Core
/// owns its registry entry, validation, and exact wire serialization; the
/// Platform remains the sole owner of the live SDK crypto observation.
/// Typed `matrix_crypto_status`.
#[cfg(test)]
pub(super) async fn crypto_status(
    state: &Arc<CoreState>,
) -> Result<MatrixCryptoStatus, MatrixIpcError> {
    let response = state.public_crypto_status().await?;
    Ok(response)
}

#[cfg(test)]
pub(super) fn matrix_crypto_status(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-crypto-status-invalid-payload"));
        }
        let response = crypto_status(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-crypto-status-serialization-failed"))
    })
}

/// `matrix_cross_signing_status` is a payload-free read observation. Core owns
/// its registration, exact wire DTO, and legacy truth-table reconstruction;
/// the Platform remains the sole owner of the Matrix SDK identity query and
/// its client/crypto/store/network side effects.
/// Typed `matrix_cross_signing_status`.
pub(super) async fn cross_signing_status(
    state: &Arc<CoreState>,
) -> Result<MatrixCrossSigningStatusResponse, MatrixIpcError> {
    let status = state
        .platform()
        .cross_signing_status()
        .await
        .map_err(cross_signing_status_transport_error)?;
    let response = MatrixCrossSigningStatusResponse::from_platform(status)?;
    Ok(response)
}

#[cfg(test)]
pub(super) fn matrix_cross_signing_status(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-cross-signing-status-invalid-payload"));
        }
        let response = cross_signing_status(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-cross-signing-status-serialization-failed"))
    })
}

/// Convert only the closed desktop status failures into static Core errors.
/// The three legacy pairs are reconstructed here; the desktop bridge accepts
/// exactly those category/diagnostic pairs and no dynamic Core text.
pub(super) fn cross_signing_status_transport_error(
    error: PlatformCrossSigningStatusError,
) -> MatrixIpcError {
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
/// Core queries its managed SDK owner when attached. Before owner attachment,
/// the platform bridge supplies the same shared domain projection through
/// closed transport fields; neither status route carries secret material.
/// Typed `matrix_secret_storage_status`.
pub(super) async fn secret_storage_status(
    state: &Arc<CoreState>,
) -> Result<MatrixSecretStorageStatusResponse, MatrixIpcError> {
    if let Some(owner) = state.device_owner()? {
        let status = owner.secret_storage_status().await.map_err(|diagnostic| {
            MatrixIpcError::new(MatrixIpcErrorCategory::RecoveryFailure).with_diagnostic(diagnostic)
        })?;
        return Ok(MatrixSecretStorageStatusResponse::from(status));
    }
    let status = state
        .platform()
        .secret_storage_status()
        .await
        .map_err(secret_storage_status_transport_error)?;
    let response = MatrixSecretStorageStatusResponse::from_platform(status)?;
    Ok(response)
}

#[cfg(test)]
pub(super) fn matrix_secret_storage_status(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-secret-storage-status-invalid-payload"));
        }
        let response = secret_storage_status(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-secret-storage-status-serialization-failed"))
    })
}

/// Convert only the closed shell failures into the established public static
/// error categories and diagnostics. No shell-provided text crosses this map.
pub(super) fn secret_storage_status_transport_error(
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

/// Typed `matrix_login_flows`.
pub(super) async fn login_flows(
    state: &Arc<CoreState>,
    payload: MatrixLoginFlowsRequest,
) -> Result<MatrixLoginFlowsResponse, MatrixIpcError> {
    let transport = HttpLoginFlowTransport::new_with_user_agent(state.platform().http_user_agent())
        .map_err(auth_transport_error)?;
    let result = discover_login_flows(&payload.homeserver_url, &transport)
        .await
        .map_err(auth_transport_error)?;
    let response: MatrixLoginFlowsResponse = login_flows_response(result.flows);
    Ok(response)
}

#[cfg(test)]
pub(super) fn matrix_login_flows(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixLoginFlowsRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-login-flows-invalid-payload"))?;
        let response = login_flows(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-login-flows-serialization-failed"))
    })
}

/// Typed `matrix_register_flows`.
pub(super) async fn register_flows(
    state: &Arc<CoreState>,
    payload: MatrixRegisterFlowsRequest,
) -> Result<RegisterFlowsProbe, MatrixIpcError> {
    let transport =
        HttpRegisterFlowTransport::new_with_user_agent(state.platform().http_user_agent())
            .map_err(auth_transport_error)?;
    let response: RegisterFlowsProbe = probe_register_flows(&payload.homeserver_url, &transport)
        .await
        .map_err(auth_transport_error)?;
    Ok(response)
}

#[cfg(test)]
pub(super) fn matrix_register_flows(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRegisterFlowsRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-register-flows-invalid-payload"))?;
        let response = register_flows(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-register-flows-serialization-failed"))
    })
}

/// Convert the credential-free auth domain's static diagnostics into the
/// versioned core transport error shape. Never attach input URLs, HTTP bodies,
/// credentials, tokens, or a raw library error.
pub(super) fn auth_transport_error(error: AuthError) -> MatrixIpcError {
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

/// Registers this domain's JSON adapters with the test-only command registry.
#[cfg(test)]
pub(super) fn register_commands(registry: &mut CommandRegistry) {
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
        .register("matrix_login_flows", matrix_login_flows)
        .expect("built-in matrix_login_flows must remain in the command census");
    registry
        .register("matrix_register_flows", matrix_register_flows)
        .expect("built-in matrix_register_flows must remain in the command census");
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
}
