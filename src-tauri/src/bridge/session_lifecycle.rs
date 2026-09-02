//! Safe desktop-session lifecycle bridge for the managed shared Core.
//!
//! The shared Core owns Matrix SDK clients and session lifecycle. The desktop
//! retains only the platform credential/vault boundary and installs a
//! credential-free session projection into that Core.

use serde::{de::DeserializeOwned, Deserialize};
use synara_core::app::sync::{
    SyncReadiness, SyncReadinessSnapshot, SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID,
};
use synara_core::dto::{SessionLifecycle, SessionSnapshot};
use synara_core::transport::CommandEnvelope;
use synara_core::Core;

use crate::matrix::auth::product::{
    MatrixAuthCommandError, MatrixCrossSigningState, MatrixCryptoStatus, MatrixLoginIdentity,
};

const SESSION_SNAPSHOT_COMMAND: &str = "matrix_session_snapshot";
const SYNC_STATUS_COMMAND: &str = "matrix_sync_status";
const CRYPTO_STATUS_COMMAND: &str = "matrix_crypto_status";

/// Owned wire form used only while the desktop bridge deserializes the Core
/// response. The core DTO keeps its diagnostic lifetime static so a platform
/// cannot attach a dynamic raw SDK error. This bridge validates that invariant
/// before reconstructing the existing desktop response type.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SyncStatusWireResponse {
    readiness: SyncReadiness,
    session_generation: u64,
    offline_mode_enabled: bool,
    failure_diagnostic_id: Option<String>,
    #[serde(default)]
    sliding_sync_capable: Option<bool>,
}

impl TryFrom<SyncStatusWireResponse> for SyncReadinessSnapshot {
    type Error = ();

    fn try_from(response: SyncStatusWireResponse) -> Result<Self, Self::Error> {
        let failure_diagnostic_id = match response.failure_diagnostic_id.as_deref() {
            None => None,
            Some(SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID) => Some(SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID),
            Some(_) => return Err(()),
        };
        Ok(Self {
            readiness: response.readiness,
            session_generation: response.session_generation,
            offline_mode_enabled: response.offline_mode_enabled,
            failure_diagnostic_id,
            sliding_sync_capable: response.sliding_sync_capable,
        })
    }
}

/// Closed response vocabulary accepted from Core for `matrix_crypto_status`.
/// This is a bridge-only parser, not a Platform projection: it proves the Core
/// response still has the exact legacy values before rebuilding the desktop DTO.
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum CryptoCrossSigningStateWire {
    Unavailable,
    NotSetUp,
    Partial,
    Ready,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CryptoStatusWireResponse {
    session_generation: u64,
    encryption_enabled: bool,
    cross_signing_state: CryptoCrossSigningStateWire,
}

impl TryFrom<CryptoStatusWireResponse> for MatrixCryptoStatus {
    type Error = ();

    fn try_from(response: CryptoStatusWireResponse) -> Result<Self, Self::Error> {
        let cross_signing_state = match response.cross_signing_state {
            CryptoCrossSigningStateWire::Unavailable => MatrixCrossSigningState::Unavailable,
            CryptoCrossSigningStateWire::NotSetUp => MatrixCrossSigningState::NotSetUp,
            CryptoCrossSigningStateWire::Partial => MatrixCrossSigningState::Partial,
            CryptoCrossSigningStateWire::Ready => MatrixCrossSigningState::Ready,
        };
        let pairing_is_valid = matches!(
            (response.encryption_enabled, cross_signing_state),
            (false, MatrixCrossSigningState::Unavailable)
                | (true, MatrixCrossSigningState::NotSetUp)
                | (true, MatrixCrossSigningState::Partial)
                | (true, MatrixCrossSigningState::Ready)
        );
        pairing_is_valid
            .then_some(MatrixCryptoStatus {
                session_generation: response.session_generation,
                encryption_enabled: response.encryption_enabled,
                cross_signing_state,
            })
            .ok_or(())
    }
}

/// `matrix_session_snapshot` is read-only, so it has no live desktop request
/// generation to forward. Zero is a valid JSON-safe Core envelope generation.
const READ_ONLY_SESSION_GENERATION: u64 = 0;

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

/// Mirror a session only after the desktop caller has installed it and released
/// its async session mutex. No SDK client, vault material, or store location is
/// accepted by this boundary.
pub(crate) async fn open_after_desktop_session_install(
    core: &Core,
    identity: &MatrixLoginIdentity,
    session_generation: u64,
) -> Result<(), MatrixAuthCommandError> {
    core.open(installed_session_projection(identity, session_generation))
        .await
        .map_err(|_| core_lifecycle_error())
}

/// Clear Core only after the desktop caller has removed its live session and
/// released its async session mutex.
pub(crate) async fn close_after_desktop_session_removal(
    core: &Core,
) -> Result<(), MatrixAuthCommandError> {
    core.close().await.map_err(|_| core_lifecycle_error())
}

/// Forward the one read-only session command through Core's envelope registry.
/// The Core handler owns the exact React-compatible JSON shape; this adapter
/// only deserializes it back to the existing desktop command DTO.
pub(crate) async fn session_snapshot<Response>(
    core: &Core,
) -> Result<Response, MatrixAuthCommandError>
where
    Response: DeserializeOwned,
{
    let response = core
        .command(CommandEnvelope {
            command: SESSION_SNAPSHOT_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .map_err(|_| core_snapshot_error())?;
    serde_json::from_value(response.payload).map_err(|_| core_snapshot_response_error())
}

/// Forward the existing payload-free sync status command through Core. The
/// desktop Platform samples the live shell-owned sync owner; Core owns the
/// registry and wire response, and this bridge retains the exact Tauri DTO.
pub(crate) async fn sync_status(
    core: &Core,
) -> Result<SyncReadinessSnapshot, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: SYNC_STATUS_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .map_err(|_| core_sync_status_error())?;
    let response: SyncStatusWireResponse =
        serde_json::from_value(response.payload).map_err(|_| core_sync_status_response_error())?;
    response
        .try_into()
        .map_err(|_| core_sync_status_response_error())
}

/// Forward the existing payload-free crypto status command through Core. The
/// desktop Platform remains the sole SDK Client/crypto owner and samples its
/// fixed projection under the existing auth mutex; Core owns the envelope and
/// exact public DTO serialization.
pub(crate) async fn crypto_status(
    core: &Core,
) -> Result<MatrixCryptoStatus, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: CRYPTO_STATUS_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .map_err(|_| core_crypto_status_error())?;
    let response: CryptoStatusWireResponse = serde_json::from_value(response.payload)
        .map_err(|_| core_crypto_status_response_error())?;
    response
        .try_into()
        .map_err(|_| core_crypto_status_response_error())
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

fn core_snapshot_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native Matrix session snapshot is unavailable.",
        "snc-p3-2-session-snapshot-response-invalid",
    )
}

fn core_sync_status_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native Matrix sync status is unavailable.",
        "snc-p3-3-sync-status-core-failed",
    )
}

fn core_sync_status_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native Matrix sync status is unavailable.",
        "snc-p3-3-sync-status-response-invalid",
    )
}

fn core_crypto_status_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native Matrix crypto status is unavailable.",
        "snc-p3-4-crypto-status-core-failed",
    )
}

fn core_crypto_status_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native Matrix crypto status is unavailable.",
        "snc-p3-4-crypto-status-response-invalid",
    )
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use synara_core::dto::NotificationCandidate;
    use synara_core::platform::{Platform, PlatformStatus, SecretVault, UnavailableSecretVault};
    use synara_core::transport::{
        CommandFuture, CommandRegistry, MatrixIpcEnvelope, MatrixIpcError, MatrixIpcErrorCategory,
    };

    use super::*;

    struct TestPlatform;

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
                Ok(synara_core::platform::PlatformSyncStatus::new(
                    synara_core::app::sync::SyncReadiness::Unconfigured,
                    0,
                    false,
                    None,
                    None,
                )
                .expect("unconfigured status is a valid string-free projection"))
            })
        }

        fn crypto_status(&self) -> synara_core::platform::CryptoStatusFuture<'_> {
            Box::pin(async {
                Ok(synara_core::platform::PlatformCryptoStatus::new(
                    0,
                    false,
                    synara_core::platform::PlatformCryptoCrossSigningState::Unavailable,
                )
                .expect("unavailable is a valid string-free crypto projection"))
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
        let core = Core::new(Arc::new(TestPlatform));
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

    fn core_returning(
        command: &'static str,
        response_payload: serde_json::Value,
        forwarded: Arc<Mutex<Vec<CommandEnvelope>>>,
    ) -> Core {
        let mut registry = CommandRegistry::new();
        registry
            .register(command, move |_state, request| -> CommandFuture {
                forwarded.lock().expect("test capture lock").push(request);
                let response_payload = response_payload.clone();
                Box::pin(async move { Ok(response_payload) })
            })
            .expect("read-only command is in the desktop command census");
        Core::with_registry(Arc::new(TestPlatform), registry)
    }

    #[tokio::test]
    async fn snapshot_bridge_forwards_exact_core_envelope_and_react_json() {
        let forwarded = Arc::new(Mutex::new(Vec::new()));
        let payload = serde_json::json!({
            "status": "logged_in",
            "user_id": "@alice:example.org",
            "device_id": "DEVICE",
            "homeserver_url": "https://matrix.example.org",
            "sessionGeneration": 9,
        });
        let core = core_returning(
            SESSION_SNAPSHOT_COMMAND,
            payload.clone(),
            Arc::clone(&forwarded),
        );

        let snapshot: crate::matrix::auth::product::MatrixSessionSnapshot = session_snapshot(&core)
            .await
            .expect("known Core session response remains the desktop DTO");
        assert_eq!(serde_json::to_value(snapshot).unwrap(), payload);
        assert_eq!(
            forwarded.lock().unwrap().as_slice(),
            &[CommandEnvelope {
                command: SESSION_SNAPSHOT_COMMAND.to_owned(),
                session_generation: READ_ONLY_SESSION_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            }]
        );
    }

    #[tokio::test]
    async fn sync_status_bridge_forwards_exact_core_envelope_and_react_json() {
        let forwarded = Arc::new(Mutex::new(Vec::new()));
        let payload = serde_json::json!({
            "readiness": "failed",
            "sessionGeneration": 9,
            "offlineModeEnabled": true,
            "failureDiagnosticId": SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID,
            "slidingSyncCapable": true,
        });
        let core = core_returning(SYNC_STATUS_COMMAND, payload.clone(), Arc::clone(&forwarded));

        let snapshot = sync_status(&core)
            .await
            .expect("known Core status response remains the desktop DTO");
        assert_eq!(serde_json::to_value(snapshot).unwrap(), payload);
        assert_eq!(
            forwarded.lock().unwrap().as_slice(),
            &[CommandEnvelope {
                command: SYNC_STATUS_COMMAND.to_owned(),
                session_generation: READ_ONLY_SESSION_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            }]
        );
    }

    #[tokio::test]
    async fn crypto_status_bridge_forwards_exact_core_envelope_and_react_json() {
        let forwarded = Arc::new(Mutex::new(Vec::new()));
        let payload = serde_json::json!({
            "sessionGeneration": 9,
            "encryptionEnabled": true,
            "crossSigningState": "partial",
        });
        let core = core_returning(
            CRYPTO_STATUS_COMMAND,
            payload.clone(),
            Arc::clone(&forwarded),
        );

        let status = crypto_status(&core)
            .await
            .expect("known Core crypto response remains the desktop DTO");
        assert_eq!(serde_json::to_value(status).unwrap(), payload);
        assert_eq!(
            forwarded.lock().unwrap().as_slice(),
            &[CommandEnvelope {
                command: CRYPTO_STATUS_COMMAND.to_owned(),
                session_generation: READ_ONLY_SESSION_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            }]
        );
    }

    fn core_failing(command: &'static str, error: MatrixIpcError) -> Core {
        let mut registry = CommandRegistry::new();
        registry
            .register(command, move |_state, _request| -> CommandFuture {
                let error = error.clone();
                Box::pin(async move { Err(error) })
            })
            .expect("read-only command is in the desktop command census");
        Core::with_registry(Arc::new(TestPlatform), registry)
    }

    #[tokio::test]
    async fn snapshot_bridge_errors_are_static_and_do_not_reflect_private_core_data() {
        let private_text = "https://private.example token=secret password=secret";
        let core_error = MatrixIpcError {
            category: MatrixIpcErrorCategory::Unknown,
            message: Some(private_text.to_owned()),
            diagnostic_id: Some(private_text.to_owned()),
            retry_after_ms: Some(1),
            request_id: Some(private_text.to_owned()),
        };
        assert!(format!("{core_error:?}").contains(private_text));
        let core = core_failing(SESSION_SNAPSHOT_COMMAND, core_error);
        let core_failure =
            session_snapshot::<crate::matrix::auth::product::MatrixSessionSnapshot>(&core)
                .await
                .expect_err("Core errors map to a static desktop error");

        let malformed = core_returning(
            SESSION_SNAPSHOT_COMMAND,
            serde_json::json!({"status": "not_a_snapshot", "private": private_text}),
            Arc::new(Mutex::new(Vec::new())),
        );
        let response_failure =
            session_snapshot::<crate::matrix::auth::product::MatrixSessionSnapshot>(&malformed)
                .await
                .expect_err("malformed Core responses map to a static desktop error");

        assert_eq!(
            core_failure.diagnostic_id,
            "snc-p3-2-session-snapshot-core-failed"
        );
        assert_eq!(
            response_failure.diagnostic_id,
            "snc-p3-2-session-snapshot-response-invalid"
        );
        for error in [
            core_lifecycle_error(),
            core_failure,
            response_failure,
            core_snapshot_response_error(),
        ] {
            let json = serde_json::to_string(&error).expect("static error serializes");
            for forbidden in ["private.example", "token", "secret", "password"] {
                assert!(
                    !json.contains(forbidden),
                    "bridge error must not reflect private Core data: {forbidden}"
                );
            }
        }
    }

    #[tokio::test]
    async fn sync_status_bridge_errors_are_static_and_reject_untrusted_diagnostics() {
        let private_text = "https://private.example token=secret password=secret";
        let core_error = MatrixIpcError {
            category: MatrixIpcErrorCategory::Unknown,
            message: Some(private_text.to_owned()),
            diagnostic_id: Some(private_text.to_owned()),
            retry_after_ms: Some(1),
            request_id: Some(private_text.to_owned()),
        };
        let core_failure = sync_status(&core_failing(SYNC_STATUS_COMMAND, core_error))
            .await
            .expect_err("Core errors map to a static desktop status error");

        let malformed = core_returning(
            SYNC_STATUS_COMMAND,
            serde_json::json!({
                "readiness": "failed",
                "sessionGeneration": 9,
                "offlineModeEnabled": true,
                "failureDiagnosticId": private_text,
                "slidingSyncCapable": false,
            }),
            Arc::new(Mutex::new(Vec::new())),
        );
        let response_failure = sync_status(&malformed)
            .await
            .expect_err("untrusted Core status diagnostics must fail closed");

        assert_eq!(
            core_failure.diagnostic_id,
            "snc-p3-3-sync-status-core-failed"
        );
        assert_eq!(
            response_failure.diagnostic_id,
            "snc-p3-3-sync-status-response-invalid"
        );
        for error in [
            core_failure,
            response_failure,
            core_sync_status_error(),
            core_sync_status_response_error(),
        ] {
            let json = serde_json::to_string(&error).expect("static error serializes");
            for forbidden in ["private.example", "token", "secret", "password"] {
                assert!(
                    !json.contains(forbidden),
                    "status bridge error must not reflect private Core data: {forbidden}"
                );
            }
        }
    }

    #[tokio::test]
    async fn crypto_status_bridge_errors_are_static_and_reject_hostile_or_invalid_core_data() {
        let private_text = "https://private.example token=secret key=secret";
        let core_error = MatrixIpcError {
            category: MatrixIpcErrorCategory::Unknown,
            message: Some(private_text.to_owned()),
            diagnostic_id: Some(private_text.to_owned()),
            retry_after_ms: Some(1),
            request_id: Some(private_text.to_owned()),
        };
        let core_failure = crypto_status(&core_failing(CRYPTO_STATUS_COMMAND, core_error))
            .await
            .expect_err("Core errors map to a static desktop crypto error");

        let hostile = core_returning(
            CRYPTO_STATUS_COMMAND,
            serde_json::json!({
                "sessionGeneration": 9,
                "encryptionEnabled": true,
                "crossSigningState": private_text,
            }),
            Arc::new(Mutex::new(Vec::new())),
        );
        let hostile_failure = crypto_status(&hostile)
            .await
            .expect_err("unknown Core cross-signing text must fail closed");

        let inconsistent = core_returning(
            CRYPTO_STATUS_COMMAND,
            serde_json::json!({
                "sessionGeneration": 9,
                "encryptionEnabled": false,
                "crossSigningState": "ready",
            }),
            Arc::new(Mutex::new(Vec::new())),
        );
        let response_failure = crypto_status(&inconsistent)
            .await
            .expect_err("impossible encryption/state pairing must fail closed");

        assert_eq!(
            core_failure.diagnostic_id,
            "snc-p3-4-crypto-status-core-failed"
        );
        assert_eq!(
            hostile_failure.diagnostic_id,
            "snc-p3-4-crypto-status-response-invalid"
        );
        assert_eq!(
            response_failure.diagnostic_id,
            "snc-p3-4-crypto-status-response-invalid"
        );
        for error in [
            core_failure,
            hostile_failure,
            response_failure,
            core_crypto_status_error(),
            core_crypto_status_response_error(),
        ] {
            let json = serde_json::to_string(&error).expect("static error serializes");
            for forbidden in ["private.example", "token", "secret", "key"] {
                assert!(
                    !json.contains(forbidden),
                    "crypto bridge error must not reflect private Core data: {forbidden}"
                );
            }
        }
    }

    #[test]
    fn lifecycle_inventory_covers_every_desktop_session_install_and_clear_site() {
        let source = include_str!("../matrix/auth/product_commands.rs");
        assert_eq!(
            source.matches("*session = Some(ManagedMatrixSession {").count(),
            3,
            "password login, completed registration, and restore are the complete install inventory"
        );
        assert_eq!(
            source.matches("*session = None;").count(),
            1,
            "logout is the sole desktop session clear site"
        );
        assert_eq!(
            source
                .matches("open_after_desktop_session_install(")
                .count(),
            3,
            "every installed desktop session must mirror into Core"
        );
        assert_eq!(
            source
                .matches("close_after_desktop_session_removal(")
                .count(),
            2,
            "logout closes Core both when already logged out and after desktop session removal"
        );

        for command in ["matrix_login_password", "matrix_restore_session"] {
            let body = tauri_command_body(source, command);
            let install = body
                .find("*session = Some(ManagedMatrixSession {")
                .expect("direct desktop session install");
            let release = body.find("drop(session);").expect("session guard release");
            let mirror = body
                .find("open_after_desktop_session_install(")
                .expect("Core session mirror");
            assert!(
                install < release && release < mirror,
                "{command} must mirror only after installing and releasing the session guard"
            );
        }

        let register = tauri_command_body(source, "matrix_register");
        let register_install = register
            .find("install_session_from_register_secrets")
            .expect("completed registration install helper");
        let register_release = register
            .find("drop(session);")
            .expect("session guard release");
        let register_mirror = register
            .find("open_after_desktop_session_install(")
            .expect("Core session mirror");
        assert!(
            register_install < register_release && register_release < register_mirror,
            "completed registration must release the session guard before its Core mirror"
        );

        let logout = tauri_command_body(source, "matrix_logout");
        let clear = logout
            .find("*session = None;")
            .expect("desktop session clear");
        let release = logout
            .rfind("drop(session);")
            .expect("post-clear session guard release");
        let close = logout
            .rfind("close_after_desktop_session_removal(")
            .expect("post-clear Core session close");
        let deferred_cleanup = logout
            .find("clear_result?;")
            .expect("deferred cleanup result");
        assert!(
            clear < release && release < close && close < deferred_cleanup,
            "logout must close Core after desktop removal, without its mutex, before cleanup errors"
        );
    }

    fn tauri_command_body<'a>(source: &'a str, command: &str) -> &'a str {
        let signature = format!("pub async fn {command}(");
        let start = source.find(&signature).expect("Tauri command must exist");
        let after_signature = &source[start..];
        let end = after_signature
            .find("#[tauri::command]")
            .map(|offset| start + offset)
            .unwrap_or(source.len());
        &source[start..end]
    }
}
