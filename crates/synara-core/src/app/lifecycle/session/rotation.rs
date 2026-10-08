//! Keep the host vault in lockstep with SDK access/refresh-token rotation.
//!
//! `handle_refresh_tokens` updates the live SDK session, but persistence is an
//! application responsibility. Without these callbacks, a later relaunch can
//! restore a consumed refresh token even though the preceding run worked.
//! Every save goes through the session's [`SessionPersistenceLease`], so a
//! refresh that lands after logout or retirement cannot write credentials back.

use std::sync::Arc;

use matrix_sdk::Client;

use super::{SessionFault, SessionLocator, SessionPersistenceLease};
use crate::app::lifecycle::{
    load_session_material, matrix_session_from_host_secrets, persist_session_after_login,
    LifecycleError, SessionMaterialVault,
};
use crate::app::store::AccountIdentity;

/// Platform-specific static ids returned to the SDK when a callback fails.
/// Desktop and iOS keep the ids they shipped with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RotationDiagnostics {
    pub reload_read_failed: &'static str,
    pub reload_material_missing: &'static str,
    pub reload_decode_failed: &'static str,
    pub reload_invalid: &'static str,
    pub persist_failed: &'static str,
    pub install_failed: &'static str,
}

impl RotationDiagnostics {
    pub const DESKTOP: Self = Self {
        reload_read_failed: "d0.1-session-reload-read-failed",
        reload_material_missing: "d0.1-session-reload-material-missing",
        reload_decode_failed: "d0.1-session-reload-decode-failed",
        reload_invalid: "d0.1-session-reload-invalid",
        persist_failed: "d0.1-session-rotation-persist-failed",
        install_failed: "d0.1-session-callback-install-failed",
    };

    pub const IOS: Self = Self {
        reload_read_failed: "session-reload-read-failed",
        reload_material_missing: "session-reload-material-missing",
        reload_decode_failed: "session-reload-decode-failed",
        reload_invalid: "session-reload-invalid",
        persist_failed: "session-rotation-persist-failed",
        install_failed: "session-callback-install-failed",
    };
}

/// Static error handed back to the SDK from a session callback.
#[derive(Debug)]
pub struct SessionRotationCallbackError(pub &'static str);

impl std::fmt::Display for SessionRotationCallbackError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for SessionRotationCallbackError {}

type IdentityCheck =
    Box<dyn Fn(&Client) -> Result<Option<SessionLocator>, &'static str> + Send + Sync>;
type LocatorPreflight<E> = Box<dyn Fn(&SessionLocator) -> Result<(), E> + Send + Sync>;
type OutcomeRecorder<E> = Box<dyn Fn(&Result<(), E>) + Send + Sync>;

/// Shell hooks around one rotation save.
pub struct RotationHooks<E> {
    /// Before the lease: confirm the rotated client still belongs to this
    /// account and return the locator the save must keep durable. `Ok(None)`
    /// skips the locator preflight (iOS keeps its locator in Swift).
    pub check_identity: IdentityCheck,
    /// Inside the lease, before the write: make the cleanup target durable.
    pub preflight: LocatorPreflight<E>,
    /// Map a vault failure to the shell's error.
    pub map_persist_error: fn(LifecycleError) -> E,
    /// Record the save outcome (desktop appends the lifecycle journal).
    pub record_outcome: OutcomeRecorder<E>,
}

impl<E> RotationHooks<E> {
    /// No identity check, preflight or journal. Persist errors map through
    /// `map_persist_error`.
    pub fn plain(map_persist_error: fn(LifecycleError) -> E) -> Self {
        Self {
            check_identity: Box::new(|_| Ok(None)),
            preflight: Box::new(|_| unreachable!("no locator without an identity check")),
            map_persist_error,
            record_outcome: Box::new(|_| {}),
        }
    }
}

/// Install the reload and save callbacks for one client.
pub fn install_session_rotation_callbacks<V, E>(
    client: &Client,
    identity: AccountIdentity,
    vault: Arc<V>,
    lease: Arc<SessionPersistenceLease>,
    diagnostics: RotationDiagnostics,
    hooks: RotationHooks<E>,
) -> Result<(), SessionFault>
where
    V: SessionMaterialVault + Sync + 'static,
    E: From<SessionFault> + 'static,
{
    let reload_identity = identity.clone();
    let reload_vault = Arc::clone(&vault);
    let save_identity = identity;
    client
        .set_session_callbacks(
            Box::new(move |_| {
                let material = load_session_material(reload_vault.as_ref(), &reload_identity)
                    .map_err(|_| SessionRotationCallbackError(diagnostics.reload_read_failed))?
                    .ok_or(SessionRotationCallbackError(
                        diagnostics.reload_material_missing,
                    ))?;
                let secrets = material
                    .decode_host_secrets()
                    .map_err(|_| SessionRotationCallbackError(diagnostics.reload_decode_failed))?;
                let session = matrix_session_from_host_secrets(&reload_identity, &secrets)
                    .map_err(|_| SessionRotationCallbackError(diagnostics.reload_invalid))?;
                Ok(session.tokens)
            }),
            Box::new(move |client| {
                let locator =
                    (hooks.check_identity)(&client).map_err(SessionRotationCallbackError)?;
                // SDK rotation can run before the explicit login save. Every
                // callback establishes the same durable cleanup target first.
                let persisted = lease.save_credentials(
                    || match &locator {
                        Some(locator) => (hooks.preflight)(locator),
                        None => Ok(()),
                    },
                    || {
                        persist_session_after_login(&client, &save_identity, vault.as_ref())
                            .map(|_| ())
                            .map_err(hooks.map_persist_error)
                    },
                );
                (hooks.record_outcome)(&persisted);
                persisted.map_err(|_| SessionRotationCallbackError(diagnostics.persist_failed))?;
                Ok(())
            }),
        )
        .map_err(|_| SessionFault::unavailable(diagnostics.install_failed))
}
