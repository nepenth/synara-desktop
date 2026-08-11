//! Desktop `Platform` adapter (P1.6 seam).
//!
//! Wraps the Tauri `AppHandle` behind [`synara_core::platform::Platform`] so the
//! shared native core can eventually route every OS sink through one trait.
//! **No behavior change in P1.6**: existing callers keep using `AppHandle`
//! directly; this adapter exists as the seam P2+ will call.

use std::sync::Arc;

use tauri::{AppHandle, Emitter, Runtime};

use synara_core::dto::NotificationCandidate;
use synara_core::platform::{Platform, PlatformStatus, SecretVault, UnavailableSecretVault};
use synara_core::transport::{MatrixIpcEnvelope, MatrixIpcError, MatrixIpcErrorCategory};

use crate::desktop_notifications::{desktop_notify, DesktopNotificationPayload};
use crate::desktop_tray;

/// Desktop platform sink: a thin, audited facade over `AppHandle`.
pub struct TauriPlatform<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriPlatform<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> Platform for TauriPlatform<R> {
    /// Push one protocol envelope to the renderer stream.
    ///
    /// P1.6: envelope topic/body map 1:1 to the existing Tauri event names so
    /// renderer behavior is unchanged when P2 routes callers through this seam.
    fn emit(&self, envelope: MatrixIpcEnvelope) -> Result<(), MatrixIpcError> {
        let topic = envelope.kind();
        let body = envelope
            .to_json_value()
            .map_err(|_| MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant))?;
        self.app.emit(topic, body).map_err(|error| {
            MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                .with_diagnostic(error.to_string())
        })
    }

    /// Secret vault. P1.6: fail-closed stub — no reads/writes are attempted
    /// until the P2 keychain/Secret Service wiring replaces this.
    fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync> {
        Arc::new(UnavailableSecretVault)
    }

    /// Deliver a native notification through the existing desktop path.
    fn notify(&self, candidate: NotificationCandidate) -> Result<(), MatrixIpcError> {
        let payload = DesktopNotificationPayload {
            title: candidate.title,
            body: Some(candidate.body),
            route: candidate.route,
            actions: None,
            action_context: None,
        };
        desktop_notify(self.app.clone(), payload)
            .map(|_| ())
            .map_err(|error| {
                MatrixIpcError::new(MatrixIpcErrorCategory::Unknown).with_diagnostic(error)
            })
    }

    /// Update the dock/taskbar badge through the existing tray path.
    fn set_badge(&self, count: u64) -> Result<(), MatrixIpcError> {
        desktop_tray::set_badge_count(&self.app, Some(count as i64)).map_err(|error| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Unknown).with_diagnostic(error.to_string())
        })
    }

    /// Broadcast engine status. P1.6: no-op (no status consumer wired yet).
    fn status(&self, _status: PlatformStatus) -> Result<(), MatrixIpcError> {
        Ok(())
    }
}
