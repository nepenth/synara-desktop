//! Desktop `Platform` adapter (P1.6 seam).
//!
//! Wraps the Tauri `AppHandle` behind [`synara_core::platform::Platform`] so the
//! shared native core can eventually route every OS sink through one trait.
//! **No behavior change in P1.6**: existing callers keep using `AppHandle`
//! directly; this adapter exists as the seam P2+ will call.

use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, Runtime};

use synara_core::dto::NotificationCandidate;
use synara_core::platform::{
    CrossSigningStatusFuture, CryptoStatusFuture, MediaConfigFuture, Platform,
    PlatformCryptoStatus, PlatformStatus, PlatformSyncStatus, SecretStorageStatusFuture,
    SecretVault, SyncStatusFuture, UnavailableSecretVault,
};
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

    /// Use the established desktop product identity for core HTTP probes.
    fn http_user_agent(&self) -> String {
        crate::matrix::client_builder::default_user_agent()
    }

    /// Observe sync through the shell-owned Matrix session. The local snapshot
    /// is normalized into a string-free Platform projection before it crosses
    /// into Core; the SDK client, credentials, store, and raw diagnostics stay
    /// inside `MatrixAuthState` on the desktop side.
    fn sync_status(&self) -> SyncStatusFuture<'_> {
        Box::pin(async move {
            let snapshot = self
                .app
                .state::<crate::matrix::auth::MatrixAuthState>()
                .sync_status_snapshot()
                .await;
            PlatformSyncStatus::from_desktop_snapshot(snapshot)
        })
    }

    /// Observe crypto through the shell-owned Matrix session. The desktop
    /// state samples `cross_signing_status` under its existing auth mutex and
    /// reduces it locally to a generation, boolean, and closed coarse state.
    /// No SDK object, raw error, identity, credential, key, or store crosses
    /// this Platform/Core boundary.
    fn crypto_status(&self) -> CryptoStatusFuture<'_> {
        Box::pin(async move {
            let projection: PlatformCryptoStatus = self
                .app
                .state::<crate::matrix::auth::MatrixAuthState>()
                .crypto_status_projection()
                .await;
            Ok(projection)
        })
    }

    /// Observe the exact legacy cross-signing status through the desktop-owned
    /// Matrix session. The auth state keeps its mutex across both existing SDK
    /// awaits and returns only a bounded generation plus closed enums/errors.
    fn cross_signing_status(&self) -> CrossSigningStatusFuture<'_> {
        Box::pin(async move {
            self.app
                .state::<crate::matrix::auth::MatrixAuthState>()
                .cross_signing_status_projection()
                .await
        })
    }

    /// Observe secret-storage status through the desktop-owned Matrix session.
    ///
    /// `MatrixAuthState` retains the existing auth mutex across the full
    /// observation and reduces the result locally to fixed booleans and closed
    /// enums. No SDK/session/key/store value or diagnostic crosses into Core.
    fn secret_storage_status(&self) -> SecretStorageStatusFuture<'_> {
        Box::pin(async move {
            self.app
                .state::<crate::matrix::auth::MatrixAuthState>()
                .secret_storage_status_projection()
                .await
        })
    }

    /// Read media configuration through the desktop-owned Matrix session.
    ///
    /// `MatrixAuthState` clones the live SDK client under its existing auth
    /// mutex, then preserves the old command's lock-release-before-load
    /// behavior while the SDK serves cache/network data. Only the bounded,
    /// string-free projection crosses into Core.
    fn media_config(&self) -> MediaConfigFuture<'_> {
        Box::pin(async move {
            self.app
                .state::<crate::matrix::auth::MatrixAuthState>()
                .media_config_projection()
                .await
        })
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
