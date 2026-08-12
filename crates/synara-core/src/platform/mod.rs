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
use crate::transport::{MatrixIpcEnvelope, MatrixIpcError, MatrixIpcErrorCategory};

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
/// Every shell provides all methods (no default impls in the crate);
/// fail-closed defaults where acceptable (e.g. `set_badge` no-op on unsupported
/// OS). P2 makes this (plus `synara_core::Core`) the only entry points shells use.
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
