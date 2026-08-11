//! Platform sink seam (P1.6).
//!
//! Transport-agnostic OS services the shared engine needs. The desktop shell
//! implements [`Platform`] behind its Tauri `AppHandle`; the iOS shell will
//! implement it behind UIKit. P1.6 introduces the trait + desktop adapter with
//! **no behavior change** — current callers keep using `AppHandle` directly;
//! P2+ route the 38 `AppHandle`/`emit` references (census §2.2) through here.

use std::sync::Arc;

use crate::dto::NotificationCandidate;
use crate::transport::{MatrixIpcEnvelope, MatrixIpcError, MatrixIpcErrorCategory};

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
