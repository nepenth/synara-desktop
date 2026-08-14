//! Platform sink seam (P1.6).
//!
//! Transport-agnostic OS services the shared engine needs. The desktop shell
//! implements [`Platform`] behind its Tauri `AppHandle`; the iOS shell will
//! implement it behind UIKit. P1.6 introduces the trait + desktop adapter with
//! **no behavior change** — current callers keep using `AppHandle` directly;
//! P2+ route the 38 `AppHandle`/`emit` references (census §2.2) through here.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::app::sync::{SyncReadiness, SyncReadinessSnapshot, SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID};
use crate::dto::NotificationCandidate;
use crate::transport::{
    MatrixIpcEnvelope, MatrixIpcError, MatrixIpcErrorCategory, MAX_WIRE_COUNTER,
};

mod ios_fail_closed;
pub use ios_fail_closed::IosFailClosedPlatform;

/// Closed failure classification that may cross from a shell into Core.
///
/// This is deliberately not a diagnostic string. Core alone maps this enum to
/// the one public `matrix_sync_status` diagnostic id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformSyncFailure {
    /// The shell-observed SyncService is in its terminal error state.
    SyncService,
}

/// Static, opaque errors from the shell-owned sync observation.
///
/// Do not add a string payload here: a shell may have raw SDK diagnostics,
/// homeserver URLs, or credentials in its local error context, but none may
/// cross the Platform/Core seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformSyncStatusError {
    /// The shell could not take a sync observation.
    Unavailable,
    /// A shell DTO was inconsistent or carried an unapproved diagnostic id.
    InvalidSnapshot,
}

/// String-free sync-status projection supplied by a platform implementation.
///
/// The fields are private so a platform cannot construct an inconsistent
/// status, and the failure is a closed enum rather than a diagnostic string.
/// Core reconstructs its public DTO only after it receives this projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformSyncStatus {
    readiness: SyncReadiness,
    session_generation: u64,
    offline_mode_enabled: bool,
    failure: Option<PlatformSyncFailure>,
    sliding_sync_capable: Option<bool>,
}

impl PlatformSyncStatus {
    /// Construct a string-free projection, rejecting inconsistent readiness
    /// and failure combinations at the Platform/Core boundary.
    pub fn new(
        readiness: SyncReadiness,
        session_generation: u64,
        offline_mode_enabled: bool,
        failure: Option<PlatformSyncFailure>,
        sliding_sync_capable: Option<bool>,
    ) -> Result<Self, PlatformSyncStatusError> {
        let failure_is_valid = matches!(
            (readiness, failure),
            (
                SyncReadiness::Failed,
                Some(PlatformSyncFailure::SyncService)
            ) | (SyncReadiness::Unconfigured, None)
                | (SyncReadiness::Idle, None)
                | (SyncReadiness::Running, None)
                | (SyncReadiness::Offline, None)
                | (SyncReadiness::Terminated, None)
        );
        if !failure_is_valid {
            return Err(PlatformSyncStatusError::InvalidSnapshot);
        }

        Ok(Self {
            readiness,
            session_generation,
            offline_mode_enabled,
            failure,
            sliding_sync_capable,
        })
    }

    /// Normalize the existing desktop-only readiness DTO before it can cross
    /// the Platform/Core seam.
    ///
    /// The source can carry a `&'static str` for legacy desktop consumers, so
    /// accept exactly the one approved id and discard it into the closed enum.
    /// Any other value is rejected locally; it is never returned through
    /// [`Platform::sync_status`].
    pub fn from_desktop_snapshot(
        snapshot: SyncReadinessSnapshot,
    ) -> Result<Self, PlatformSyncStatusError> {
        let failure = match snapshot.failure_diagnostic_id {
            None => None,
            Some(SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID) => Some(PlatformSyncFailure::SyncService),
            Some(_) => return Err(PlatformSyncStatusError::InvalidSnapshot),
        };
        Self::new(
            snapshot.readiness,
            snapshot.session_generation,
            snapshot.offline_mode_enabled,
            failure,
            snapshot.sliding_sync_capable,
        )
    }

    pub fn readiness(self) -> SyncReadiness {
        self.readiness
    }

    pub fn session_generation(self) -> u64 {
        self.session_generation
    }

    pub fn offline_mode_enabled(self) -> bool {
        self.offline_mode_enabled
    }

    pub fn failure(self) -> Option<PlatformSyncFailure> {
        self.failure
    }

    pub fn sliding_sync_capable(self) -> Option<bool> {
        self.sliding_sync_capable
    }
}

/// Read-only, platform-provided sync-status observation.
///
/// The future may borrow its platform implementation while it reads a live
/// shell-owned SDK session. Its output is a string-free projection and a
/// static opaque error, so raw shell/SDK diagnostics cannot reach Core.
pub type SyncStatusFuture<'a> =
    Pin<Box<dyn Future<Output = Result<PlatformSyncStatus, PlatformSyncStatusError>> + Send + 'a>>;

/// Closed cross-signing state that may cross from a shell into Core.
///
/// This is deliberately not an SDK status object, identity, key, or string.
/// Core alone maps this enum to the exact public `crossSigningState` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformCryptoCrossSigningState {
    /// The shell has no encryption status to observe.
    Unavailable,
    /// Encryption is available but no cross-signing keys are configured.
    NotSetUp,
    /// At least one, but not all, cross-signing keys are configured.
    Partial,
    /// All cross-signing keys are configured.
    Ready,
}

/// Static, opaque error from the shell-owned crypto-status observation.
///
/// Do not add a string payload here. Raw SDK errors, identities, credentials,
/// and keys must remain in the shell and never cross the Platform/Core seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformCryptoStatusError {
    /// A shell projection had an inconsistent encryption/state pairing.
    InvalidSnapshot,
}

/// String-free crypto-status projection supplied by a platform implementation.
///
/// The fields remain private so a shell can pass only booleans and a closed
/// state enum to Core. Core owns construction and validation of the exact
/// React-facing response DTO.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformCryptoStatus {
    session_generation: u64,
    encryption_enabled: bool,
    cross_signing_state: PlatformCryptoCrossSigningState,
}

impl PlatformCryptoStatus {
    /// Construct a closed crypto projection, rejecting state combinations that
    /// could not have been produced by the existing desktop observation.
    pub fn new(
        session_generation: u64,
        encryption_enabled: bool,
        cross_signing_state: PlatformCryptoCrossSigningState,
    ) -> Result<Self, PlatformCryptoStatusError> {
        let encryption_pairing_is_valid = matches!(
            (encryption_enabled, cross_signing_state),
            (false, PlatformCryptoCrossSigningState::Unavailable)
                | (true, PlatformCryptoCrossSigningState::NotSetUp)
                | (true, PlatformCryptoCrossSigningState::Partial)
                | (true, PlatformCryptoCrossSigningState::Ready)
        );
        if !encryption_pairing_is_valid {
            return Err(PlatformCryptoStatusError::InvalidSnapshot);
        }

        Ok(Self {
            session_generation,
            encryption_enabled,
            cross_signing_state,
        })
    }

    pub fn session_generation(self) -> u64 {
        self.session_generation
    }

    pub fn encryption_enabled(self) -> bool {
        self.encryption_enabled
    }

    pub fn cross_signing_state(self) -> PlatformCryptoCrossSigningState {
        self.cross_signing_state
    }
}

/// Read-only, platform-provided crypto-status observation.
///
/// The future may borrow its platform implementation while it reads the live,
/// shell-owned SDK session. Its result is a closed, string-free projection, so
/// no raw SDK object, error, identity, credential, or key reaches Core.
pub type CryptoStatusFuture<'a> = Pin<
    Box<dyn Future<Output = Result<PlatformCryptoStatus, PlatformCryptoStatusError>> + Send + 'a>,
>;

/// Closed private cross-signing state that may cross from a shell into Core.
///
/// It records only which local private-signing-material condition the
/// desktop-owned SDK reported. It carries no key, identity, client, store,
/// credential, raw error, or recovery material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformCrossSigningPrivateState {
    /// The desktop SDK did not expose a cross-signing status.
    Unavailable,
    /// A status exists but no private cross-signing key is present.
    Missing,
    /// Some, but not all, private cross-signing keys are present.
    Partial,
    /// All private cross-signing keys are present.
    Complete,
}

/// Closed own-identity condition that may cross from a shell into Core.
///
/// This reports no identifier or key: it is only the result of the existing
/// desktop-owned identity query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformCrossSigningOwnIdentity {
    /// The existing identity query returned no own identity.
    Missing,
    /// The existing identity query returned an unverified own identity.
    Unverified,
    /// The existing identity query returned a verified own identity.
    Verified,
}

/// String-free `matrix_cross_signing_status` observation supplied by a shell.
///
/// Core reconstructs the exact legacy public fields and truth table from this
/// bounded generation plus two closed enums. The SDK client/crypto/store,
/// queried user id, identities, keys, credentials, and raw diagnostics remain
/// owned by the shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformCrossSigningStatus {
    session_generation: u64,
    private_state: PlatformCrossSigningPrivateState,
    own_identity: PlatformCrossSigningOwnIdentity,
}

impl PlatformCrossSigningStatus {
    /// Construct the only status observation permitted across this seam.
    /// JavaScript receives the generation on the legacy wire, so reject an
    /// unsafe counter before Core can serialize it.
    pub fn new(
        session_generation: u64,
        private_state: PlatformCrossSigningPrivateState,
        own_identity: PlatformCrossSigningOwnIdentity,
    ) -> Result<Self, PlatformCrossSigningStatusError> {
        (session_generation <= MAX_WIRE_COUNTER)
            .then_some(Self {
                session_generation,
                private_state,
                own_identity,
            })
            .ok_or(PlatformCrossSigningStatusError::UnsafeSessionGeneration)
    }

    pub fn session_generation(self) -> u64 {
        self.session_generation
    }

    pub fn private_state(self) -> PlatformCrossSigningPrivateState {
        self.private_state
    }

    pub fn own_identity(self) -> PlatformCrossSigningOwnIdentity {
        self.own_identity
    }
}

/// Static failures from the desktop-owned cross-signing observation.
///
/// Do not add a string payload. In particular, user ids, identities, keys,
/// secrets, SDK/client/store values, and raw diagnostics must not cross this
/// Platform/Core seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformCrossSigningStatusError {
    /// No desktop session owns a live Matrix SDK client.
    NoSession,
    /// The active SDK client anomalously did not expose its own user id.
    UserMissing,
    /// The existing desktop-owned own-identity query failed.
    IdentityQueryFailed,
    /// The active session generation is outside the JSON-safe wire range.
    UnsafeSessionGeneration,
}

/// Read-only cross-signing observation from the shell-owned Matrix SDK.
///
/// Implementations retain their existing auth mutex semantics while sampling
/// both the private status and the existing identity query. Only the closed
/// projection/error above reaches Core.
pub type CrossSigningStatusFuture<'a> = Pin<
    Box<
        dyn Future<Output = Result<PlatformCrossSigningStatus, PlatformCrossSigningStatusError>>
            + Send
            + 'a,
    >,
>;

/// Closed status vocabulary for the existing secret-storage observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformSecretStorageState {
    Unavailable,
    NotSetUp,
    Locked,
    Ready,
}

/// Closed action vocabulary for the existing secret-storage observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformSecretStorageAction {
    BootstrapRequired,
    UnlockRequired,
    None,
}

/// Fixed, scalar-only record of the four known missing-secret conditions.
///
/// This intentionally represents each known condition as a bit rather than a
/// list or identifier. Core reconstructs the legacy ordered public labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlatformSecretStorageMissingSecrets {
    cross_signing_master: bool,
    cross_signing_self_signing: bool,
    cross_signing_user_signing: bool,
    encryption_backup: bool,
}

impl PlatformSecretStorageMissingSecrets {
    pub fn new(
        cross_signing_master: bool,
        cross_signing_self_signing: bool,
        cross_signing_user_signing: bool,
        encryption_backup: bool,
    ) -> Self {
        Self {
            cross_signing_master,
            cross_signing_self_signing,
            cross_signing_user_signing,
            encryption_backup,
        }
    }

    pub fn cross_signing_master(self) -> bool {
        self.cross_signing_master
    }

    pub fn cross_signing_self_signing(self) -> bool {
        self.cross_signing_self_signing
    }

    pub fn cross_signing_user_signing(self) -> bool {
        self.cross_signing_user_signing
    }

    pub fn encryption_backup(self) -> bool {
        self.encryption_backup
    }
}

/// Static failures from the desktop-owned secret-storage observation.
///
/// This vocabulary is deliberately closed and string-free. In particular, no
/// secret, key, identifier, SDK error, or account-data value crosses this seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformSecretStorageStatusError {
    NoSession,
    DefaultKeyLoadFailed,
    KeyInfoLoadFailed,
    SecretCheckFailed,
    UnsafeSessionGeneration,
    InvalidSnapshot,
}

/// String-free, bounded secret-storage status supplied by a platform.
///
/// The shell samples its existing local observation and reduces it to fixed
/// booleans and closed enums. Core owns only validation and the legacy wire
/// DTO; it never receives any underlying secret-storage value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformSecretStorageStatus {
    session_generation: u64,
    state: PlatformSecretStorageState,
    exists: bool,
    unlocked: bool,
    default_key_set: bool,
    passphrase_configured: bool,
    bootstrap_ready: bool,
    missing_secrets: PlatformSecretStorageMissingSecrets,
    action: PlatformSecretStorageAction,
}

impl PlatformSecretStorageStatus {
    #[allow(clippy::too_many_arguments)] // Fixed legacy DTO fields are intentional.
    pub fn new(
        session_generation: u64,
        state: PlatformSecretStorageState,
        exists: bool,
        unlocked: bool,
        default_key_set: bool,
        passphrase_configured: bool,
        bootstrap_ready: bool,
        missing_secrets: PlatformSecretStorageMissingSecrets,
        action: PlatformSecretStorageAction,
    ) -> Result<Self, PlatformSecretStorageStatusError> {
        if session_generation > MAX_WIRE_COUNTER {
            return Err(PlatformSecretStorageStatusError::UnsafeSessionGeneration);
        }
        let state_pairing_is_valid = matches!(
            (state, unlocked, action),
            (
                PlatformSecretStorageState::Unavailable,
                false,
                PlatformSecretStorageAction::UnlockRequired,
            ) | (
                PlatformSecretStorageState::NotSetUp,
                false,
                PlatformSecretStorageAction::BootstrapRequired,
            ) | (
                PlatformSecretStorageState::Locked,
                false,
                PlatformSecretStorageAction::UnlockRequired,
            ) | (
                PlatformSecretStorageState::Ready,
                true,
                PlatformSecretStorageAction::None,
            )
        );
        state_pairing_is_valid
            .then_some(Self {
                session_generation,
                state,
                exists,
                unlocked,
                default_key_set,
                passphrase_configured,
                bootstrap_ready,
                missing_secrets,
                action,
            })
            .ok_or(PlatformSecretStorageStatusError::InvalidSnapshot)
    }

    pub fn session_generation(self) -> u64 {
        self.session_generation
    }

    pub fn state(self) -> PlatformSecretStorageState {
        self.state
    }

    pub fn exists(self) -> bool {
        self.exists
    }

    pub fn unlocked(self) -> bool {
        self.unlocked
    }

    pub fn default_key_set(self) -> bool {
        self.default_key_set
    }

    pub fn passphrase_configured(self) -> bool {
        self.passphrase_configured
    }

    pub fn bootstrap_ready(self) -> bool {
        self.bootstrap_ready
    }

    pub fn missing_secrets(self) -> PlatformSecretStorageMissingSecrets {
        self.missing_secrets
    }

    pub fn action(self) -> PlatformSecretStorageAction {
        self.action
    }
}

/// Read-only secret-storage observation from a shell-owned SDK session.
pub type SecretStorageStatusFuture<'a> = Pin<
    Box<
        dyn Future<Output = Result<PlatformSecretStorageStatus, PlatformSecretStorageStatusError>>
            + Send
            + 'a,
    >,
>;

/// Closed, scalar-only media-config projection supplied by a platform.
///
/// The maximum upload size is a JSON number on the existing React/Tauri wire,
/// so it is bounded by [`MAX_WIRE_COUNTER`] before Core receives it. The SDK
/// client, cache/store, homeserver details, and any SDK error stay owned by the
/// shell that created this projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformMediaConfig {
    upload_size: u64,
}

impl PlatformMediaConfig {
    /// Construct the only media-config value permitted across the Platform/Core
    /// seam. This rejects values that JavaScript could not represent exactly.
    pub fn new(upload_size: u64) -> Result<Self, PlatformMediaConfigError> {
        (upload_size <= MAX_WIRE_COUNTER)
            .then_some(Self { upload_size })
            .ok_or(PlatformMediaConfigError::UnsafeSize)
    }

    pub fn upload_size(self) -> u64 {
        self.upload_size
    }
}

/// Static failures from the desktop-owned media-config observation.
///
/// This vocabulary intentionally has no string, SDK, URL, credential, or key
/// payload. Core maps it to its own static transport errors; the desktop bridge
/// then restores the established public command diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformMediaConfigError {
    /// No desktop session owns a live Matrix client.
    NoSession,
    /// The SDK cache/network observation did not complete.
    LoadFailed,
    /// The SDK supplied a value outside the JSON-safe upload-size range.
    UnsafeSize,
}

/// Read-only media-config observation from the shell-owned Matrix client.
///
/// Implementations may retain or release their session mutex according to the
/// pre-Core command's concurrency contract, but must return only the closed
/// projection/error above. In particular, Core never receives an SDK client,
/// cache/store handle, raw SDK error, URL, credential, or key.
pub type MediaConfigFuture<'a> = Pin<
    Box<dyn Future<Output = Result<PlatformMediaConfig, PlatformMediaConfigError>> + Send + 'a>,
>;

/// Broad engine status broadcast to the OS layer (health/readiness).
///
/// The real consumption (dock/tray/today-widget on iOS) lands with the P2 sink
/// routing; the enumerant is stable so shells can match without recompiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformStatus {
    /// Engine booting; core subsystems not yet live.
    Booting,
    /// Engine live and handling normal traffic.
    Live,
    /// Engine reconnecting after a transport interruption.
    Reconnecting,
    /// Engine paused (e.g. suppressed by OS).
    Paused,
    /// Engine in a failed state; may need operator attention.
    Failed,
}

/// Key-value secret vault (session material, keys).
///
/// Fail-closed: any unavailable backend returns `Err` rather than silently
/// degrading. The desktop keychain (macOS) / Secret Service (Linux) plumbing
/// is wired by the shell's [`Platform`] implementation.
pub trait SecretVault: Send + Sync {
    /// Read a stored secret. `Ok(None)` when absent; `Err` on backend failure.
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, MatrixIpcError>;
    /// Persist a secret (create or overwrite).
    fn put(&self, key: &str, value: &[u8]) -> Result<(), MatrixIpcError>;
    /// Delete a secret. Missing keys are `Ok(())` (idempotent).
    fn delete(&self, key: &str) -> Result<(), MatrixIpcError>;
}

/// OS-surface sink the shared engine uses.
///
/// Every installed operation has a shell implementation. A newly added,
/// read-only observation may fail closed by default until a shell explicitly
/// opts in; P2 makes this (plus `synara_core::Core`) the only entry points
/// shells use.
pub trait Platform: Send + Sync + 'static {
    /// Push a protocol envelope onto the UI stream (sic the ipc protocol).
    fn emit(&self, envelope: MatrixIpcEnvelope) -> Result<(), MatrixIpcError>;
    /// Key-value secret vault (session material, keys).
    fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync>;
    /// Product identity for unauthenticated HTTP requests.
    ///
    /// This is a non-secret User-Agent value, not an authentication header,
    /// token, or other credential. Returning an owned string keeps the core
    /// independent of a shell's configuration and lifetime.
    fn http_user_agent(&self) -> String;
    /// Read the current sync state as a string-free safe projection.
    ///
    /// The shell remains the sole owner of its SDK client, credentials, stores,
    /// and raw diagnostics. Implementations must normalize their local DTO via
    /// [`PlatformSyncStatus::from_desktop_snapshot`] (or construct the closed
    /// projection directly) before returning; Core only owns the read-only
    /// transport command and its wire response.
    fn sync_status(&self) -> SyncStatusFuture<'_>;
    /// Read the current crypto state as a closed, string-free projection.
    ///
    /// The shell remains the sole owner of its SDK client, crypto state,
    /// credentials, stores, and raw errors. Implementations must reduce their
    /// local observation to [`PlatformCryptoStatus`] before returning; Core
    /// owns only the read-only transport command and its exact wire response.
    fn crypto_status(&self) -> CryptoStatusFuture<'_>;
    /// Read the existing desktop cross-signing status as a closed projection.
    ///
    /// The shell keeps the Matrix SDK client/crypto/store/network ownership and
    /// performs the existing own-identity query locally. Core receives only
    /// [`PlatformCrossSigningStatus`] or a static closed error.
    fn cross_signing_status(&self) -> CrossSigningStatusFuture<'_>;
    /// Read the existing secret-storage status as fixed booleans and enums.
    ///
    /// The desktop overrides this with its local observation. Other existing
    /// Platform implementors fail closed until they explicitly support this
    /// read-only command, so adding the P2 method never widens their surface.
    fn secret_storage_status(&self) -> SecretStorageStatusFuture<'_> {
        Box::pin(async { Err(PlatformSecretStorageStatusError::NoSession) })
    }
    /// Read the live media upload-size configuration as a closed projection.
    ///
    /// The shell retains the Matrix SDK client, authenticated session, cache,
    /// and store. It maps its local observation to [`PlatformMediaConfig`] or
    /// [`PlatformMediaConfigError`] before Core sees it.
    fn media_config(&self) -> MediaConfigFuture<'_>;
    /// Deliver a native notification (tray/toast on desktop, APNs/badge on iOS).
    fn notify(&self, candidate: NotificationCandidate) -> Result<(), MatrixIpcError>;
    /// App icon badge count (dock/taskbar/today on iOS).
    fn set_badge(&self, count: u64) -> Result<(), MatrixIpcError>;
    /// Broadcast engine status (health/readiness) to the OS layer.
    fn status(&self, status: PlatformStatus) -> Result<(), MatrixIpcError>;
}

/// Fail-closed [`SecretVault`] for shells that have not yet wired a backend.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnavailableSecretVault;

impl SecretVault for UnavailableSecretVault {
    fn get(&self, _key: &str) -> Result<Option<Vec<u8>>, MatrixIpcError> {
        Err(MatrixIpcError::new(
            MatrixIpcErrorCategory::StoreUnavailable,
        ))
    }
    fn put(&self, _key: &str, _value: &[u8]) -> Result<(), MatrixIpcError> {
        Err(MatrixIpcError::new(
            MatrixIpcErrorCategory::StoreUnavailable,
        ))
    }
    fn delete(&self, _key: &str) -> Result<(), MatrixIpcError> {
        Err(MatrixIpcError::new(
            MatrixIpcErrorCategory::StoreUnavailable,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_snapshot_normalization_rejects_private_diagnostic_before_seam() {
        let private_text: &'static str = Box::leak(
            "https://private.example token=secret"
                .to_owned()
                .into_boxed_str(),
        );
        let snapshot = SyncReadinessSnapshot {
            readiness: SyncReadiness::Failed,
            session_generation: 4,
            offline_mode_enabled: true,
            failure_diagnostic_id: Some(private_text),
            sliding_sync_capable: Some(false),
        };

        let result = PlatformSyncStatus::from_desktop_snapshot(snapshot);
        assert_eq!(result, Err(PlatformSyncStatusError::InvalidSnapshot));
        assert!(!format!("{result:?}").contains(private_text));
    }

    #[test]
    fn platform_sync_status_requires_the_closed_failure_pairing() {
        assert_eq!(
            PlatformSyncStatus::new(SyncReadiness::Failed, 4, true, None, None),
            Err(PlatformSyncStatusError::InvalidSnapshot)
        );
        assert_eq!(
            PlatformSyncStatus::new(
                SyncReadiness::Running,
                4,
                true,
                Some(PlatformSyncFailure::SyncService),
                None,
            ),
            Err(PlatformSyncStatusError::InvalidSnapshot)
        );
    }

    #[test]
    fn crypto_projection_is_closed_and_rejects_inconsistent_pairings() {
        let unavailable =
            PlatformCryptoStatus::new(4, false, PlatformCryptoCrossSigningState::Unavailable)
                .expect("unavailable is an existing crypto-status outcome");
        assert_eq!(unavailable.session_generation(), 4);
        assert!(!unavailable.encryption_enabled());
        assert_eq!(
            unavailable.cross_signing_state(),
            PlatformCryptoCrossSigningState::Unavailable
        );

        for state in [
            PlatformCryptoCrossSigningState::NotSetUp,
            PlatformCryptoCrossSigningState::Partial,
            PlatformCryptoCrossSigningState::Ready,
        ] {
            assert!(PlatformCryptoStatus::new(4, true, state).is_ok());
        }
        assert_eq!(
            PlatformCryptoStatus::new(4, false, PlatformCryptoCrossSigningState::Ready),
            Err(PlatformCryptoStatusError::InvalidSnapshot)
        );
        assert_eq!(
            PlatformCryptoStatus::new(4, true, PlatformCryptoCrossSigningState::Unavailable),
            Err(PlatformCryptoStatusError::InvalidSnapshot)
        );
    }

    #[test]
    fn cross_signing_projection_is_closed_bounded_and_covers_every_private_state() {
        for private_state in [
            PlatformCrossSigningPrivateState::Unavailable,
            PlatformCrossSigningPrivateState::Missing,
            PlatformCrossSigningPrivateState::Partial,
            PlatformCrossSigningPrivateState::Complete,
        ] {
            for own_identity in [
                PlatformCrossSigningOwnIdentity::Missing,
                PlatformCrossSigningOwnIdentity::Unverified,
                PlatformCrossSigningOwnIdentity::Verified,
            ] {
                let projection =
                    PlatformCrossSigningStatus::new(MAX_WIRE_COUNTER, private_state, own_identity)
                        .expect(
                            "all closed cross-signing state combinations are legacy observations",
                        );
                assert_eq!(projection.session_generation(), MAX_WIRE_COUNTER);
                assert_eq!(projection.private_state(), private_state);
                assert_eq!(projection.own_identity(), own_identity);
            }
        }
        assert_eq!(
            PlatformCrossSigningStatus::new(
                MAX_WIRE_COUNTER + 1,
                PlatformCrossSigningPrivateState::Complete,
                PlatformCrossSigningOwnIdentity::Verified,
            ),
            Err(PlatformCrossSigningStatusError::UnsafeSessionGeneration)
        );
    }

    #[test]
    fn cross_signing_platform_seam_has_no_dynamic_or_sdk_bearing_type() {
        let projection = PlatformCrossSigningStatus::new(
            4,
            PlatformCrossSigningPrivateState::Partial,
            PlatformCrossSigningOwnIdentity::Unverified,
        )
        .expect("closed cross-signing projection is valid");
        let private_text = "@alice:private.example token=secret key=secret";
        assert!(!format!("{projection:?}").contains(private_text));
        assert!(
            !format!("{:?}", PlatformCrossSigningStatusError::IdentityQueryFailed)
                .contains(private_text)
        );

        let source = include_str!("mod.rs");
        let seam = source
            .split("/// Closed private cross-signing state that may cross from a shell into Core.")
            .nth(1)
            .and_then(|section| {
                section
                    .split(
                        "/// Closed, scalar-only media-config projection supplied by a platform.",
                    )
                    .next()
            })
            .expect("cross-signing projection seam must remain isolated");
        for forbidden in [
            ": String",
            "String)",
            "String,",
            "String>",
            "&str",
            "MatrixIpcError",
            "matrix_sdk::",
        ] {
            assert!(
                !seam.contains(forbidden),
                "cross-signing Platform/Core seam must remain closed and string-free: {forbidden}"
            );
        }
    }

    #[test]
    fn secret_storage_projection_is_closed_bounded_and_covers_every_missing_secret_case() {
        let states = [
            (
                PlatformSecretStorageState::Unavailable,
                false,
                PlatformSecretStorageAction::UnlockRequired,
            ),
            (
                PlatformSecretStorageState::NotSetUp,
                false,
                PlatformSecretStorageAction::BootstrapRequired,
            ),
            (
                PlatformSecretStorageState::Locked,
                false,
                PlatformSecretStorageAction::UnlockRequired,
            ),
            (
                PlatformSecretStorageState::Ready,
                true,
                PlatformSecretStorageAction::None,
            ),
        ];
        for (state, unlocked, action) in states {
            for bits in 0_u8..16 {
                let missing = PlatformSecretStorageMissingSecrets::new(
                    bits & 1 != 0,
                    bits & 2 != 0,
                    bits & 4 != 0,
                    bits & 8 != 0,
                );
                let projection = PlatformSecretStorageStatus::new(
                    MAX_WIRE_COUNTER,
                    state,
                    true,
                    unlocked,
                    true,
                    true,
                    true,
                    missing,
                    action,
                )
                .expect("every fixed legacy missing-secret combination is valid");
                assert_eq!(projection.session_generation(), MAX_WIRE_COUNTER);
                assert_eq!(projection.state(), state);
                assert_eq!(projection.unlocked(), unlocked);
                assert_eq!(projection.action(), action);
                assert_eq!(projection.missing_secrets(), missing);
            }
        }
        assert_eq!(
            PlatformSecretStorageStatus::new(
                MAX_WIRE_COUNTER + 1,
                PlatformSecretStorageState::Ready,
                true,
                true,
                true,
                true,
                true,
                PlatformSecretStorageMissingSecrets::new(false, false, false, false),
                PlatformSecretStorageAction::None,
            ),
            Err(PlatformSecretStorageStatusError::UnsafeSessionGeneration)
        );
        assert_eq!(
            PlatformSecretStorageStatus::new(
                1,
                PlatformSecretStorageState::Ready,
                true,
                false,
                true,
                true,
                true,
                PlatformSecretStorageMissingSecrets::new(false, false, false, false),
                PlatformSecretStorageAction::None,
            ),
            Err(PlatformSecretStorageStatusError::InvalidSnapshot)
        );
    }

    #[test]
    fn secret_storage_platform_seam_has_no_dynamic_or_sdk_bearing_type() {
        let projection = PlatformSecretStorageStatus::new(
            4,
            PlatformSecretStorageState::Locked,
            true,
            false,
            true,
            true,
            false,
            PlatformSecretStorageMissingSecrets::new(true, false, true, false),
            PlatformSecretStorageAction::UnlockRequired,
        )
        .expect("closed secret-storage projection is valid");
        let private_text = "https://private.example token=secret recovery=key";
        assert!(!format!("{projection:?}").contains(private_text));
        for error in [
            PlatformSecretStorageStatusError::NoSession,
            PlatformSecretStorageStatusError::DefaultKeyLoadFailed,
            PlatformSecretStorageStatusError::KeyInfoLoadFailed,
            PlatformSecretStorageStatusError::SecretCheckFailed,
            PlatformSecretStorageStatusError::UnsafeSessionGeneration,
            PlatformSecretStorageStatusError::InvalidSnapshot,
        ] {
            assert!(!format!("{error:?}").contains(private_text));
        }

        let source = include_str!("mod.rs");
        let seam = source
            .split("/// Closed status vocabulary for the existing secret-storage observation.")
            .nth(1)
            .and_then(|section| {
                section
                    .split(
                        "/// Closed, scalar-only media-config projection supplied by a platform.",
                    )
                    .next()
            })
            .expect("secret-storage projection seam must remain isolated");
        for forbidden in [
            ": String",
            "String)",
            "String,",
            "String>",
            "&str",
            "Vec<",
            "HashMap",
            "MatrixIpcError",
            "matrix_sdk::",
        ] {
            assert!(
                !seam.contains(forbidden),
                "secret-storage Platform/Core seam must remain closed and string-free: {forbidden}"
            );
        }
    }

    #[test]
    fn media_config_projection_is_closed_and_bounded_to_the_wire_counter() {
        let at_limit = PlatformMediaConfig::new(MAX_WIRE_COUNTER)
            .expect("the maximum JavaScript-safe integer is a valid upload size");
        assert_eq!(at_limit.upload_size(), MAX_WIRE_COUNTER);
        assert_eq!(
            PlatformMediaConfig::new(MAX_WIRE_COUNTER + 1),
            Err(PlatformMediaConfigError::UnsafeSize)
        );

        let private_text = "https://private.example token=secret key=secret";
        assert!(!format!("{at_limit:?}").contains(private_text));
        assert!(!format!("{:?}", PlatformMediaConfigError::LoadFailed).contains(private_text));
    }

    #[test]
    fn media_config_platform_seam_has_no_dynamic_or_sdk_bearing_type() {
        let source = include_str!("mod.rs");
        let media_seam = source
            .split("/// Closed, scalar-only media-config projection supplied by a platform.")
            .nth(1)
            .and_then(|section| section.split("/// Broad engine status broadcast").next())
            .expect("media projection seam must remain isolated");
        for forbidden in [
            ": String",
            "String)",
            "String,",
            "String>",
            "&str",
            "MatrixIpcError",
            "matrix_sdk::",
        ] {
            assert!(
                !media_seam.contains(forbidden),
                "media Platform/Core seam must remain closed and string-free: {forbidden}"
            );
        }
    }

    #[test]
    fn crypto_projection_seam_has_no_dynamic_or_sdk_bearing_type() {
        let private_text = "https://private.example token=secret key=secret";
        let projection =
            PlatformCryptoStatus::new(4, true, PlatformCryptoCrossSigningState::Partial)
                .expect("closed projection is valid");
        assert!(!format!("{projection:?}").contains(private_text));

        let source = include_str!("mod.rs");
        let crypto_seam = source
            .split("/// Closed cross-signing state that may cross from a shell into Core.")
            .nth(1)
            .and_then(|section| section.split("/// Broad engine status broadcast").next())
            .expect("crypto projection seam must remain isolated");
        for forbidden in [
            ": String",
            "String)",
            "String,",
            "String>",
            "&str",
            "MatrixIpcError",
            "matrix_sdk::",
        ] {
            assert!(
                !crypto_seam.contains(forbidden),
                "crypto Platform/Core seam must remain closed and string-free: {forbidden}"
            );
        }
    }
}
