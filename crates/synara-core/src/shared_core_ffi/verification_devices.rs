//! Typed SharedCore operations and projections for verification devices.

use super::*;

super::wire_enum::wire_enum! {
    pub enum VerificationDirectionDto {
        Incoming => "incoming",
        Outgoing => "outgoing",
    }
}
super::wire_enum::wire_enum_from!(NativeVerificationDirection => VerificationDirectionDto {
    Incoming, Outgoing
});

super::wire_enum::wire_enum! {
    pub enum VerificationPhaseDto {
        Requested => "requested",
        Ready => "ready",
        Started => "started",
        KeysExchanging => "keys_exchanging",
        SasReady => "sas_ready",
        QrScanned => "qr_scanned",
        Confirmed => "confirmed",
        Done => "done",
        Mismatched => "mismatched",
        Cancelled => "cancelled",
        Failed => "failed",
    }
}
super::wire_enum::wire_enum_from!(NativeVerificationPhase => VerificationPhaseDto {
    Requested, Ready, Started, KeysExchanging, SasReady, QrScanned, Confirmed, Done,
    Mismatched, Cancelled, Failed
});

super::wire_enum::wire_enum! {
    pub enum DeviceTrustDto {
        Verified => "verified",
        VerifiedLocallyOnly => "verified_locally_only",
        VerifiedByCertificate => "verified_by_certificate",
        Unverified => "unverified",
        NoEncryption => "no_encryption",
        Dehydrated => "dehydrated",
    }
}
super::wire_enum::wire_enum_from!(NativeDeviceTrust => DeviceTrustDto {
    Verified, VerifiedLocallyOnly, VerifiedByCertificate, Unverified, NoEncryption, Dehydrated
});

super::wire_enum::wire_enum! {
    pub enum OwnDeviceVerificationDto {
        Unknown => "unknown",
        Unverified => "unverified",
        Verified => "verified",
    }
}
super::wire_enum::wire_enum_from!(crate::app::devices::NativeOwnDeviceVerification => OwnDeviceVerificationDto {
    Unknown, Unverified, Verified
});

super::wire_enum::wire_enum! {
    pub enum DeviceDeleteAuthenticationDto {
        Password => "password",
    }
}
super::wire_enum::wire_enum_from!(NativeDeviceDeleteAuthentication => DeviceDeleteAuthenticationDto {
    Password
});

super::wire_enum::wire_enum! {
    pub enum DeviceDeleteOutcomeDto {
        Complete => "complete",
        AuthenticationRequired => "authentication_required",
    }
}

/// Privacy-safe SAS emoji. User-visible comparison only; no key material.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct VerificationEmojiDto {
    pub symbol: String,
    pub description: String,
}

/// Privacy-safe SAS comparison. Emoji/decimals only; no tokens or MACs.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct VerificationSasDto {
    pub emoji: Option<Vec<VerificationEmojiDto>>,
    pub decimals: Option<Vec<u16>>,
}

/// Privacy-safe show-QR payload. SVG data-URL only; no MAC or QR bytes.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct VerificationQrDto {
    pub image_data_url: String,
    pub scanned: bool,
}

/// Privacy-safe verification request row. Identity/flow fields and optional
/// display-only SAS / QR values; no tokens, MACs, or key material.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct VerificationRequestDto {
    pub flow_id: String,
    pub other_user_id: String,
    pub other_device_id: Option<String>,
    pub direction: VerificationDirectionDto,
    pub phase: VerificationPhaseDto,
    pub started_ts: Option<u64>,
    pub sas: Option<VerificationSasDto>,
    pub qr: Option<VerificationQrDto>,
}

/// Privacy-safe verification inbox. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct VerificationInboxDto {
    pub session_generation: u64,
    pub requests: Vec<VerificationRequestDto>,
}

/// Static fail-closed verification-list error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum VerificationListError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for VerificationListError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for VerificationListError {}

pub(super) fn verification_list_failed(
    code: &'static str,
    description: &'static str,
) -> VerificationListError {
    VerificationListError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_verification_list_core_error(error: MatrixIpcError) -> VerificationListError {
    match error.diagnostic_id.as_deref() {
        Some("p2-verification-list-no-session") => verification_list_failed(
            VERIFICATION_LIST_NO_SESSION_CODE,
            VERIFICATION_LIST_NO_SESSION_DESCRIPTION,
        ),
        _ => verification_list_failed(
            VERIFICATION_LIST_FAILED_CODE,
            VERIFICATION_LIST_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn verification_emoji_dto(emoji: NativeVerificationEmoji) -> VerificationEmojiDto {
    VerificationEmojiDto {
        symbol: emoji.symbol,
        description: emoji.description,
    }
}

pub(super) fn verification_sas_dto(sas: NativeVerificationSas) -> VerificationSasDto {
    VerificationSasDto {
        emoji: sas
            .emoji
            .map(|emoji| emoji.into_iter().map(verification_emoji_dto).collect()),
        decimals: sas.decimals.map(|decimals| decimals.to_vec()),
    }
}

pub(super) fn verification_qr_dto(qr: NativeVerificationQr) -> VerificationQrDto {
    VerificationQrDto {
        image_data_url: qr.image_data_url,
        scanned: qr.scanned,
    }
}

pub(super) fn verification_request_dto_with_sas(
    request: NativeVerificationRequest,
) -> VerificationRequestDto {
    VerificationRequestDto {
        flow_id: request.flow_id,
        other_user_id: request.other_user_id,
        other_device_id: request.other_device_id,
        direction: request.direction.into(),
        phase: request.phase.into(),
        started_ts: request.started_ts,
        sas: request.sas.map(verification_sas_dto),
        qr: request.qr.map(verification_qr_dto),
    }
}

/// Static fail-closed verification-SAS error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum VerificationSasError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for VerificationSasError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for VerificationSasError {}

pub(super) fn verification_sas_failed(
    code: &str,
    description: &'static str,
) -> VerificationSasError {
    VerificationSasError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_verification_sas_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> VerificationSasError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            verification_sas_failed(code, VERIFICATION_SAS_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-crypto.1-") => {
            verification_sas_failed(code, VERIFICATION_SAS_OWNER_DESCRIPTION)
        }
        _ => verification_sas_failed(
            VERIFICATION_SAS_FAILED_CODE,
            VERIFICATION_SAS_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn parse_verification_sas_request(
    payload: NativeVerificationRequest,
) -> Result<VerificationRequestDto, VerificationSasError> {
    let request: NativeVerificationRequest = payload;
    Ok(verification_request_dto_with_sas(request))
}

/// Privacy-safe device row. Identity/presentation fields only; no keys or tokens.
/// Additive fingerprint/first-seen/cross-sign fields are optional for older
/// consumers.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DeviceSummaryDto {
    pub device_id: String,
    pub display_name: Option<String>,
    pub last_seen_ip: Option<String>,
    pub last_seen_ts: Option<u64>,
    pub trust: DeviceTrustDto,
    pub is_current: bool,
    pub is_cross_signed_by_owner: Option<bool>,
    pub first_seen_ts: Option<u64>,
    pub ed25519_fingerprint: Option<String>,
}

/// Privacy-safe device inbox. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DeviceSnapshotDto {
    pub session_generation: u64,
    pub own_verification: OwnDeviceVerificationDto,
    pub has_devices_to_verify_against: Option<bool>,
    pub devices: Vec<DeviceSummaryDto>,
}

/// Privacy-safe delete challenge. Authentication type only; no password.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DeviceDeleteChallengeDto {
    pub operation_id: u64,
    pub session_generation: u64,
    pub authentication: DeviceDeleteAuthenticationDto,
    pub authentication_failed: bool,
}

/// Privacy-safe delete start result. Complete snapshot or challenge; no password.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DeviceDeleteDto {
    pub outcome: DeviceDeleteOutcomeDto,
    pub snapshot: Option<DeviceSnapshotDto>,
    pub challenge: Option<DeviceDeleteChallengeDto>,
}

/// Static fail-closed device-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum DeviceCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for DeviceCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for DeviceCommandError {}

pub(super) fn device_failed(code: &str, description: &'static str) -> DeviceCommandError {
    DeviceCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_device_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> DeviceCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => device_failed(code, DEVICE_NO_SESSION_DESCRIPTION),
        Some(code) if code.starts_with("v-crypto.7-") => {
            device_failed(code, DEVICE_OWNER_DESCRIPTION)
        }
        _ => device_failed(DEVICE_FAILED_CODE, DEVICE_FAILED_DESCRIPTION),
    }
}

pub(super) fn device_snapshot_dto(snapshot: NativeDeviceSnapshot) -> DeviceSnapshotDto {
    DeviceSnapshotDto {
        session_generation: snapshot.session_generation,
        own_verification: snapshot.own_verification.into(),
        has_devices_to_verify_against: snapshot.has_devices_to_verify_against,
        devices: snapshot
            .devices
            .into_iter()
            .map(|device| DeviceSummaryDto {
                device_id: device.device_id,
                display_name: device.display_name,
                last_seen_ip: device.last_seen_ip,
                last_seen_ts: device.last_seen_ts,
                trust: device.trust.into(),
                is_current: device.is_current,
                is_cross_signed_by_owner: Some(device.is_cross_signed_by_owner),
                first_seen_ts: device.first_seen_ts,
                ed25519_fingerprint: device.ed25519_fingerprint,
            })
            .collect(),
    }
}

pub(super) fn device_delete_dto(result: NativeDeviceDeleteResult) -> DeviceDeleteDto {
    match result {
        NativeDeviceDeleteResult::Complete { snapshot } => DeviceDeleteDto {
            outcome: DeviceDeleteOutcomeDto::Complete,
            snapshot: Some(device_snapshot_dto(snapshot)),
            challenge: None,
        },
        NativeDeviceDeleteResult::AuthenticationRequired { challenge } => DeviceDeleteDto {
            outcome: DeviceDeleteOutcomeDto::AuthenticationRequired,
            snapshot: None,
            challenge: Some(DeviceDeleteChallengeDto {
                operation_id: challenge.operation_id,
                session_generation: challenge.session_generation,
                authentication: challenge.authentication.into(),
                authentication_failed: challenge.authentication_failed,
            }),
        },
    }
}

impl SharedCore {
    pub(super) async fn verification_flow_command(
        &self,
        no_session: &'static str,
        request: impl std::future::Future<Output = Result<NativeVerificationRequest, MatrixIpcError>>,
    ) -> Result<VerificationRequestDto, VerificationSasError> {
        let response = request
            .await
            .map_err(|error| map_verification_sas_core_error(no_session, error))?;
        parse_verification_sas_request(response)
    }

    pub(super) async fn device_null_command<T>(
        &self,
        no_session: &'static str,
        request: impl std::future::Future<Output = Result<T, MatrixIpcError>>,
    ) -> Result<T, DeviceCommandError> {
        let response = request
            .await
            .map_err(|error| map_device_core_error(no_session, error))?;
        Ok(response)
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SharedCore {
    pub async fn verification_list(&self) -> Result<VerificationInboxDto, VerificationListError> {
        let response = self
            .core
            .verification_list()
            .await
            .map_err(map_verification_list_core_error)?;
        let inbox: NativeVerificationInbox = response;
        Ok(VerificationInboxDto {
            session_generation: inbox.session_generation,
            requests: inbox
                .requests
                .into_iter()
                // `verification_list` is the long-lived UI observation path.
                // Preserve the display-only SAS payload here; returning it only
                // from one-shot mutations makes `sas_ready` impossible to render.
                .map(verification_request_dto_with_sas)
                .collect(),
        })
    }

    pub async fn verification_start(
        &self,
        device_id: Option<String>,
    ) -> Result<VerificationRequestDto, VerificationSasError> {
        let response = self
            .core
            .verification_start(crate::core_api::MatrixVerificationStartRequest { device_id })
            .await
            .map_err(|error| {
                map_verification_sas_core_error(VERIFICATION_START_NO_SESSION_CODE, error)
            })?;
        parse_verification_sas_request(response)
    }

    pub async fn verification_accept(
        &self,
        flow_id: String,
    ) -> Result<VerificationRequestDto, VerificationSasError> {
        self.verification_flow_command(
            VERIFICATION_ACCEPT_NO_SESSION_CODE,
            self.core
                .verification_accept(crate::core_api::MatrixVerificationAcceptRequest { flow_id }),
        )
        .await
    }

    pub async fn verification_begin_sas(
        &self,
        flow_id: String,
    ) -> Result<VerificationRequestDto, VerificationSasError> {
        self.verification_flow_command(
            VERIFICATION_BEGIN_SAS_NO_SESSION_CODE,
            self.core
                .verification_begin_sas(crate::core_api::MatrixVerificationBeginSasRequest {
                    flow_id,
                }),
        )
        .await
    }

    pub async fn verification_confirm(
        &self,
        flow_id: String,
    ) -> Result<VerificationRequestDto, VerificationSasError> {
        self.verification_flow_command(
            VERIFICATION_CONFIRM_NO_SESSION_CODE,
            self.core
                .verification_confirm(crate::core_api::MatrixVerificationConfirmRequest {
                    flow_id,
                }),
        )
        .await
    }

    pub async fn verification_mismatch(
        &self,
        flow_id: String,
    ) -> Result<VerificationRequestDto, VerificationSasError> {
        self.verification_flow_command(
            VERIFICATION_MISMATCH_NO_SESSION_CODE,
            self.core
                .verification_mismatch(crate::core_api::MatrixVerificationMismatchRequest {
                    flow_id,
                }),
        )
        .await
    }

    pub async fn verification_cancel(
        &self,
        flow_id: String,
    ) -> Result<VerificationRequestDto, VerificationSasError> {
        self.verification_flow_command(
            VERIFICATION_CANCEL_NO_SESSION_CODE,
            self.core
                .verification_cancel(crate::core_api::MatrixVerificationCancelRequest { flow_id }),
        )
        .await
    }

    pub async fn verification_dismiss(&self, flow_id: String) -> Result<(), VerificationSasError> {
        self.core
            .verification_dismiss(crate::core_api::MatrixVerificationDismissRequest { flow_id })
            .await
            .map_err(|error| {
                map_verification_sas_core_error(VERIFICATION_DISMISS_NO_SESSION_CODE, error)
            })?;
        Ok(())
    }

    pub async fn device_snapshot(&self) -> Result<DeviceSnapshotDto, DeviceCommandError> {
        let response = self
            .device_null_command(DEVICE_SNAPSHOT_NO_SESSION_CODE, self.core.device_snapshot())
            .await?;
        let snapshot: NativeDeviceSnapshot = response;
        Ok(device_snapshot_dto(snapshot))
    }

    pub async fn device_rename(
        &self,
        device_id: String,
        display_name: String,
    ) -> Result<DeviceSnapshotDto, DeviceCommandError> {
        let response = self
            .core
            .device_rename(crate::core_api::MatrixDeviceRenameRequest {
                device_id,
                display_name,
            })
            .await
            .map_err(|error| map_device_core_error(DEVICE_RENAME_NO_SESSION_CODE, error))?;
        let snapshot: NativeDeviceSnapshot = response;
        Ok(device_snapshot_dto(snapshot))
    }

    pub async fn device_delete_start(
        &self,
        device_ids: Vec<String>,
    ) -> Result<DeviceDeleteDto, DeviceCommandError> {
        let response = self
            .core
            .device_delete_start(crate::core_api::MatrixDeviceDeleteStartRequest { device_ids })
            .await
            .map_err(|error| map_device_core_error(DEVICE_DELETE_START_NO_SESSION_CODE, error))?;
        let result: NativeDeviceDeleteResult = response;
        Ok(device_delete_dto(result))
    }

    pub async fn device_delete_cancel(
        &self,
        operation_id: u64,
        session_generation: u64,
    ) -> Result<(), DeviceCommandError> {
        self.core
            .device_delete_cancel(crate::core_api::MatrixDeviceDeleteCancelRequest {
                operation_id,
                session_generation,
            })
            .await
            .map_err(|error| map_device_core_error(DEVICE_DELETE_CANCEL_NO_SESSION_CODE, error))?;
        Ok(())
    }

    /// Password UIAA for a pending device delete. Password is a method argument,
    /// never a Core JSON field.
    pub async fn device_delete_password(
        &self,
        operation_id: u64,
        session_generation: u64,
        password: String,
    ) -> Result<DeviceDeleteDto, DeviceCommandError> {
        let result = self
            .core
            .device_delete_password(operation_id, session_generation, &password)
            .await
            .map_err(|error| {
                map_device_core_error(DEVICE_DELETE_PASSWORD_NO_SESSION_CODE, error)
            })?;
        drop(password);
        Ok(device_delete_dto(result))
    }
}
