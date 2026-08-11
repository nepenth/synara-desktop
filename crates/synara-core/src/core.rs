//! Shared native-core entry points (P2 foundation).
//!
//! `Core` owns safe session projection/lifecycle plus the transport command
//! registry. It intentionally has no Tauri dependency; P2 command groups add
//! handlers, P3 makes the desktop shell a thin `Core::command` registrar.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::app::auth::{
    discover_login_flows, login_flows_response, probe_register_flows, AuthError,
    HttpLoginFlowTransport, HttpRegisterFlowTransport, MatrixLoginFlowsResponse,
    RegisterFlowsProbe,
};
use crate::app::sync::{SyncReadinessSnapshot, SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID};
use crate::dto::SessionSnapshot;
use crate::platform::{Platform, PlatformSyncFailure, PlatformSyncStatus};
use crate::transport::{
    CommandEnvelope, CommandFuture, CommandRegistry, CommandResponseEnvelope, MatrixIpcError,
    MatrixIpcErrorCategory,
};

/// React-compatible payload for `matrix_session_snapshot`.
///
/// This deliberately selects only the fields returned by the desktop command,
/// rather than serializing the broader safe session projection wholesale.
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum MatrixSessionSnapshotResponse {
    LoggedOut,
    LoggedIn {
        user_id: String,
        device_id: String,
        homeserver_url: String,
        #[serde(rename = "sessionGeneration")]
        session_generation: u64,
    },
}

impl From<Option<SessionSnapshot>> for MatrixSessionSnapshotResponse {
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

/// Exact React/Tauri envelope payload for `matrix_login_flows`.
///
/// The renderer sends the camel-case `homeserverUrl` key; unknown keys are
/// rejected so accidental credential fields do not cross this boundary.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixLoginFlowsRequest {
    homeserver_url: String,
}

/// Exact React/Tauri envelope payload for `matrix_register_flows`.
///
/// This read-only probe accepts exactly the existing camel-case homeserver
/// field and rejects all credential or UIAA-continuation fields at the core
/// boundary.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MatrixRegisterFlowsRequest {
    homeserver_url: String,
}

/// Internal state passed to command handlers. It never carries shell types.
/// Opaque state context supplied to registered core command handlers.
///
/// Shells never construct it; fields stay private so handlers use only stable
/// core accessors instead of reaching into platform/session ownership.
pub struct CoreState {
    platform: Arc<dyn Platform>,
    session: Mutex<Option<SessionSnapshot>>,
}

impl CoreState {
    pub fn platform(&self) -> Arc<dyn Platform> {
        Arc::clone(&self.platform)
    }

    pub fn session_snapshot(&self) -> Result<Option<SessionSnapshot>, MatrixIpcError> {
        self.session
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| core_state_error("p2-core-state-poisoned"))
    }
}

/// Platform-neutral native engine root.
pub struct Core {
    state: Arc<CoreState>,
    registry: CommandRegistry,
}

impl Core {
    /// Build a core with the built-in P2 command handlers. P3 shells
    /// instantiate this once at startup; [`Self::with_registry`] remains for
    /// explicit construction and handler-focused tests.
    pub fn new(platform: Arc<dyn Platform>) -> Self {
        Self::with_registry(platform, built_in_registry())
    }

    pub fn with_registry(platform: Arc<dyn Platform>, registry: CommandRegistry) -> Self {
        Self {
            state: Arc::new(CoreState {
                platform,
                session: Mutex::new(None),
            }),
            registry,
        }
    }

    /// Dispatch one validated `matrix_*` request to the registered core handler.
    pub async fn command(
        &self,
        request: CommandEnvelope,
    ) -> Result<CommandResponseEnvelope, MatrixIpcError> {
        request
            .validate()
            .map_err(|_| core_state_error("p2-command-invalid-envelope"))?;
        let handler = self
            .registry
            .handler(&request.command)
            .ok_or_else(|| core_state_error("p2-command-unregistered"))?;
        let response_payload = handler
            .handle(Arc::clone(&self.state), request.clone())
            .await?;
        Ok(request.response(response_payload))
    }

    /// Open a safe session projection. Credential material remains in the
    /// platform vault/session owner, never this DTO.
    pub async fn open(&self, session: SessionSnapshot) -> Result<(), MatrixIpcError> {
        let mut guard = self
            .state
            .session
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *guard = Some(session);
        Ok(())
    }

    /// Close the in-memory core projection. P2 deliberately does not erase
    /// platform persistence; lifecycle/destructive policies remain explicit.
    pub async fn close(&self) -> Result<(), MatrixIpcError> {
        let mut guard = self
            .state
            .session
            .lock()
            .map_err(|_| core_state_error("p2-core-state-poisoned"))?;
        *guard = None;
        Ok(())
    }

    pub fn session_snapshot(&self) -> Result<Option<SessionSnapshot>, MatrixIpcError> {
        self.state.session_snapshot()
    }

    pub fn registered_commands(&self) -> Vec<String> {
        self.registry.command_names()
    }
}

fn built_in_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();
    registry
        .register("matrix_session_snapshot", matrix_session_snapshot)
        .expect("built-in matrix_session_snapshot must remain in the command census");
    registry
        .register("matrix_sync_status", matrix_sync_status)
        .expect("built-in matrix_sync_status must remain in the command census");
    registry
        .register("matrix_login_flows", matrix_login_flows)
        .expect("built-in matrix_login_flows must remain in the command census");
    registry
        .register("matrix_register_flows", matrix_register_flows)
        .expect("built-in matrix_register_flows must remain in the command census");
    registry
}

fn matrix_session_snapshot(state: Arc<CoreState>, _request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let response = MatrixSessionSnapshotResponse::from(state.session_snapshot()?);
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-session-snapshot-serialization-failed"))
    })
}

/// Reconstruct the public status DTO from the string-free Platform projection.
///
/// This is the only Platform-to-public mapping: Core constructs the fixed
/// `p4.1-sync-service-error` value from the closed failure enum, then validates
/// the full DTO contract before it can be serialized.
fn public_sync_status(status: PlatformSyncStatus) -> Result<SyncReadinessSnapshot, MatrixIpcError> {
    let failure_diagnostic_id = match status.failure() {
        None => None,
        Some(PlatformSyncFailure::SyncService) => Some(SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID),
    };
    let snapshot = SyncReadinessSnapshot {
        readiness: status.readiness(),
        session_generation: status.session_generation(),
        offline_mode_enabled: status.offline_mode_enabled(),
        failure_diagnostic_id,
        sliding_sync_capable: status.sliding_sync_capable(),
    };
    snapshot
        .is_valid_public_sync_status()
        .then_some(snapshot)
        .ok_or_else(|| core_state_error("p2-sync-status-invalid-platform-projection"))
}

/// `matrix_sync_status` is deliberately a payload-free observation. Core owns
/// its registry entry and exact wire serialization; the Platform remains the
/// sole owner of the live SDK client from which it reads the safe projection.
fn matrix_sync_status(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-sync-status-invalid-payload"));
        }
        let platform = state.platform();
        let status = platform
            .sync_status()
            .await
            // Platform status errors are closed enums, and Core still exposes
            // only its static command error through this public observation.
            .map_err(|_| core_state_error("p2-sync-status-platform-unavailable"))?;
        let snapshot = public_sync_status(status)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-sync-status-serialization-failed"))
    })
}

fn matrix_login_flows(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixLoginFlowsRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-login-flows-invalid-payload"))?;
        let transport =
            HttpLoginFlowTransport::new_with_user_agent(state.platform().http_user_agent())
                .map_err(auth_transport_error)?;
        let result = discover_login_flows(&payload.homeserver_url, &transport)
            .await
            .map_err(auth_transport_error)?;
        let response: MatrixLoginFlowsResponse = login_flows_response(result.flows);
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-login-flows-serialization-failed"))
    })
}

fn matrix_register_flows(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRegisterFlowsRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-register-flows-invalid-payload"))?;
        let transport =
            HttpRegisterFlowTransport::new_with_user_agent(state.platform().http_user_agent())
                .map_err(auth_transport_error)?;
        let response: RegisterFlowsProbe =
            probe_register_flows(&payload.homeserver_url, &transport)
                .await
                .map_err(auth_transport_error)?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-register-flows-serialization-failed"))
    })
}

/// Convert the credential-free auth domain's static diagnostics into the
/// versioned core transport error shape. Never attach input URLs, HTTP bodies,
/// credentials, tokens, or a raw library error.
fn auth_transport_error(error: AuthError) -> MatrixIpcError {
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

fn core_state_error(diagnostic_id: &'static str) -> MatrixIpcError {
    MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant).with_diagnostic(diagnostic_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::sync::SyncReadiness;
    use crate::dto::{SessionLifecycle, SessionSnapshot};
    use crate::platform::{PlatformStatus, SecretVault, UnavailableSecretVault};
    use crate::transport::{CommandFuture, CommandRegistry};

    const TEST_HTTP_USER_AGENT: &str = "Synara-Core-Test/1.0";

    fn unconfigured_platform_status() -> PlatformSyncStatus {
        PlatformSyncStatus::new(SyncReadiness::Unconfigured, 0, false, None, None)
            .expect("unconfigured status is a valid string-free projection")
    }

    #[derive(Default)]
    struct TestPlatform;
    impl Platform for TestPlatform {
        fn emit(
            &self,
            _envelope: crate::transport::MatrixIpcEnvelope,
        ) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync> {
            Arc::new(UnavailableSecretVault)
        }
        fn http_user_agent(&self) -> String {
            TEST_HTTP_USER_AGENT.into()
        }
        fn sync_status(&self) -> crate::platform::SyncStatusFuture<'_> {
            Box::pin(async { Ok(unconfigured_platform_status()) })
        }
        fn notify(
            &self,
            _candidate: crate::dto::NotificationCandidate,
        ) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn set_badge(&self, _count: u64) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn status(&self, _status: PlatformStatus) -> Result<(), MatrixIpcError> {
            Ok(())
        }
    }

    /// A test shell may supply only the closed platform status/error types.
    /// It has no field in which a diagnostic string can enter Core.
    struct StatusPlatform {
        status: Result<PlatformSyncStatus, crate::platform::PlatformSyncStatusError>,
    }

    impl Platform for StatusPlatform {
        fn emit(
            &self,
            _envelope: crate::transport::MatrixIpcEnvelope,
        ) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync> {
            Arc::new(UnavailableSecretVault)
        }
        fn http_user_agent(&self) -> String {
            TEST_HTTP_USER_AGENT.into()
        }
        fn sync_status(&self) -> crate::platform::SyncStatusFuture<'_> {
            Box::pin(async move { self.status })
        }
        fn notify(
            &self,
            _candidate: crate::dto::NotificationCandidate,
        ) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn set_badge(&self, _count: u64) -> Result<(), MatrixIpcError> {
            Ok(())
        }
        fn status(&self, _status: PlatformStatus) -> Result<(), MatrixIpcError> {
            Ok(())
        }
    }

    fn session() -> SessionSnapshot {
        SessionSnapshot {
            session_generation: 1,
            user_id: "@alice:example.org".into(),
            device_id: "DEVICE".into(),
            homeserver_url: "https://example.org".into(),
            display_name: None,
            avatar_url: None,
            lifecycle: SessionLifecycle::Ready,
            crypto_ready: true,
        }
    }

    #[test]
    fn matrix_session_snapshot_response_uses_exact_desktop_wire_keys() {
        assert_eq!(
            serde_json::to_value(MatrixSessionSnapshotResponse::from(None)).unwrap(),
            serde_json::json!({"status":"logged_out"})
        );

        let response = MatrixSessionSnapshotResponse::from(Some(SessionSnapshot {
            session_generation: 7,
            user_id: "@alice:example.org".into(),
            device_id: "DEVICE".into(),
            homeserver_url: "https://example.org".into(),
            display_name: Some("Alice".into()),
            avatar_url: Some("mxc://example.org/avatar".into()),
            lifecycle: SessionLifecycle::Ready,
            crypto_ready: true,
        }));
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            serde_json::json!({
                "status":"logged_in",
                "user_id":"@alice:example.org",
                "device_id":"DEVICE",
                "homeserver_url":"https://example.org",
                "sessionGeneration":7,
            })
        );
    }

    #[tokio::test]
    async fn default_registry_dispatches_matrix_session_snapshot() {
        let core = Core::new(Arc::new(TestPlatform));
        assert_eq!(
            core.registered_commands(),
            vec![
                "matrix_login_flows",
                "matrix_register_flows",
                "matrix_session_snapshot",
                "matrix_sync_status",
            ]
        );

        let request = CommandEnvelope {
            command: "matrix_session_snapshot".into(),
            session_generation: 1,
            request_id: None,
            payload: serde_json::Value::Null,
        };
        assert_eq!(
            core.command(request.clone()).await.unwrap().payload,
            serde_json::json!({"status":"logged_out"})
        );

        core.open(session()).await.unwrap();
        assert_eq!(
            core.command(request).await.unwrap().payload,
            serde_json::json!({
                "status":"logged_in",
                "user_id":"@alice:example.org",
                "device_id":"DEVICE",
                "homeserver_url":"https://example.org",
                "sessionGeneration":1,
            })
        );
    }

    #[tokio::test]
    async fn core_sync_status_uses_exact_desktop_wire_shape() {
        let core = Core::new(Arc::new(TestPlatform));
        let request = CommandEnvelope {
            command: "matrix_sync_status".into(),
            session_generation: 0,
            request_id: Some("sync-status-fixture".into()),
            payload: serde_json::Value::Null,
        };

        let response = core
            .command(request)
            .await
            .expect("status observation succeeds");
        assert_eq!(response.command, "matrix_sync_status");
        assert_eq!(response.session_generation, 0);
        assert_eq!(response.request_id.as_deref(), Some("sync-status-fixture"));
        assert_eq!(
            response.payload,
            serde_json::json!({
                "readiness": "unconfigured",
                "sessionGeneration": 0,
                "offlineModeEnabled": false,
                "failureDiagnosticId": null,
                "slidingSyncCapable": null,
            })
        );
    }

    #[tokio::test]
    async fn core_sync_status_constructs_the_only_public_failure_diagnostic() {
        let status = PlatformSyncStatus::new(
            SyncReadiness::Failed,
            9,
            true,
            Some(PlatformSyncFailure::SyncService),
            Some(true),
        )
        .expect("closed sync failure is a valid Platform projection");
        let response = Core::new(Arc::new(StatusPlatform { status: Ok(status) }))
            .command(CommandEnvelope {
                command: "matrix_sync_status".into(),
                session_generation: 9,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect("closed Platform failure serializes through Core");

        assert_eq!(
            response.payload,
            serde_json::json!({
                "readiness": "failed",
                "sessionGeneration": 9,
                "offlineModeEnabled": true,
                "failureDiagnosticId": "p4.1-sync-service-error",
                "slidingSyncCapable": true,
            })
        );
    }

    #[tokio::test]
    async fn hostile_desktop_diagnostic_is_rejected_before_platform_core_or_public_transport() {
        let private_text: &'static str = Box::leak(
            "https://private.example token=secret password=secret"
                .to_owned()
                .into_boxed_str(),
        );
        let hostile_desktop_snapshot = SyncReadinessSnapshot {
            readiness: SyncReadiness::Failed,
            session_generation: 9,
            offline_mode_enabled: true,
            failure_diagnostic_id: Some(private_text),
            sliding_sync_capable: Some(false),
        };

        // This is the desktop-side normalization step. Its typed result has no
        // diagnostic-string field, so the hostile value cannot enter Platform.
        let normalized = PlatformSyncStatus::from_desktop_snapshot(hostile_desktop_snapshot);
        assert_eq!(
            normalized,
            Err(crate::platform::PlatformSyncStatusError::InvalidSnapshot)
        );
        assert!(!format!("{normalized:?}").contains(private_text));

        let error = Core::new(Arc::new(StatusPlatform { status: normalized }))
            .command(CommandEnvelope {
                command: "matrix_sync_status".into(),
                session_generation: 9,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect_err("rejected desktop diagnostic has no public status payload");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-sync-status-platform-unavailable")
        );
        let public_error = serde_json::to_string(&error).expect("static Core error serializes");
        for forbidden in ["private.example", "token", "secret", "password"] {
            assert!(
                !public_error.contains(forbidden),
                "hostile desktop diagnostic must not cross Platform/Core or public transport: {forbidden}"
            );
        }
    }

    #[tokio::test]
    async fn core_sync_status_fails_closed_with_static_errors() {
        let malformed = Core::new(Arc::new(TestPlatform))
            .command(CommandEnvelope {
                command: "matrix_sync_status".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"private": "token=secret"}),
            })
            .await
            .expect_err("status command must accept no payload");
        assert_eq!(malformed.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            malformed.diagnostic_id.as_deref(),
            Some("p2-sync-status-invalid-payload")
        );

        let error = Core::new(Arc::new(StatusPlatform {
            status: Err(crate::platform::PlatformSyncStatusError::Unavailable),
        }))
        .command(CommandEnvelope {
            command: "matrix_sync_status".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("opaque platform errors must not cross the Core transport");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-sync-status-platform-unavailable")
        );
    }

    fn assert_test_user_agent(request: &str) {
        let user_agent = request
            .split("\r\n")
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("user-agent")
                    .then_some(value.trim())
            })
            .expect("core auth probe must send a user-agent");
        assert_eq!(user_agent, TEST_HTTP_USER_AGENT);
    }

    async fn serve_login_flows_once(listener: &tokio::net::TcpListener) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (mut socket, _) = listener.accept().await.expect("accept login-flow request");
        let mut request = [0_u8; 2048];
        let read = socket
            .read(&mut request)
            .await
            .expect("read login-flow request");
        let request = std::str::from_utf8(&request[..read]).expect("HTTP request is text");
        assert!(
            request.starts_with("GET /_matrix/client/v3/login "),
            "handler must request only the login-types endpoint"
        );
        assert_test_user_agent(request);
        let body = r#"{"flows":[{"type":"m.login.password"},{"type":"m.login.token","get_login_token":true},{"type":"m.login.application_service"},{"type":"m.login.custom"}]}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket
            .write_all(response.as_bytes())
            .await
            .expect("write login-flow response");
    }

    #[tokio::test]
    async fn core_login_flows_uses_exact_react_payload_and_response_json() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind login-flow server");
        let address = listener.local_addr().expect("login-flow address");
        let server = tokio::spawn(async move { serve_login_flows_once(&listener).await });
        let core = Core::new(Arc::new(TestPlatform));

        let response = core
            .command(CommandEnvelope {
                command: "matrix_login_flows".into(),
                session_generation: 1,
                request_id: Some("login-flows-fixture".into()),
                payload: serde_json::json!({
                    "homeserverUrl": format!("http://{address}"),
                }),
            })
            .await
            .expect("login-flow handler succeeds");

        assert_eq!(
            response.payload,
            serde_json::json!({
                "flows": [
                    {"kind":"password","matrixType":"m.login.password"},
                    {"kind":"token","matrixType":"m.login.token","getLoginToken":true},
                    {"kind":"application_service","matrixType":"m.login.application_service"},
                    {"kind":"unknown","matrixType":"m.login.custom"},
                ]
            })
        );
        server.await.expect("login-flow server task");
    }

    #[tokio::test]
    async fn core_login_flows_rejects_malformed_missing_and_unsafe_input_privately() {
        let core = Core::new(Arc::new(TestPlatform));
        for payload in [
            serde_json::Value::Null,
            serde_json::json!({"homeserver_url":"https://not-the-react-key.invalid"}),
        ] {
            let error = core
                .command(CommandEnvelope {
                    command: "matrix_login_flows".into(),
                    session_generation: 1,
                    request_id: None,
                    payload,
                })
                .await
                .expect_err("malformed or missing payload must fail closed");
            assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
            assert_eq!(
                error.diagnostic_id.as_deref(),
                Some("p2-login-flows-invalid-payload")
            );
        }

        let unsafe_url = "https://private.example.invalid/../must-not-appear";
        let error = core
            .command(CommandEnvelope {
                command: "matrix_login_flows".into(),
                session_generation: 1,
                request_id: None,
                payload: serde_json::json!({"homeserverUrl": unsafe_url}),
            })
            .await
            .expect_err("unsafe homeserver must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p3.1-invalid-homeserver-url")
        );
        assert!(!format!("{error:?}").contains(unsafe_url));
    }

    async fn serve_register_flows_once(
        listener: &tokio::net::TcpListener,
        status: u16,
        body: &'static str,
    ) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (mut socket, _) = listener
            .accept()
            .await
            .expect("accept registration-flow request");
        let mut request = [0_u8; 4096];
        let read = socket
            .read(&mut request)
            .await
            .expect("read registration-flow request");
        let request = std::str::from_utf8(&request[..read]).expect("HTTP request is text");
        assert_test_user_agent(request);
        let (headers, request_body) = request
            .split_once("\r\n\r\n")
            .expect("registration request has headers and body");
        assert!(
            headers.starts_with("POST /_matrix/client/v3/register "),
            "handler must request only the empty registration-probe endpoint"
        );
        let headers_lower = headers.to_ascii_lowercase();
        assert!(headers_lower.contains("content-type: application/json"));
        assert_eq!(request_body, "{}", "probe must use only an empty JSON body");
        for forbidden in [
            "authorization:",
            "access_token",
            "refresh_token",
            "password",
            "registration_token",
            "client_secret",
            "captcha",
            "threepid",
            "session",
        ] {
            assert!(
                !request.to_ascii_lowercase().contains(forbidden),
                "registration probe request must not contain {forbidden}"
            );
        }

        let reason = match status {
            200 => "OK",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            429 => "Too Many Requests",
            _ => "Error",
        };
        let response = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket
            .write_all(response.as_bytes())
            .await
            .expect("write registration-flow response");
    }

    async fn core_register_flows_request(
        address: std::net::SocketAddr,
    ) -> Result<CommandResponseEnvelope, MatrixIpcError> {
        Core::new(Arc::new(TestPlatform))
            .command(CommandEnvelope {
                command: "matrix_register_flows".into(),
                session_generation: 1,
                request_id: Some("register-flows-fixture".into()),
                payload: serde_json::json!({
                    "homeserverUrl": format!("http://{address}"),
                }),
            })
            .await
    }

    #[tokio::test]
    async fn core_register_flows_uses_exact_react_wire_fixtures_and_empty_post() {
        const FLOW_REQUIRED_UIAA: &str = r#"{
            "flows":[
                {"stages":["m.login.terms","m.login.dummy"]},
                {"stages":["m.login.registration_token"]}
            ],
            "completed":["m.login.terms"],
            "params":{"m.login.terms":{"policies":[]}},
            "session":"opaque-uia-session"
        }"#;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind registration-flow server");
        let address = listener.local_addr().expect("registration-flow address");
        let server = tokio::spawn(async move {
            serve_register_flows_once(&listener, 401, FLOW_REQUIRED_UIAA).await;
        });

        let response = core_register_flows_request(address)
            .await
            .expect("registration UIAA probe succeeds");
        assert_eq!(
            response.payload,
            serde_json::json!({
                "status":"flow_required",
                "session":"opaque-uia-session",
                "flows":[
                    {"stages":["m.login.terms","m.login.dummy"]},
                    {"stages":["m.login.registration_token"]}
                ],
                "completed":["m.login.terms"],
                "params":{"m.login.terms":{"policies":[]}},
            })
        );
        assert_eq!(response.command, "matrix_register_flows");
        assert_eq!(
            response.request_id.as_deref(),
            Some("register-flows-fixture")
        );
        server.await.expect("registration-flow server task");
    }

    #[tokio::test]
    async fn core_register_flows_preserves_all_non_uia_probe_wire_variants() {
        for (status, expected) in [
            (200, serde_json::json!({"status":"invalid_request"})),
            (400, serde_json::json!({"status":"invalid_request"})),
            (403, serde_json::json!({"status":"registration_disabled"})),
            (429, serde_json::json!({"status":"rate_limited"})),
        ] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind registration-flow server");
            let address = listener.local_addr().expect("registration-flow address");
            let server = tokio::spawn(async move {
                serve_register_flows_once(&listener, status, "{").await;
            });

            let response = core_register_flows_request(address)
                .await
                .expect("known registration-probe status has a safe wire outcome");
            assert_eq!(response.payload, expected, "status {status}");
            server.await.expect("registration-flow server task");
        }
    }

    #[tokio::test]
    async fn core_register_flows_rejects_non_react_or_sensitive_payloads_privately() {
        let core = Core::new(Arc::new(TestPlatform));
        for payload in [
            serde_json::Value::Null,
            serde_json::json!({"homeserver_url":"https://not-the-react-key.invalid"}),
            serde_json::json!({
                "homeserverUrl":"https://not-the-react-key.invalid",
                "password":"must-not-cross-core",
            }),
            serde_json::json!({
                "homeserverUrl":"https://not-the-react-key.invalid",
                "session":"must-not-continue-uia",
            }),
        ] {
            let error = core
                .command(CommandEnvelope {
                    command: "matrix_register_flows".into(),
                    session_generation: 1,
                    request_id: None,
                    payload,
                })
                .await
                .expect_err("malformed or sensitive probe payload must fail closed");
            assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
            assert_eq!(
                error.diagnostic_id.as_deref(),
                Some("p2-register-flows-invalid-payload")
            );
            assert!(!format!("{error:?}").contains("must-not"));
        }

        let unsafe_url = "https://private.example.invalid/../must-not-appear";
        let error = core
            .command(CommandEnvelope {
                command: "matrix_register_flows".into(),
                session_generation: 1,
                request_id: None,
                payload: serde_json::json!({"homeserverUrl": unsafe_url}),
            })
            .await
            .expect_err("unsafe homeserver must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p3.1-invalid-homeserver-url")
        );
        assert!(!format!("{error:?}").contains(unsafe_url));
    }

    #[tokio::test]
    async fn core_register_flows_malformed_uiaa_fails_closed_without_raw_body() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind registration-flow server");
        let address = listener.local_addr().expect("registration-flow address");
        let raw_body = r#"{"flows":"not-an-array","error":"private remote body"}"#;
        let server = tokio::spawn(async move {
            serve_register_flows_once(&listener, 401, raw_body).await;
        });

        let error = core_register_flows_request(address)
            .await
            .expect_err("malformed UIAA response must fail closed");
        assert_eq!(
            error.category,
            MatrixIpcErrorCategory::UnsupportedCapability
        );
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-register-flows-uiaa-response-invalid")
        );
        assert!(!format!("{error:?}").contains("private remote body"));
        server.await.expect("registration-flow server task");
    }

    #[tokio::test]
    async fn core_open_and_close_only_manage_safe_session_projection() {
        let core = Core::new(Arc::new(TestPlatform));
        assert!(core.session_snapshot().unwrap().is_none());
        core.open(session()).await.unwrap();
        assert_eq!(core.session_snapshot().unwrap(), Some(session()));
        core.close().await.unwrap();
        assert!(core.session_snapshot().unwrap().is_none());
    }

    #[tokio::test]
    async fn command_registry_dispatches_one_typed_envelope() {
        let mut registry = CommandRegistry::new();
        registry
            .register(
                "matrix_login_flows",
                |_state: Arc<CoreState>, request: CommandEnvelope| -> CommandFuture {
                    Box::pin(async move { Ok(request.payload) })
                },
            )
            .unwrap();
        let core = Core::with_registry(Arc::new(TestPlatform), registry);
        let response = core
            .command(CommandEnvelope {
                command: "matrix_login_flows".into(),
                session_generation: 1,
                request_id: Some("r1".into()),
                payload: serde_json::json!({"safe":true}),
            })
            .await
            .unwrap();
        assert_eq!(response.payload, serde_json::json!({"safe":true}));
        assert_eq!(core.registered_commands(), vec!["matrix_login_flows"]);
    }

    #[tokio::test]
    async fn known_but_unregistered_commands_fail_closed_with_static_diagnostic() {
        let core = Core::new(Arc::new(TestPlatform));
        for command in [
            "matrix_login_password",
            "matrix_register",
            "matrix_register_request_email_token",
        ] {
            let error = core
                .command(CommandEnvelope {
                    command: command.into(),
                    session_generation: 1,
                    request_id: None,
                    payload: serde_json::Value::Null,
                })
                .await
                .expect_err("known but unregistered command must fail closed");
            assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
            assert_eq!(
                error.diagnostic_id.as_deref(),
                Some("p2-command-unregistered")
            );
        }
    }
}
