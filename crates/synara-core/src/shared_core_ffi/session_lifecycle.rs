//! Typed SharedCore operations and projections for session lifecycle.

use super::*;
use crate::app::lifecycle::session as session_policy;
use crate::app::lifecycle::session::{
    RotationDiagnostics, RotationHooks, SessionFault, SessionPersistenceLease,
    SessionPersistenceOwner,
};
use crate::app::sync::{CommandGate, SYNC_AUTHENTICATION_FAILURE_DIAGNOSTIC_ID};

/// Static fail-closed vault error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IosSecretVaultError {
    Unavailable { code: String, description: String },
}

/// Swift-owned key/value secret store described by the existing UDL callback.
///
/// UniFFI UDL mode generates glue only; the trait itself must live in Rust.
pub trait IosSecretVault: Send + Sync {
    fn get(&self, key: String) -> Result<Option<Vec<u8>>, IosSecretVaultError>;
    fn put(&self, key: String, value: Vec<u8>) -> Result<(), IosSecretVaultError>;
    fn delete(&self, key: String) -> Result<(), IosSecretVaultError>;
}

impl std::fmt::Display for IosSecretVaultError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for IosSecretVaultError {}

/// Privacy-safe restore outcome. Tokens never appear here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRestoreDto {
    pub user_id: String,
    pub device_id: String,
    pub homeserver_url: String,
}

/// Static fail-closed restore error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionRestoreError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SessionRestoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SessionRestoreError {}

pub(super) fn restore_failed(code: &'static str, description: &'static str) -> SessionRestoreError {
    SessionRestoreError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

/// Privacy-safe login outcome. Tokens and password never appear here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionLoginDto {
    pub user_id: String,
    pub device_id: String,
    pub homeserver_url: String,
}

/// Static fail-closed login error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionLoginError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SessionLoginError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SessionLoginError {}

pub(super) fn login_failed(code: &'static str, description: &'static str) -> SessionLoginError {
    SessionLoginError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

/// Privacy-safe attach outcome. Owner names only; no tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionAttachDto {
    pub owners: Vec<String>,
}

/// Static fail-closed attach error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionAttachError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SessionAttachError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SessionAttachError {}

pub(super) fn attach_failed(code: &'static str, description: &'static str) -> SessionAttachError {
    SessionAttachError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

/// Privacy-safe start outcome. No tokens, URLs, or SDK error text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncStartDto {
    pub readiness: String,
    pub session_generation: u64,
    pub started: bool,
    pub offline_mode_enabled: bool,
}

/// Privacy-safe stop outcome. No tokens, URLs, paths, or SDK error text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncStopDto {
    pub readiness: String,
    pub session_generation: u64,
    pub stopped: bool,
    pub offline_mode_enabled: bool,
}

/// Static fail-closed stop error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncStopError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SyncStopError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SyncStopError {}

pub(super) fn sync_stop_failed(code: &'static str, description: &'static str) -> SyncStopError {
    SyncStopError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

/// Static fail-closed start error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncStartError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SyncStartError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SyncStartError {}

pub(super) fn sync_start_failed(code: &'static str, description: &'static str) -> SyncStartError {
    SyncStartError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

/// Privacy-safe live session snapshot from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSnapshotDto {
    pub status: String,
    pub user_id: Option<String>,
    pub device_id: Option<String>,
    pub homeserver_url: Option<String>,
    pub session_generation: Option<u64>,
}

/// Privacy-safe sync readiness from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncStatusDto {
    pub readiness: String,
    pub session_generation: u64,
    pub offline_mode_enabled: bool,
    pub failure_diagnostic_id: Option<String>,
    pub sliding_sync_capable: Option<bool>,
    /// `open` or `closed`. Never a free-form string.
    pub command_gate: String,
}

/// Static fail-closed session/status error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionStatusError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SessionStatusError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SessionStatusError {}

/// Privacy-safe leftover write ack. Status only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeftoverAckDto {
    pub status: String,
}

/// Privacy-safe leftover bytes readback. Callers must not log the payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeftoverBytesDto {
    pub payload: Vec<u8>,
}

/// Static fail-closed leftover error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeftoverCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for LeftoverCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for LeftoverCommandError {}

pub(super) fn leftover_failed(code: &str, description: &'static str) -> LeftoverCommandError {
    LeftoverCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

/// Privacy-safe MSC4426 in-call field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserInCallDto {
    pub call_joined_ts: Option<u64>,
}

pub(super) enum RestoredClientSlot {
    Empty,
    InFlight,
    /// Retained for S3d attach after restore or login, with the persistence
    /// owner that fences this client's token-rotation saves. Dropping or
    /// revoking it stops late SDK refreshes from writing the vault.
    Ready(Client, SessionPersistenceOwner),
}

pub(super) enum OwnerAttachSlot {
    Empty,
    InFlight,
    Ready,
}

pub(super) fn json_optional_string(value: Option<&serde_json::Value>) -> Option<String> {
    value.and_then(|value| value.as_str()).map(str::to_owned)
}

pub(super) fn closed_creators_event_type(value: &str) -> Option<&'static str> {
    match value {
        "m.room.create" => Some("m.room.create"),
        _ => None,
    }
}

pub(super) fn snapshot_content_json(
    content: &serde_json::Value,
) -> Result<String, RoomMembersSnapshotError> {
    let content_json = serde_json::to_string(content).map_err(|_| {
        room_members_snapshot_failed(
            ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
            ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
        )
    })?;
    if content_json.len() > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(room_members_snapshot_failed(
            ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
            ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
        ));
    }
    Ok(content_json)
}

pub(super) fn session_status_failed(code: &str, description: &'static str) -> SessionStatusError {
    SessionStatusError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_session_status_core_error(error: MatrixIpcError) -> SessionStatusError {
    match error.diagnostic_id.as_deref() {
        Some(code)
            if code.starts_with("p2-session-snapshot-")
                || code.starts_with("p2-sync-status-")
                || code.starts_with("p2-media-config-")
                || code.starts_with("p2-secret-storage-status-")
                || code.starts_with("v-crypto.4-") =>
        {
            session_status_failed(code, SESSION_STATUS_OWNER_DESCRIPTION)
        }
        _ => session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn session_status_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, SessionStatusError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

#[derive(Debug, Deserialize)]
pub(super) struct SessionSnapshotResultWire {
    pub(super) status: String,
    pub(super) user_id: Option<String>,
    pub(super) device_id: Option<String>,
    pub(super) homeserver_url: Option<String>,
    #[serde(rename = "sessionGeneration")]
    pub(super) session_generation: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SyncStatusResultWire {
    pub(super) readiness: String,
    pub(super) session_generation: u64,
    pub(super) offline_mode_enabled: bool,
    pub(super) failure_diagnostic_id: Option<String>,
    pub(super) sliding_sync_capable: Option<bool>,
    #[serde(default)]
    pub(super) command_gate: Option<String>,
}

pub(super) fn closed_session_snapshot_status(value: &str) -> Option<&'static str> {
    match value {
        "logged_out" => Some("logged_out"),
        "logged_in" => Some("logged_in"),
        _ => None,
    }
}

pub(super) fn closed_sync_readiness(value: &str) -> Option<&'static str> {
    match value {
        "unconfigured" => Some("unconfigured"),
        "idle" => Some("idle"),
        "running" => Some("running"),
        "offline" => Some("offline"),
        "terminated" => Some("terminated"),
        "failed" => Some("failed"),
        _ => None,
    }
}

pub(super) fn closed_sync_failure_diagnostic(value: Option<&str>) -> Option<Option<&'static str>> {
    match value {
        None => Some(None),
        Some(SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID) => Some(Some(SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID)),
        Some(SYNC_AUTHENTICATION_FAILURE_DIAGNOSTIC_ID) => {
            Some(Some(SYNC_AUTHENTICATION_FAILURE_DIAGNOSTIC_ID))
        }
        Some(_) => None,
    }
}

pub(super) fn closed_command_gate(value: Option<&str>) -> &'static str {
    match value {
        None | Some("open") => "open",
        Some("closed") | Some(_) => "closed",
    }
}

pub(super) fn closed_missing_secret(value: &str) -> Option<&'static str> {
    match value {
        "cross_signing_master" => Some("cross_signing_master"),
        "cross_signing_self_signing" => Some("cross_signing_self_signing"),
        "cross_signing_user_signing" => Some("cross_signing_user_signing"),
        "encryption_backup" => Some("encryption_backup"),
        _ => None,
    }
}

pub(super) fn session_snapshot_dto(
    payload: serde_json::Value,
) -> Result<SessionSnapshotDto, SessionStatusError> {
    let result: SessionSnapshotResultWire = serde_json::from_value(payload).map_err(|_| {
        session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        )
    })?;
    let status = closed_session_snapshot_status(&result.status).ok_or_else(|| {
        session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        )
    })?;
    match status {
        "logged_out" => Ok(SessionSnapshotDto {
            status: status.to_owned(),
            user_id: None,
            device_id: None,
            homeserver_url: None,
            session_generation: None,
        }),
        "logged_in" => {
            let user_id = result
                .user_id
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    session_status_failed(
                        SESSION_STATUS_FAILED_CODE,
                        SESSION_STATUS_FAILED_DESCRIPTION,
                    )
                })?;
            let device_id = result
                .device_id
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    session_status_failed(
                        SESSION_STATUS_FAILED_CODE,
                        SESSION_STATUS_FAILED_DESCRIPTION,
                    )
                })?;
            let homeserver_url = result
                .homeserver_url
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    session_status_failed(
                        SESSION_STATUS_FAILED_CODE,
                        SESSION_STATUS_FAILED_DESCRIPTION,
                    )
                })?;
            let session_generation = result.session_generation.ok_or_else(|| {
                session_status_failed(
                    SESSION_STATUS_FAILED_CODE,
                    SESSION_STATUS_FAILED_DESCRIPTION,
                )
            })?;
            Ok(SessionSnapshotDto {
                status: status.to_owned(),
                user_id: Some(user_id),
                device_id: Some(device_id),
                homeserver_url: Some(homeserver_url),
                session_generation: Some(session_generation),
            })
        }
        _ => Err(session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        )),
    }
}

pub(super) fn product_live_readiness(readiness: SyncReadiness) -> bool {
    matches!(readiness, SyncReadiness::Running | SyncReadiness::Offline)
}

pub(super) fn sync_start_dto_from_snapshot(snapshot: SyncReadinessSnapshot) -> SyncStartDto {
    SyncStartDto {
        readiness: snapshot.readiness.as_str().to_owned(),
        session_generation: snapshot.session_generation,
        started: product_live_readiness(snapshot.readiness),
        offline_mode_enabled: snapshot.offline_mode_enabled,
    }
}

pub(super) fn sync_stop_dto_from_snapshot(snapshot: SyncReadinessSnapshot) -> SyncStopDto {
    SyncStopDto {
        readiness: snapshot.readiness.as_str().to_owned(),
        session_generation: snapshot.session_generation,
        stopped: matches!(
            snapshot.readiness,
            SyncReadiness::Idle | SyncReadiness::Terminated
        ),
        offline_mode_enabled: snapshot.offline_mode_enabled,
    }
}

pub(super) async fn wait_for_started_readiness(
    owner: &SyncServiceOwner,
    mut snapshot: SyncReadinessSnapshot,
) -> SyncReadinessSnapshot {
    let deadline = tokio::time::Instant::now() + START_OBSERVE_TIMEOUT;
    while snapshot.readiness == SyncReadiness::Idle && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(START_OBSERVE_POLL).await;
        snapshot = owner.observe();
    }
    snapshot
}

pub(super) fn sync_status_from_owner_snapshot(
    snapshot: SyncReadinessSnapshot,
    timeline_owner_attached: bool,
) -> Result<SyncStatusDto, SessionStatusError> {
    if !snapshot.is_valid_public_sync_status() {
        return Err(session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        ));
    }
    let command_gate =
        CommandGate::for_installed_session(timeline_owner_attached, snapshot.failure_diagnostic_id);
    Ok(SyncStatusDto {
        readiness: snapshot.readiness.as_str().to_owned(),
        session_generation: snapshot.session_generation,
        offline_mode_enabled: snapshot.offline_mode_enabled,
        failure_diagnostic_id: snapshot.failure_diagnostic_id.map(str::to_owned),
        sliding_sync_capable: snapshot.sliding_sync_capable,
        command_gate: command_gate.as_str().to_owned(),
    })
}

pub(super) fn sync_status_dto(
    payload: serde_json::Value,
) -> Result<SyncStatusDto, SessionStatusError> {
    let result: SyncStatusResultWire = serde_json::from_value(payload).map_err(|_| {
        session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        )
    })?;
    let readiness = closed_sync_readiness(&result.readiness).ok_or_else(|| {
        session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        )
    })?;
    let failure_diagnostic_id =
        closed_sync_failure_diagnostic(result.failure_diagnostic_id.as_deref())
            .ok_or_else(|| {
                session_status_failed(
                    SESSION_STATUS_FAILED_CODE,
                    SESSION_STATUS_FAILED_DESCRIPTION,
                )
            })?
            .map(str::to_owned);
    let command_gate = closed_command_gate(result.command_gate.as_deref());
    Ok(SyncStatusDto {
        readiness: readiness.to_owned(),
        session_generation: result.session_generation,
        offline_mode_enabled: result.offline_mode_enabled,
        failure_diagnostic_id,
        sliding_sync_capable: result.sliding_sync_capable,
        command_gate: command_gate.to_owned(),
    })
}

/// Claims the restore slot for one in-flight attempt. Drop releases it unless
/// [`RestoreClaim::commit`] stores the Client after a successful Core open.
pub(super) struct RestoreClaim<'a> {
    pub(super) slot: &'a Mutex<RestoredClientSlot>,
    pub(super) committed: bool,
}

impl<'a> RestoreClaim<'a> {
    pub(super) fn acquire(
        slot: &'a Mutex<RestoredClientSlot>,
    ) -> Result<Self, SessionRestoreError> {
        let mut guard = slot
            .lock()
            .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
        match *guard {
            RestoredClientSlot::Empty => {
                *guard = RestoredClientSlot::InFlight;
                Ok(Self {
                    slot,
                    committed: false,
                })
            }
            RestoredClientSlot::Ready(..) => Err(restore_failed(
                ALREADY_RESTORED_CODE,
                ALREADY_RESTORED_DESCRIPTION,
            )),
            RestoredClientSlot::InFlight => Err(restore_failed(
                RESTORE_FAILED_CODE,
                RESTORE_FAILED_DESCRIPTION,
            )),
        }
    }

    fn commit(
        mut self,
        client: Client,
        persistence: SessionPersistenceOwner,
    ) -> Result<(), SessionRestoreError> {
        let mut guard = self
            .slot
            .lock()
            .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
        if !matches!(*guard, RestoredClientSlot::InFlight) {
            return Err(restore_failed(
                RESTORE_FAILED_CODE,
                RESTORE_FAILED_DESCRIPTION,
            ));
        }
        *guard = RestoredClientSlot::Ready(client, persistence);
        self.committed = true;
        Ok(())
    }
}

impl Drop for RestoreClaim<'_> {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        if let Ok(mut guard) = self.slot.lock() {
            if matches!(*guard, RestoredClientSlot::InFlight) {
                *guard = RestoredClientSlot::Empty;
            }
        }
    }
}

/// Claims the owner-attach slot for one in-flight attempt.
pub(super) struct AttachClaim<'a> {
    pub(super) slot: &'a Mutex<OwnerAttachSlot>,
    pub(super) committed: bool,
}

impl<'a> AttachClaim<'a> {
    pub(super) fn acquire(slot: &'a Mutex<OwnerAttachSlot>) -> Result<Self, SessionAttachError> {
        let mut guard = slot
            .lock()
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        match *guard {
            OwnerAttachSlot::Empty => {
                *guard = OwnerAttachSlot::InFlight;
                Ok(Self {
                    slot,
                    committed: false,
                })
            }
            OwnerAttachSlot::Ready => Err(attach_failed(
                ATTACH_ALREADY_CODE,
                ATTACH_ALREADY_DESCRIPTION,
            )),
            OwnerAttachSlot::InFlight => {
                Err(attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))
            }
        }
    }

    fn commit(mut self) -> Result<(), SessionAttachError> {
        let mut guard = self
            .slot
            .lock()
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        if !matches!(*guard, OwnerAttachSlot::InFlight) {
            return Err(attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION));
        }
        *guard = OwnerAttachSlot::Ready;
        self.committed = true;
        Ok(())
    }
}

impl Drop for AttachClaim<'_> {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        if let Ok(mut guard) = self.slot.lock() {
            if matches!(*guard, OwnerAttachSlot::InFlight) {
                *guard = OwnerAttachSlot::Empty;
            }
        }
    }
}

pub(super) fn leftover_reject_oversize(size: usize) -> Result<(), LeftoverCommandError> {
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(leftover_failed(
            LEFTOVER_OVERSIZE_CODE,
            LEFTOVER_OVERSIZE_DESCRIPTION,
        ));
    }
    Ok(())
}

pub(super) fn leftover_status_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, LeftoverCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    leftover_reject_oversize(size)?;
    Ok(payload)
}

pub(super) fn map_leftover_status_core_error(error: MatrixIpcError) -> LeftoverCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code)
            if code.ends_with("-no-session")
                || code.contains("requires-session")
                || code.contains("-session-missing") =>
        {
            leftover_failed(code, LEFTOVER_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-backup-status-")
                || code.starts_with("p2-crypto-status-")
                || code.starts_with("p2-cross-signing-status-")
                || code.starts_with("p2-room-key-transfer-status-")
                || code.starts_with("v-crypto.2-")
                || code.starts_with("v-crypto.3-") =>
        {
            leftover_failed(code, LEFTOVER_UNAVAILABLE_DESCRIPTION)
        }
        _ => leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION),
    }
}

pub(super) struct CallbackSecretVault {
    pub(super) inner: Box<dyn IosSecretVault>,
}

impl SecretVault for CallbackSecretVault {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, MatrixIpcError> {
        self.inner
            .get(key.to_owned())
            .map_err(|_| vault_unavailable())
    }

    fn put(&self, key: &str, value: &[u8]) -> Result<(), MatrixIpcError> {
        self.inner
            .put(key.to_owned(), value.to_vec())
            .map_err(|_| vault_unavailable())
    }

    fn delete(&self, key: &str) -> Result<(), MatrixIpcError> {
        self.inner
            .delete(key.to_owned())
            .map_err(|_| vault_unavailable())
    }
}

#[derive(Clone)]
pub(super) struct SecretStoreSessionVault {
    pub(super) store: Arc<dyn SecretVault + Send + Sync>,
}

/// Keep the host vault in lockstep with SDK access/refresh-token rotation,
/// fenced by this client's persistence lease (shared Core implementation).
pub(super) fn install_session_rotation_callbacks(
    client: &matrix_sdk::Client,
    identity: AccountIdentity,
    store: Arc<dyn SecretVault + Send + Sync>,
    lease: Arc<SessionPersistenceLease>,
) -> Result<(), SessionFault> {
    session_policy::install_session_rotation_callbacks(
        client,
        identity,
        Arc::new(SecretStoreSessionVault { store }),
        lease,
        RotationDiagnostics::IOS,
        RotationHooks::<SessionFault>::plain(|_| {
            SessionFault::unavailable(RotationDiagnostics::IOS.persist_failed)
        }),
    )
}

impl SessionMaterialVault for SecretStoreSessionVault {
    fn get(
        &self,
        id: &SessionMaterialId,
    ) -> Result<Option<SessionMaterial>, crate::app::lifecycle::LifecycleError> {
        match self.store.get(id.account()) {
            Ok(Some(bytes)) => Ok(Some(SessionMaterial::from_sealed_blob(bytes))),
            Ok(None) => Ok(None),
            Err(_) => Err(crate::app::lifecycle::LifecycleError::Vault {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
                category: MatrixIpcErrorCategory::StoreUnavailable,
            }),
        }
    }

    fn set(
        &self,
        id: &SessionMaterialId,
        material: &SessionMaterial,
    ) -> Result<(), crate::app::lifecycle::LifecycleError> {
        self.store
            .put(id.account(), material.as_bytes())
            .map_err(|_| crate::app::lifecycle::LifecycleError::Vault {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
                category: MatrixIpcErrorCategory::StoreUnavailable,
            })
    }

    fn clear(&self, id: &SessionMaterialId) -> Result<bool, crate::app::lifecycle::LifecycleError> {
        let existed = self.store.get(id.account()).ok().flatten().is_some();
        self.store.delete(id.account()).map_err(|_| {
            crate::app::lifecycle::LifecycleError::Vault {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
                category: MatrixIpcErrorCategory::StoreUnavailable,
            }
        })?;
        Ok(existed)
    }
}

pub(super) fn vault_unavailable() -> MatrixIpcError {
    MatrixIpcError::new(MatrixIpcErrorCategory::StoreUnavailable)
        .with_diagnostic("p4-s3-secret-vault-unavailable")
}

impl SharedCore {
    /// Next session generation for this instance. Restore and login each
    /// install a new generation, so a shell fence keyed on a retired
    /// generation (for example rejected-auth retirement) cannot match the
    /// session that replaced it.
    pub(super) fn allocate_session_generation(&self) -> u64 {
        self.generations.allocate()
    }

    /// Construct a real Core with the fail-closed iOS Platform.
    pub fn new() -> Self {
        let platform = IosFailClosedPlatform::new();
        let secret_store = Platform::secret_store(&platform);
        Self {
            core: Core::new(Arc::new(platform)),
            secret_store,
            restored_client: Mutex::new(RestoredClientSlot::Empty),
            owner_attach: Mutex::new(OwnerAttachSlot::Empty),
            sync_lifecycle: tokio::sync::Mutex::new(()),
            nse_read_only: Mutex::new(false),
            timeline_view_updates: Arc::new(Mutex::new(Vec::new())),
            owner_updates: Arc::new(Mutex::new(Vec::new())),
            room_list_updates: Arc::new(Mutex::new(Vec::new())),
            room_list_live: Arc::new(Mutex::new(None)),
            own_profile_live: Arc::new(Mutex::new(None)),
            media_retention_live: Arc::new(Mutex::new(None)),
            generations: crate::app::lifecycle::session::SessionGenerations::new(),
            save_retry_backoff: Mutex::new(crate::app::lifecycle::session::RetryBackoff::new()),
            rejected_session: Mutex::new(None),
        }
    }

    /// Construct a real Core whose `Platform::secret_store` is the Swift vault.
    pub fn new_with_secret_store(store: Box<dyn IosSecretVault>) -> Self {
        let vault: Arc<dyn SecretVault + Send + Sync> =
            Arc::new(CallbackSecretVault { inner: store });
        let platform = IosFailClosedPlatform::with_secret_store(Arc::clone(&vault));
        Self {
            core: Core::new(Arc::new(platform)),
            secret_store: vault,
            restored_client: Mutex::new(RestoredClientSlot::Empty),
            owner_attach: Mutex::new(OwnerAttachSlot::Empty),
            sync_lifecycle: tokio::sync::Mutex::new(()),
            nse_read_only: Mutex::new(false),
            timeline_view_updates: Arc::new(Mutex::new(Vec::new())),
            owner_updates: Arc::new(Mutex::new(Vec::new())),
            room_list_updates: Arc::new(Mutex::new(Vec::new())),
            room_list_live: Arc::new(Mutex::new(None)),
            own_profile_live: Arc::new(Mutex::new(None)),
            media_retention_live: Arc::new(Mutex::new(None)),
            generations: crate::app::lifecycle::session::SessionGenerations::new(),
            save_retry_backoff: Mutex::new(crate::app::lifecycle::session::RetryBackoff::new()),
            rejected_session: Mutex::new(None),
        }
    }

    /// Restore an already-persisted session from the S3a vault. No password.
    ///
    /// `store_root` is the shell-owned SDK store directory. It is never echoed.
    /// This is not `matrix_restore_session` and does not attach owners or
    /// expose `Core.command`.
    pub async fn restore_persisted_session(
        &self,
        user_id: String,
        homeserver_url: String,
        store_root: String,
    ) -> Result<SessionRestoreDto, SessionRestoreError> {
        self.restore_persisted_session_with_policy(user_id, homeserver_url, store_root, false, None)
            .await
    }

    pub(super) async fn restore_persisted_session_with_policy(
        &self,
        user_id: String,
        homeserver_url: String,
        store_root: String,
        nse_read_only: bool,
        room_load_settings: Option<RoomLoadSettings>,
    ) -> Result<SessionRestoreDto, SessionRestoreError> {
        let identity = AccountIdentity::new(&user_id, &homeserver_url)
            .map_err(|_| restore_failed(IDENTITY_INVALID_CODE, IDENTITY_INVALID_DESCRIPTION))?;
        let root = validate_store_root(&store_root)?;
        let claim = RestoreClaim::acquire(&self.restored_client)?;
        let vault = SecretStoreSessionVault {
            store: Arc::clone(&self.secret_store),
        };
        if vault
            .get(&SessionMaterialId::from_identity(&identity))
            .map_err(|_| restore_failed(VAULT_UNAVAILABLE_CODE, VAULT_UNAVAILABLE_DESCRIPTION))?
            .is_none()
        {
            return Err(restore_failed(
                MATERIAL_MISSING_CODE,
                MATERIAL_MISSING_DESCRIPTION,
            ));
        }

        let store_key = if nse_read_only {
            store_key_for_read_only(&self.secret_store, &identity)?
        } else {
            store_key_for(&self.secret_store, &identity)?
        };
        let mut config =
            ClientBuildConfig::product_default(root, identity.clone(), Some(store_key))
                .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
        if nse_read_only {
            config = config
                .with_timeouts(TimeoutPolicy {
                    request_timeout: NSE_REQUEST_TIMEOUT,
                    retry_limit: 0,
                })
                .and_then(|config| {
                    config.with_cross_process_store_lock_holder(NSE_STORE_LOCK_HOLDER)
                })
                .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
            config.handle_refresh_tokens = false;
        }
        let client = build_unauthenticated_client(&config)
            .await
            .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
        let persistence = SessionPersistenceOwner::new();
        if !nse_read_only {
            install_session_rotation_callbacks(
                &client,
                identity.clone(),
                Arc::clone(&self.secret_store),
                persistence.callback_lease(),
            )
            .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
        }
        let outcome = if let Some(room_load_settings) = room_load_settings {
            restore_session_from_vault_with_room_load_settings(
                &client,
                &identity,
                &vault,
                room_load_settings,
            )
            .await
        } else {
            restore_session_from_vault(&client, &identity, &vault).await
        }
        .map_err(|error| match error {
            crate::app::lifecycle::LifecycleError::Vault {
                diagnostic_id: "p3.6-session-material-missing",
                ..
            } => restore_failed(MATERIAL_MISSING_CODE, MATERIAL_MISSING_DESCRIPTION),
            _ => restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION),
        })?;

        let snapshot = SessionSnapshot {
            session_generation: self.allocate_session_generation(),
            user_id: outcome.meta.user_id.clone(),
            device_id: outcome.meta.device_id.clone(),
            homeserver_url: outcome.meta.homeserver_url.clone(),
            display_name: None,
            avatar_url: None,
            lifecycle: SessionLifecycle::Ready,
            crypto_ready: false,
        };
        self.core
            .open(snapshot)
            .await
            .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;

        if claim.commit(client, persistence).is_err() {
            let _ = self.core.close().await;
            return Err(restore_failed(
                RESTORE_FAILED_CODE,
                RESTORE_FAILED_DESCRIPTION,
            ));
        }

        Ok(SessionRestoreDto {
            user_id: outcome.meta.user_id,
            device_id: outcome.meta.device_id,
            homeserver_url: outcome.meta.homeserver_url,
        })
    }

    /// Password login through Core, persisted into the S3a vault for S3b restore.
    ///
    /// `password` is a dedicated FFI argument. It is never stored, never copied
    /// into the DTO, never echoed, and is zeroized when this frame returns.
    /// This is not `matrix_login_password` and does not attach owners.
    pub async fn login_with_password(
        &self,
        user_id: String,
        homeserver_url: String,
        store_root: String,
        password: String,
    ) -> Result<SessionLoginDto, SessionLoginError> {
        let password = Zeroizing::new(password);
        self.login_with_password_inner(&user_id, &homeserver_url, &store_root, password.as_str())
            .await
    }

    pub(super) async fn login_with_password_inner(
        &self,
        user_id: &str,
        homeserver_url: &str,
        store_root: &str,
        password: &str,
    ) -> Result<SessionLoginDto, SessionLoginError> {
        let identity = AccountIdentity::new(user_id, homeserver_url).map_err(|_| {
            login_failed(
                LOGIN_IDENTITY_INVALID_CODE,
                LOGIN_IDENTITY_INVALID_DESCRIPTION,
            )
        })?;
        let root = parse_store_root(store_root).map_err(|_| {
            login_failed(
                LOGIN_STORE_ROOT_INVALID_CODE,
                LOGIN_STORE_ROOT_INVALID_DESCRIPTION,
            )
        })?;
        if password.is_empty() {
            return Err(login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION));
        }
        let claim = RestoreClaim::acquire(&self.restored_client)
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let vault = SecretStoreSessionVault {
            store: Arc::clone(&self.secret_store),
        };
        let store_key =
            store_key_for(&self.secret_store, &identity).map_err(|error| match error {
                SessionRestoreError::Failed { code, .. } if code == VAULT_UNAVAILABLE_CODE => {
                    login_failed(
                        LOGIN_VAULT_UNAVAILABLE_CODE,
                        LOGIN_VAULT_UNAVAILABLE_DESCRIPTION,
                    )
                }
                _ => login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION),
            })?;
        let config = ClientBuildConfig::product_default(root, identity.clone(), Some(store_key))
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let existing_device_id = existing_sqlite_crypto_device_id(
            config.state_store_path(),
            config.store_passphrase_hex().as_deref(),
        )
        .await
        .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let client = build_unauthenticated_client(&config)
            .await
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let persistence = SessionPersistenceOwner::new();
        install_session_rotation_callbacks(
            &client,
            identity.clone(),
            Arc::clone(&self.secret_store),
            persistence.callback_lease(),
        )
        .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let outcome = core_login_with_password(
            &client,
            identity.user_id(),
            password,
            &LoginOptions {
                request_refresh_token: true,
                device_display_name: Some(DevicePlatform::Ios.device_display_name().to_owned()),
                device_id: existing_device_id,
            },
        )
        .await
        .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let live_identity = AccountIdentity::new(&outcome.user_id, &outcome.homeserver_url)
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        if live_identity != identity {
            return Err(login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION));
        }
        self.persist_open_and_retain(
            client,
            persistence,
            &live_identity,
            &vault,
            claim,
            outcome.device_id,
        )
        .await
    }

    /// Test-only persist+open+retain through the production login path.
    ///
    /// Plants a Matrix session on an unauthenticated Client (no homeserver),
    /// then calls the same `store_key_for` + `persist_session_after_login` +
    /// `Core::open` + retain sequence `login_with_password` uses. Not on UDL.
    #[doc(hidden)]
    pub async fn persist_planted_session_for_test(
        &self,
        user_id: String,
        homeserver_url: String,
        store_root: String,
        device_id: String,
        access_token: String,
        refresh_token: Option<String>,
    ) -> Result<SessionLoginDto, SessionLoginError> {
        let identity = AccountIdentity::new(&user_id, &homeserver_url).map_err(|_| {
            login_failed(
                LOGIN_IDENTITY_INVALID_CODE,
                LOGIN_IDENTITY_INVALID_DESCRIPTION,
            )
        })?;
        let root = parse_store_root(&store_root).map_err(|_| {
            login_failed(
                LOGIN_STORE_ROOT_INVALID_CODE,
                LOGIN_STORE_ROOT_INVALID_DESCRIPTION,
            )
        })?;
        let claim = RestoreClaim::acquire(&self.restored_client)
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let vault = SecretStoreSessionVault {
            store: Arc::clone(&self.secret_store),
        };
        let store_key =
            store_key_for(&self.secret_store, &identity).map_err(|error| match error {
                SessionRestoreError::Failed { code, .. } if code == VAULT_UNAVAILABLE_CODE => {
                    login_failed(
                        LOGIN_VAULT_UNAVAILABLE_CODE,
                        LOGIN_VAULT_UNAVAILABLE_DESCRIPTION,
                    )
                }
                _ => login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION),
            })?;
        let config = ClientBuildConfig::product_default(root, identity.clone(), Some(store_key))
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let client = build_unauthenticated_client(&config)
            .await
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let persistence = SessionPersistenceOwner::new();
        install_session_rotation_callbacks(
            &client,
            identity.clone(),
            Arc::clone(&self.secret_store),
            persistence.callback_lease(),
        )
        .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let material = SessionMaterial::from_matrix_tokens(
            &identity,
            &device_id,
            &access_token,
            refresh_token.as_deref(),
        )
        .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        restore_session_onto_client(&client, &identity, &material)
            .await
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        self.persist_open_and_retain(client, persistence, &identity, &vault, claim, device_id)
            .await
    }

    pub(super) async fn persist_open_and_retain(
        &self,
        client: Client,
        persistence: SessionPersistenceOwner,
        identity: &AccountIdentity,
        vault: &SecretStoreSessionVault,
        claim: RestoreClaim<'_>,
        device_id: String,
    ) -> Result<SessionLoginDto, SessionLoginError> {
        // The login save shares the rotation callbacks' fence.
        persistence
            .lease()
            .save(|| {
                persist_session_after_login(&client, identity, vault)
                    .map(|_| ())
                    .map_err(|_| SessionFault::unavailable(RotationDiagnostics::IOS.persist_failed))
            })
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;

        let snapshot = SessionSnapshot {
            session_generation: self.allocate_session_generation(),
            user_id: identity.user_id().to_owned(),
            device_id: device_id.clone(),
            homeserver_url: identity.homeserver_url().to_owned(),
            display_name: None,
            avatar_url: None,
            lifecycle: SessionLifecycle::Ready,
            crypto_ready: false,
        };
        self.core
            .open(snapshot)
            .await
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;

        if claim.commit(client, persistence).is_err() {
            let _ = self.core.close().await;
            return Err(login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION));
        }

        Ok(SessionLoginDto {
            user_id: identity.user_id().to_owned(),
            device_id,
            homeserver_url: identity.homeserver_url().to_owned(),
        })
    }

    /// Attach the desktop owner set on the retained Client. No Core.command.
    ///
    /// Builds owners with a queued timeline view-delta sink (P4-S14).
    /// Other product emits stay no-op. Platform::emit is still not used
    /// for product events. SyncService is attached but not started;
    /// P4-S12 `start_sync` starts it. Fail-closed if no Client is
    /// retained or owners are already attached.
    pub async fn attach_session_owners(&self) -> Result<SessionAttachDto, SessionAttachError> {
        if self.is_nse_read_only() {
            return Err(attach_failed(
                NSE_FORBIDS_ATTACH_CODE,
                NSE_FORBIDS_ATTACH_DESCRIPTION,
            ));
        }
        let claim = AttachClaim::acquire(&self.owner_attach)?;
        let client = {
            let guard = self
                .restored_client
                .lock()
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
            match &*guard {
                RestoredClientSlot::Ready(client, _) => client.clone(),
                RestoredClientSlot::Empty | RestoredClientSlot::InFlight => {
                    return Err(attach_failed(
                        ATTACH_SESSION_MISSING_CODE,
                        ATTACH_SESSION_MISSING_DESCRIPTION,
                    ));
                }
            }
        };
        let generation = self
            .core
            .session_snapshot()
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?
            .ok_or_else(|| {
                attach_failed(
                    ATTACH_SESSION_MISSING_CODE,
                    ATTACH_SESSION_MISSING_DESCRIPTION,
                )
            })?
            .session_generation;
        if generation == 0 {
            return Err(attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION));
        }

        let owner_updates = Arc::clone(&self.owner_updates);
        let typing_emit = {
            let queue = Arc::clone(&owner_updates);
            Arc::new(move |update: NativeTypingUpdateSignal| {
                push_owner_update(
                    &queue,
                    "typing",
                    update.session_generation,
                    Some(update.room_id),
                );
            })
        };
        let typing = Arc::new(
            NativeTypingOwner::with_emit(&client, typing_emit, generation)
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );
        let presence_emit = {
            let queue = Arc::clone(&owner_updates);
            Arc::new(move |update: NativePresenceUpdate| {
                let _ = update;
                push_owner_update(&queue, "presence", generation, None);
            })
        };
        let presence = Arc::new(
            NativePresenceOwner::start(&client, presence_emit, generation)
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );
        let rtc_transports = Arc::new(NativeRtcTransportsOwner::start(&client, generation));
        let user_status = Arc::new(NativeUserStatusOwner::start(&client, generation));
        let verification_emit = {
            let queue = Arc::clone(&owner_updates);
            Arc::new(move |update: NativeVerificationUpdateSignal| {
                push_owner_update(&queue, "verification", update.session_generation, None);
            })
        };
        // SharedCore is the iOS host. It cannot render the show-QR SVG, so
        // advertise SAS only and never generate a code the sheet cannot show.
        let verification = Arc::new(NativeVerificationOwner::with_show_qr(
            &client,
            verification_emit,
            generation,
            false,
        ));
        let devices_emit = {
            let queue = Arc::clone(&owner_updates);
            Arc::new(move |update: NativeDeviceUpdateSignal| {
                push_owner_update(&queue, "devices", update.session_generation, None);
            })
        };
        let devices = Arc::new(
            NativeDeviceOwner::start(&client, devices_emit, generation)
                .await
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );
        let dehydrated_emit = {
            let queue = Arc::clone(&owner_updates);
            Arc::new(move |update: NativeDeviceUpdateSignal| {
                push_owner_update(&queue, "devices", update.session_generation, None);
            })
        };
        let dehydrated_devices = Arc::new(
            NativeDehydratedDevicesOwner::start(&client, dehydrated_emit, generation).await,
        );
        let join_rules_emit = {
            let queue = Arc::clone(&owner_updates);
            Arc::new(move |update: NativeRoomJoinRuleUpdate| {
                let (session_generation, room_id) = match update {
                    NativeRoomJoinRuleUpdate::Ready {
                        room_id,
                        session_generation,
                        ..
                    }
                    | NativeRoomJoinRuleUpdate::Unavailable {
                        room_id,
                        session_generation,
                    } => (session_generation, Some(room_id)),
                };
                push_owner_update(&queue, "join_rules", session_generation, room_id);
            })
        };
        let join_rules = Arc::new(
            NativeRoomJoinRuleOwner::start(&client, join_rules_emit, generation)
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );
        let image_packs_emit = {
            let queue = Arc::clone(&owner_updates);
            Arc::new(move |update: NativeImagePackUpdateSignal| {
                if let Some(family) = account_data_owner_update_family(update.kind) {
                    push_owner_update(&queue, family, update.session_generation, None);
                }
            })
        };
        let image_packs = Arc::new(
            NativeImagePackOwner::start(&client, image_packs_emit, generation)
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );
        let http_pusher = Arc::new(
            NativeHttpPusherOwner::new(&client)
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );
        let timeline_updates = Arc::clone(&self.timeline_view_updates);
        let timeline_emit: TimelineViewUpdateEmit = Arc::new(move |batch| {
            if let Ok(mut guard) = timeline_updates.lock() {
                if guard.len() >= TIMELINE_VIEW_UPDATE_QUEUE_CAP {
                    guard.remove(0);
                }
                guard.push(batch);
            }
        });
        let timelines = Arc::new(NativeTimelineOwner::new(&client, timeline_emit, generation));
        let sync = Arc::new(
            build_sync_service(&client, generation, SyncServiceConfig::default())
                .await
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );

        self.core
            .attach_typing(typing)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_presence(presence)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_rtc_transports(rtc_transports)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_user_status(user_status)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_verification(verification)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_devices(devices)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_dehydrated_devices(dehydrated_devices)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_join_rules(join_rules)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_image_packs(image_packs)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_http_pusher(http_pusher)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_timelines(timelines)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_sync(sync)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;

        claim
            .commit()
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        Ok(SessionAttachDto {
            owners: ATTACHED_OWNER_NAMES
                .iter()
                .map(|name| (*name).to_owned())
                .collect(),
        })
    }

    /// Start the already-attached SyncService. Not `Core.command`.
    ///
    /// NSE forbids this. Missing attach fail-closes. Start failures stay
    /// static and never echo user id, homeserver, device id, or tokens.
    /// Reopens every retained Client store before starting sync. `resume()` is
    /// idempotent, so the first start and repeated foreground activation share
    /// one lifecycle route. A second start is a restart of the same owner.
    /// This is not iOS-on-engine and not P4 acceptance.
    pub async fn start_sync(&self) -> Result<SyncStartDto, SyncStartError> {
        if self.is_nse_read_only() {
            return Err(sync_start_failed(
                NSE_FORBIDS_START_CODE,
                NSE_FORBIDS_START_DESCRIPTION,
            ));
        }
        let _lifecycle = self.sync_lifecycle.lock().await;
        if !self.owners_attached() {
            return Err(sync_start_failed(
                SYNC_NOT_ATTACHED_CODE,
                SYNC_NOT_ATTACHED_DESCRIPTION,
            ));
        }
        let client = self.retained_client().map_err(|_| {
            sync_start_failed(CLIENT_RESUME_FAILED_CODE, CLIENT_RESUME_FAILED_DESCRIPTION)
        })?;
        client.resume().await.map_err(|_| {
            sync_start_failed(CLIENT_RESUME_FAILED_CODE, CLIENT_RESUME_FAILED_DESCRIPTION)
        })?;
        let snapshot = self.core.start_attached_sync().await.map_err(|code| {
            if code == SYNC_NOT_ATTACHED_CODE {
                sync_start_failed(SYNC_NOT_ATTACHED_CODE, SYNC_NOT_ATTACHED_DESCRIPTION)
            } else {
                sync_start_failed(SYNC_START_FAILED_CODE, SYNC_START_FAILED_DESCRIPTION)
            }
        })?;
        self.spawn_room_list_live();
        self.spawn_room_surface_owners();
        let snapshot = match self.core.attached_sync_owner() {
            Some(owner) => wait_for_started_readiness(owner.as_ref(), snapshot).await,
            None => snapshot,
        };
        Ok(sync_start_dto_from_snapshot(snapshot))
    }

    /// Quiesce the retained Client for iOS suspension without logging out or
    /// replacing the session owner set.
    ///
    /// Matrix SDK lifecycle order is authoritative: stop SyncService first,
    /// then `Client::pause()` to disable send queues, await every in-flight
    /// store operation, and release all SQLite connections and file locks.
    /// Returning `stopped = true` therefore means the complete persistence
    /// boundary is safe for OS suspension, not merely that network sync ended.
    pub async fn stop_sync(&self) -> Result<SyncStopDto, SyncStopError> {
        if self.is_nse_read_only() {
            return Err(sync_stop_failed(
                NSE_FORBIDS_STOP_CODE,
                NSE_FORBIDS_STOP_DESCRIPTION,
            ));
        }
        let _lifecycle = self.sync_lifecycle.lock().await;
        if !self.owners_attached() {
            return Err(sync_stop_failed(
                SYNC_NOT_ATTACHED_CODE,
                SYNC_NOT_ATTACHED_DESCRIPTION,
            ));
        }
        if let Ok(mut live) = self.room_list_live.lock() {
            *live = None;
        }
        if let Ok(mut live) = self.own_profile_live.lock() {
            *live = None;
        }
        if let Ok(mut live) = self.media_retention_live.lock() {
            *live = None;
        }
        let snapshot = self.core.stop_attached_sync().await.map_err(|code| {
            if code == SYNC_NOT_ATTACHED_CODE {
                sync_stop_failed(SYNC_NOT_ATTACHED_CODE, SYNC_NOT_ATTACHED_DESCRIPTION)
            } else {
                sync_stop_failed(SYNC_STOP_FAILED_CODE, SYNC_STOP_FAILED_DESCRIPTION)
            }
        })?;
        let client = self.retained_client().map_err(|_| {
            sync_stop_failed(CLIENT_PAUSE_FAILED_CODE, CLIENT_PAUSE_FAILED_DESCRIPTION)
        })?;
        client.pause().await.map_err(|_| {
            sync_stop_failed(CLIENT_PAUSE_FAILED_CODE, CLIENT_PAUSE_FAILED_DESCRIPTION)
        })?;
        Ok(sync_stop_dto_from_snapshot(snapshot))
    }

    pub async fn session_snapshot(&self) -> Result<SessionSnapshotDto, SessionStatusError> {
        let payload = self
            .session_status_command(SESSION_SNAPSHOT_COMMAND)
            .await?;
        session_snapshot_dto(payload)
    }

    pub async fn sync_status(&self) -> Result<SyncStatusDto, SessionStatusError> {
        // Swift polls this while the session is live; it is the iOS watchdog.
        self.retry_failed_session_save(std::time::Instant::now());
        if let Some(owner) = self.core.attached_sync_owner() {
            let observed = owner.observe();
            self.note_authentication_rejection(observed.failure_diagnostic_id);
            return sync_status_from_owner_snapshot(
                observed,
                self.core.attached_timeline_owner().is_some(),
            );
        }
        let payload = self.session_status_command(SYNC_STATUS_COMMAND).await?;
        sync_status_dto(payload)
    }

    pub async fn wipe_persisted_stores(
        &self,
        store_root: String,
    ) -> Result<LeftoverAckDto, LeftoverCommandError> {
        leftover_reject_oversize(store_root.len())?;
        let root = parse_store_root(&store_root).map_err(|_| {
            leftover_failed(
                LEFTOVER_STORE_ROOT_INVALID_CODE,
                LEFTOVER_STORE_ROOT_INVALID_DESCRIPTION,
            )
        })?;
        if root.exists() {
            std::fs::remove_dir_all(root)
                .map_err(|_| leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION))?;
        }
        Ok(LeftoverAckDto {
            status: "wiped".to_owned(),
        })
    }

    /// Best-effort remote revocation on the already-loaded, exact device.
    /// No store restore or new client is permitted on the logout route.
    pub async fn revoke_server_session(
        &self,
        user_id: String,
        device_id: String,
        homeserver_url: String,
    ) -> Result<bool, LeftoverCommandError> {
        let identity = AccountIdentity::new(&user_id, &homeserver_url)
            .map_err(|_| leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION))?;
        let Ok(client) = self.retained_client() else {
            return Ok(false);
        };
        let matches = client.user_id().map(|id| id.as_str()) == Some(identity.user_id())
            && client.device_id().map(|id| id.as_str()) == Some(device_id.as_str())
            && AccountIdentity::new(&user_id, client.homeserver().as_str())
                .ok()
                .as_ref()
                == Some(&identity);
        if !matches || self.is_nse_read_only() {
            return Err(leftover_failed(
                LEFTOVER_FAILED_CODE,
                LEFTOVER_FAILED_DESCRIPTION,
            ));
        }
        // A generation whose refresh was rejected never POSTs `/logout` with
        // its credentials (same policy as desktop `remote_logout_allowed`).
        let failure = self
            .core
            .attached_sync_owner()
            .and_then(|owner| owner.observe().failure_diagnostic_id);
        if !session_policy::remote_logout_allowed(None, failure) {
            return Ok(false);
        }
        Ok(matches!(
            tokio::time::timeout(
                std::time::Duration::from_secs(5),
                client.matrix_auth().logout()
            )
            .await,
            Ok(Ok(_))
        ))
    }

    /// Remove account authentication even when its SDK store cannot restore.
    /// The encrypted history/key are retained for a later password login.
    pub async fn forget_session(
        &self,
        user_id: String,
        homeserver_url: String,
    ) -> Result<LeftoverAckDto, LeftoverCommandError> {
        let identity = AccountIdentity::new(&user_id, &homeserver_url)
            .map_err(|_| leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION))?;
        if self.is_nse_read_only() {
            return Err(leftover_failed(
                LEFTOVER_FAILED_CODE,
                LEFTOVER_FAILED_DESCRIPTION,
            ));
        }
        if let Some(snapshot) = self
            .core
            .session_snapshot()
            .map_err(|_| leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION))?
        {
            if AccountIdentity::new(&snapshot.user_id, &snapshot.homeserver_url)
                .ok()
                .as_ref()
                != Some(&identity)
            {
                return Err(leftover_failed(
                    LEFTOVER_FAILED_CODE,
                    LEFTOVER_FAILED_DESCRIPTION,
                ));
            }
        }
        self.logout().await?;
        self.secret_store
            .delete(SessionMaterialId::from_identity(&identity).account())
            .map_err(|_| leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION))?;
        Ok(LeftoverAckDto {
            status: "forgotten".to_owned(),
        })
    }

    /// Re-save the retained client's current in-memory tokens after a failed
    /// rotation save, at most once per backoff window. It never replays an old
    /// refresh token, and a revoked lease (logout, forget) makes it a no-op.
    /// Returns whether a retry ran and succeeded.
    pub(super) fn retry_failed_session_save(&self, now: std::time::Instant) -> Option<bool> {
        let (client, lease) = match &*self.restored_client.lock().ok()? {
            RestoredClientSlot::Ready(client, persistence)
                if persistence.lease().save_failed() && !persistence.lease().is_revoked() =>
            {
                (client.clone(), persistence.callback_lease())
            }
            _ => return None,
        };
        let mut backoff = self.save_retry_backoff.lock().ok()?;
        if !backoff.ready(now) {
            return None;
        }
        let snapshot = self.core.session_snapshot().ok().flatten()?;
        let identity = AccountIdentity::new(&snapshot.user_id, &snapshot.homeserver_url).ok()?;
        let vault = SecretStoreSessionVault {
            store: Arc::clone(&self.secret_store),
        };
        let saved = lease
            .save(|| {
                persist_session_after_login(&client, &identity, &vault)
                    .map(|_| ())
                    .map_err(|_| SessionFault::unavailable(RotationDiagnostics::IOS.persist_failed))
            })
            .is_ok();
        backoff.record(now, saved);
        Some(saved)
    }

    /// Permanently reject further credential writes from the retained client.
    pub(super) fn revoke_retained_persistence(&self) {
        if let Ok(guard) = self.restored_client.lock() {
            if let RestoredClientSlot::Ready(_, persistence) = &*guard {
                persistence.lease().revoke();
            }
        }
    }

    /// Retire a generation whose refresh token the homeserver rejected.
    ///
    /// Generation-fenced: it acts only while that generation is installed and
    /// its sync owner still reports the authentication rejection. It stops
    /// sync, revokes the persistence lease, closes Core and forgets the vault
    /// session material, keeping the store key and encrypted history. It never
    /// contacts the homeserver. Calling it again after retirement is a no-op.
    pub async fn retire_rejected_session(
        &self,
        session_generation: u64,
    ) -> Result<LeftoverAckDto, LeftoverCommandError> {
        let failed = || leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION);
        let retired = || LeftoverAckDto {
            status: "retired".to_owned(),
        };
        if self.is_nse_read_only() {
            return Err(failed());
        }
        // Latch a rejection the live owner reports now, before any teardown.
        if let Some(owner) = self.core.attached_sync_owner() {
            self.note_authentication_rejection(owner.observe().failure_diagnostic_id);
        }
        let latched = self
            .rejected_session
            .lock()
            .map_err(|_| failed())?
            .clone()
            .filter(|(generation, _)| *generation == session_generation);
        let snapshot = self.core.session_snapshot().map_err(|_| failed())?;
        let identity = match (&snapshot, latched) {
            // A different live generation, or one that never reported the
            // rejection, is refused and left alone.
            (Some(snapshot), _) if snapshot.session_generation != session_generation => {
                return Err(failed());
            }
            (Some(_), None) => return Err(failed()),
            (_, Some((_, identity))) => identity,
            // No live session and nothing latched for it: already retired.
            (None, None) => return Ok(retired()),
        };
        if snapshot.is_some() {
            // Local teardown only: logout() never contacts the homeserver.
            self.logout().await?;
        }
        // Forgetting is idempotent, so a repeat call after a partial failure
        // finishes the job. The store key and encrypted history stay.
        self.secret_store
            .delete(SessionMaterialId::from_identity(&identity).account())
            .map_err(|_| failed())?;
        Ok(retired())
    }

    /// Remember the current generation when its sync reported a rejected
    /// refresh. A later generation simply never matches the latch.
    pub(crate) fn note_authentication_rejection(&self, failure_diagnostic_id: Option<&str>) {
        if failure_diagnostic_id != Some(SYNC_AUTHENTICATION_FAILURE_DIAGNOSTIC_ID) {
            return;
        }
        let Ok(Some(snapshot)) = self.core.session_snapshot() else {
            return;
        };
        let Ok(identity) = AccountIdentity::new(&snapshot.user_id, &snapshot.homeserver_url) else {
            return;
        };
        if let Ok(mut latch) = self.rejected_session.lock() {
            *latch = Some((snapshot.session_generation, identity));
        }
    }

    pub async fn logout(&self) -> Result<LeftoverAckDto, LeftoverCommandError> {
        // Serialize teardown with foreground resume and release every store
        // before dropping ownership. This operation performs no remote logout.
        let _lifecycle = self.sync_lifecycle.lock().await;
        // Fence token-rotation saves before teardown: a refresh that lands
        // while sync stops must not write credentials back into the vault.
        self.revoke_retained_persistence();
        if self.owners_attached() {
            self.core
                .stop_attached_sync()
                .await
                .map_err(|_| leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION))?;
        }
        if let Ok(client) = self.retained_client() {
            client
                .pause()
                .await
                .map_err(|_| leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION))?;
        }
        self.core
            .close()
            .await
            .map_err(|_| leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION))?;
        if let Ok(mut live) = self.room_list_live.lock() {
            *live = None;
        }
        if let Ok(mut live) = self.own_profile_live.lock() {
            *live = None;
        }
        if let Ok(mut live) = self.media_retention_live.lock() {
            *live = None;
        }
        if let Ok(mut updates) = self.timeline_view_updates.lock() {
            updates.clear();
        }
        if let Ok(mut updates) = self.room_list_updates.lock() {
            updates.clear();
        }
        if let Ok(mut updates) = self.owner_updates.lock() {
            updates.clear();
        }
        let mut guard = self
            .restored_client
            .lock()
            .map_err(|_| leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION))?;
        *guard = RestoredClientSlot::Empty;
        drop(guard);
        let mut attach = self
            .owner_attach
            .lock()
            .map_err(|_| leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION))?;
        *attach = OwnerAttachSlot::Empty;
        Ok(LeftoverAckDto {
            status: "logged_out".to_owned(),
        })
    }

    pub async fn recover(
        &self,
        recovery_key: String,
    ) -> Result<LeftoverAckDto, LeftoverCommandError> {
        let recovery_key = Zeroizing::new(recovery_key);
        leftover_reject_oversize(recovery_key.len())?;
        if recovery_key.is_empty() {
            return Err(leftover_failed(
                LEFTOVER_FAILED_CODE,
                LEFTOVER_FAILED_DESCRIPTION,
            ));
        }
        // Recovery requires live secret-storage I/O. Planted tests stay
        // fail-closed and never echo the recovery key.
        Err(leftover_failed(
            LEFTOVER_UNAVAILABLE_CODE,
            LEFTOVER_UNAVAILABLE_DESCRIPTION,
        ))
    }

    pub(super) async fn leftover_status_command(
        &self,
        command: &'static str,
    ) -> Result<serde_json::Value, LeftoverCommandError> {
        let payload = leftover_status_envelope_payload(serde_json::Value::Null)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: LEFTOVER_STATUS_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(map_leftover_status_core_error)?;
        Ok(response.payload)
    }

    pub(super) fn has_retained_client(&self) -> bool {
        self.restored_client
            .lock()
            .map(|guard| matches!(*guard, RestoredClientSlot::Ready(..)))
            .unwrap_or(false)
    }

    pub(super) fn owners_attached(&self) -> bool {
        self.owner_attach
            .lock()
            .map(|guard| matches!(*guard, OwnerAttachSlot::Ready))
            .unwrap_or(false)
    }

    pub(super) fn retained_client(&self) -> Result<Client, NseStoreError> {
        let guard = self
            .restored_client
            .lock()
            .map_err(|_| nse_failed(NSE_FAILED_CODE, NSE_FAILED_DESCRIPTION))?;
        match &*guard {
            RestoredClientSlot::Ready(client, _) => Ok(client.clone()),
            RestoredClientSlot::Empty | RestoredClientSlot::InFlight => Err(nse_failed(
                NSE_STORE_NOT_OPEN_CODE,
                NSE_STORE_NOT_OPEN_DESCRIPTION,
            )),
        }
    }

    pub(super) async fn session_status_command(
        &self,
        command: &'static str,
    ) -> Result<serde_json::Value, SessionStatusError> {
        let payload = session_status_envelope_payload(serde_json::Value::Null)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: SESSION_STATUS_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(map_session_status_core_error)?;
        Ok(response.payload)
    }
}
