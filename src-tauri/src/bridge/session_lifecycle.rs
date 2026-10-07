//! Safe desktop-session lifecycle bridge for the managed shared Core.
//!
//! The shared Core owns Matrix SDK clients and session lifecycle. The desktop
//! retains only the platform credential/vault boundary and installs a
//! credential-free session projection into that Core.

use synara_core::app::sync::SyncReadinessSnapshot;
use synara_core::dto::{SessionLifecycle, SessionSnapshot};
use synara_core::Core;

use crate::matrix::auth::product::{
    MatrixAuthCommandError, MatrixCryptoStatus, MatrixLoginIdentity, MatrixSessionSnapshot,
};

/// Build the sole safe projection which may cross from desktop session ownership
/// into Core. The identity DTO carries no credentials; profile fields are
/// deliberately absent because this lifecycle mirror needs only readiness state.
pub(crate) fn installed_session_projection(
    identity: &MatrixLoginIdentity,
    session_generation: u64,
) -> SessionSnapshot {
    SessionSnapshot {
        session_generation,
        user_id: identity.user_id.clone(),
        device_id: identity.device_id.clone(),
        homeserver_url: identity.homeserver_url.clone(),
        display_name: None,
        avatar_url: None,
        lifecycle: SessionLifecycle::Ready,
        crypto_ready: true,
    }
}

/// Mirror an installed session while the desktop caller retains its session
/// transition gate through Core opening, owner attachment, and any rollback.
/// Core opening does not call back into desktop auth. No SDK client, vault
/// material, or store location is accepted by this projection boundary.
pub(crate) async fn open_after_desktop_session_install(
    core: &Core,
    identity: &MatrixLoginIdentity,
    session_generation: u64,
) -> Result<(), MatrixAuthCommandError> {
    core.open(installed_session_projection(identity, session_generation))
        .await
        .map_err(|_| core_lifecycle_error())
}

/// Clear Core after desktop retirement. Logout calls this while it holds
/// `MatrixAuthState`'s transition gate but not the session mutex, so ordinary
/// commands keep failing closed on the empty slot while install, restore, and
/// another logout wait for the close. Install rollback still holds both. Core
/// closing does not call back into desktop auth.
pub(crate) async fn close_after_desktop_session_removal(
    core: &Core,
) -> Result<(), MatrixAuthCommandError> {
    core.close().await.map_err(|_| core_lifecycle_error())
}

/// Read the session snapshot from Core's typed API. Core owns the exact
/// React-compatible shape; the desktop no longer re-parses Core JSON.
pub(crate) fn session_snapshot(
    core: &Core,
) -> Result<MatrixSessionSnapshot, MatrixAuthCommandError> {
    core.session_status_snapshot()
        .map_err(|_| core_snapshot_error())
}

/// Read sync readiness and the command gate from Core's typed API. The
/// desktop Platform samples the live shell-owned sync owner.
pub(crate) async fn sync_status(
    core: &Core,
) -> Result<SyncReadinessSnapshot, MatrixAuthCommandError> {
    core.sync_status()
        .await
        .map_err(|_| core_sync_status_error())
}

/// Read crypto status from Core's typed API. The desktop Platform remains the
/// sole SDK Client/crypto owner; Core validates the closed projection.
pub(crate) async fn crypto_status(
    core: &Core,
) -> Result<MatrixCryptoStatus, MatrixAuthCommandError> {
    core.crypto_status()
        .await
        .map_err(|_| core_crypto_status_error())
}

fn core_lifecycle_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native Matrix session state is unavailable.",
        "snc-p3-2-session-core-mirror-failed",
    )
}

fn core_snapshot_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native Matrix session snapshot is unavailable.",
        "snc-p3-2-session-snapshot-core-failed",
    )
}

fn core_sync_status_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native Matrix sync status is unavailable.",
        "snc-p3-3-sync-status-core-failed",
    )
}

fn core_crypto_status_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native Matrix crypto status is unavailable.",
        "snc-p3-4-crypto-status-core-failed",
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use synara_core::app::sync::{
        SyncReadiness, SYNC_AUTHENTICATION_FAILURE_DIAGNOSTIC_ID,
        SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID,
    };
    use synara_core::dto::NotificationCandidate;
    use synara_core::platform::{
        Platform, PlatformCryptoCrossSigningState, PlatformCryptoStatus, PlatformCryptoStatusError,
        PlatformStatus, PlatformSyncFailure, PlatformSyncStatus, PlatformSyncStatusError,
        SecretVault, UnavailableSecretVault,
    };
    use synara_core::transport::{MatrixIpcEnvelope, MatrixIpcError};

    use super::*;

    #[derive(Default)]
    struct TestPlatform {
        sync: Option<PlatformSyncStatus>,
        crypto: Option<PlatformCryptoStatus>,
        fail: bool,
    }

    impl Platform for TestPlatform {
        fn emit(&self, _envelope: MatrixIpcEnvelope) -> Result<(), MatrixIpcError> {
            Ok(())
        }

        fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync> {
            Arc::new(UnavailableSecretVault)
        }

        fn http_user_agent(&self) -> String {
            "Synara-Desktop-Session-Bridge-Test/1.0".to_owned()
        }

        fn sync_status(&self) -> synara_core::platform::SyncStatusFuture<'_> {
            Box::pin(async {
                if self.fail {
                    return Err(PlatformSyncStatusError::Unavailable);
                }
                Ok(self.sync.unwrap_or_else(|| {
                    PlatformSyncStatus::new(SyncReadiness::Unconfigured, 0, false, None, None)
                        .expect("unconfigured status is a valid string-free projection")
                }))
            })
        }

        fn crypto_status(&self) -> synara_core::platform::CryptoStatusFuture<'_> {
            Box::pin(async {
                if self.fail {
                    return Err(PlatformCryptoStatusError::InvalidSnapshot);
                }
                Ok(self.crypto.unwrap_or_else(|| {
                    PlatformCryptoStatus::new(
                        0,
                        false,
                        PlatformCryptoCrossSigningState::Unavailable,
                    )
                    .expect("unavailable is a valid string-free crypto projection")
                }))
            })
        }

        fn cross_signing_status(&self) -> synara_core::platform::CrossSigningStatusFuture<'_> {
            Box::pin(async {
                Err(synara_core::platform::PlatformCrossSigningStatusError::NoSession)
            })
        }

        fn media_config(&self) -> synara_core::platform::MediaConfigFuture<'_> {
            Box::pin(async {
                Ok(synara_core::platform::PlatformMediaConfig::new(0)
                    .expect("zero is a valid closed media projection"))
            })
        }

        fn notify(&self, _candidate: NotificationCandidate) -> Result<(), MatrixIpcError> {
            Ok(())
        }

        fn set_badge(&self, _count: u64) -> Result<(), MatrixIpcError> {
            Ok(())
        }

        fn status(&self, _status: PlatformStatus) -> Result<(), MatrixIpcError> {
            Ok(())
        }
    }

    fn identity() -> MatrixLoginIdentity {
        MatrixLoginIdentity {
            user_id: "@alice:example.org".to_owned(),
            device_id: "DEVICE".to_owned(),
            homeserver_url: "https://matrix.example.org".to_owned(),
        }
    }

    #[test]
    fn installed_projection_is_safe_and_has_only_lifecycle_readiness_data() {
        let projection = installed_session_projection(&identity(), 7);
        assert_eq!(projection.session_generation, 7);
        assert_eq!(projection.lifecycle, SessionLifecycle::Ready);
        assert!(projection.crypto_ready);
        assert!(projection.display_name.is_none());
        assert!(projection.avatar_url.is_none());

        let json = serde_json::to_string(&projection).expect("projection serializes");
        for forbidden in [
            "access_token",
            "accessToken",
            "refresh_token",
            "refreshToken",
            "password",
            "recovery_key",
            "private_key",
            "client",
            "store_path",
        ] {
            assert!(
                !json.contains(forbidden),
                "safe Core projection must not contain {forbidden}"
            );
        }
    }

    #[tokio::test]
    async fn lifecycle_mirror_opens_then_closes_the_same_core_projection() {
        let core = Core::new(Arc::new(TestPlatform::default()));
        open_after_desktop_session_install(&core, &identity(), 9)
            .await
            .expect("installed desktop session mirrors into Core");
        assert_eq!(
            core.session_snapshot().expect("Core state is readable"),
            Some(installed_session_projection(&identity(), 9))
        );

        close_after_desktop_session_removal(&core)
            .await
            .expect("removed desktop session closes Core");
        assert!(core
            .session_snapshot()
            .expect("Core state is readable")
            .is_none());
    }

    #[tokio::test]
    async fn snapshot_bridge_returns_the_core_session_snapshot_in_react_json() {
        let core = Core::new(Arc::new(TestPlatform::default()));
        let logged_out = session_snapshot(&core).expect("Core state is readable");
        assert_eq!(
            serde_json::to_value(logged_out).unwrap(),
            serde_json::json!({"status": "logged_out"})
        );

        open_after_desktop_session_install(&core, &identity(), 9)
            .await
            .expect("installed desktop session mirrors into Core");
        let logged_in = session_snapshot(&core).expect("Core state is readable");
        assert_eq!(
            serde_json::to_value(logged_in).unwrap(),
            serde_json::json!({
                "status": "logged_in",
                "user_id": "@alice:example.org",
                "device_id": "DEVICE",
                "homeserver_url": "https://matrix.example.org",
                "sessionGeneration": 9,
            })
        );
    }

    #[tokio::test]
    async fn sync_status_bridge_returns_core_status_and_gate_in_react_json() {
        for (failure, diagnostic) in [
            (
                PlatformSyncFailure::SyncService,
                SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID,
            ),
            (
                PlatformSyncFailure::AuthenticationRejected,
                SYNC_AUTHENTICATION_FAILURE_DIAGNOSTIC_ID,
            ),
        ] {
            let core = Core::new(Arc::new(TestPlatform {
                sync: Some(
                    PlatformSyncStatus::new(
                        SyncReadiness::Failed,
                        9,
                        true,
                        Some(failure),
                        Some(true),
                    )
                    .expect("failed status carries a closed failure"),
                ),
                ..TestPlatform::default()
            }));
            let snapshot = sync_status(&core)
                .await
                .expect("Core sync status is readable");
            // No timeline owner is attached, so Core reports the gate closed.
            assert_eq!(
                serde_json::to_value(snapshot).unwrap(),
                serde_json::json!({
                    "readiness": "failed",
                    "sessionGeneration": 9,
                    "offlineModeEnabled": true,
                    "failureDiagnosticId": diagnostic,
                    "slidingSyncCapable": true,
                    "commandGate": "closed",
                })
            );
        }
    }

    #[tokio::test]
    async fn crypto_status_bridge_returns_core_status_in_react_json() {
        let core = Core::new(Arc::new(TestPlatform {
            crypto: Some(
                PlatformCryptoStatus::new(9, true, PlatformCryptoCrossSigningState::Partial)
                    .expect("partial cross-signing is a valid encrypted projection"),
            ),
            ..TestPlatform::default()
        }));
        let status = crypto_status(&core)
            .await
            .expect("Core crypto status is readable");
        assert_eq!(
            serde_json::to_value(status).unwrap(),
            serde_json::json!({
                "sessionGeneration": 9,
                "encryptionEnabled": true,
                "crossSigningState": "partial",
            })
        );
    }

    #[tokio::test]
    async fn status_bridge_errors_are_static_desktop_errors() {
        let core = Core::new(Arc::new(TestPlatform {
            fail: true,
            ..TestPlatform::default()
        }));
        let sync_failure = sync_status(&core)
            .await
            .expect_err("Platform sync failures map to a static desktop error");
        let crypto_failure = crypto_status(&core)
            .await
            .expect_err("Platform crypto failures map to a static desktop error");
        assert_eq!(
            sync_failure.diagnostic_id,
            "snc-p3-3-sync-status-core-failed"
        );
        assert_eq!(
            crypto_failure.diagnostic_id,
            "snc-p3-4-crypto-status-core-failed"
        );
        for error in [
            core_lifecycle_error(),
            core_snapshot_error(),
            sync_failure,
            crypto_failure,
        ] {
            let json = serde_json::to_string(&error).expect("static error serializes");
            for forbidden in ["private.example", "token", "secret", "password"] {
                assert!(!json.contains(forbidden), "bridge error leaked {forbidden}");
            }
        }
    }
}
