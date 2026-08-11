//! Shared native-core entry points (P2 foundation).
//!
//! `Core` owns safe session projection/lifecycle plus the transport command
//! registry. It intentionally has no Tauri dependency; P2 command groups add
//! handlers, P3 makes the desktop shell a thin `Core::command` registrar.

use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::dto::SessionSnapshot;
use crate::platform::Platform;
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
}

fn matrix_session_snapshot(state: Arc<CoreState>, _request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let response = MatrixSessionSnapshotResponse::from(state.session_snapshot()?);
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-session-snapshot-serialization-failed"))
    })
}

fn core_state_error(diagnostic_id: &'static str) -> MatrixIpcError {
    MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant).with_diagnostic(diagnostic_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{SessionLifecycle, SessionSnapshot};
    use crate::platform::{PlatformStatus, SecretVault, UnavailableSecretVault};
    use crate::transport::{CommandFuture, CommandRegistry};

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
        assert_eq!(core.registered_commands(), vec!["matrix_session_snapshot"]);

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
    async fn known_but_unregistered_command_fails_closed_with_static_diagnostic() {
        let core = Core::new(Arc::new(TestPlatform));
        let error = core
            .command(CommandEnvelope {
                command: "matrix_login_password".into(),
                session_generation: 1,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .unwrap_err();
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-command-unregistered")
        );
    }
}
