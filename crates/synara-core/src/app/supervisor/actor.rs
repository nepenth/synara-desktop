//! Matrix supervisor actor — sole owner of client handle + lifecycle state.

use crate::dto::SessionLifecycle;
use crate::transport::MatrixIpcErrorCategory;

use super::error::{SupervisorError, TransitionError};
use super::handle::{ClientFactory, ClientHandle, NullClientFactory};
use super::state::SupervisorState;
use super::transition::{self, SupervisorCommand, TransitionEffect};

/// Privacy-safe last failure recorded on the supervisor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureInfo {
    pub category: MatrixIpcErrorCategory,
    pub diagnostic_id: &'static str,
}

/// Read-only snapshot for diagnostics / tests (no secrets, no SDK types).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisorSnapshot {
    pub state: SupervisorState,
    pub lifecycle: SessionLifecycle,
    pub session_generation: u64,
    pub has_client: bool,
    pub live_handles: usize,
    pub last_failure: Option<FailureInfo>,
}

/// Single-owner Matrix supervisor.
///
/// **Invariant:** at most one `ClientHandle` is installed. Construction happens
/// only through [`Self::apply`] + [`SupervisorCommand::InstallClient`] + the
/// injected [`ClientFactory`]. There is no dual-backend path.
pub struct MatrixSupervisor {
    state: SupervisorState,
    session_generation: u64,
    client: Option<Box<dyn ClientHandle>>,
    last_failure: Option<FailureInfo>,
    /// Monotonic count of handles installed for leak assertions across cycles.
    installed_total: u64,
    /// Handles that have been shut down (should equal installed when idle).
    shutdown_total: u64,
}

impl Default for MatrixSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl MatrixSupervisor {
    /// Fresh supervisor: empty, generation 0, no client.
    pub fn new() -> Self {
        Self {
            state: SupervisorState::Empty,
            session_generation: 0,
            client: None,
            last_failure: None,
            installed_total: 0,
            shutdown_total: 0,
        }
    }

    pub fn state(&self) -> SupervisorState {
        self.state
    }

    /// Product DTO projection of the actor state.
    pub fn lifecycle(&self) -> SessionLifecycle {
        self.state.into()
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn has_client(&self) -> bool {
        self.client.is_some()
    }

    pub fn live_handles(&self) -> usize {
        usize::from(self.client.is_some())
    }

    pub fn installed_total(&self) -> u64 {
        self.installed_total
    }

    pub fn shutdown_total(&self) -> u64 {
        self.shutdown_total
    }

    pub fn last_failure(&self) -> Option<&FailureInfo> {
        self.last_failure.as_ref()
    }

    /// True when `observed` equals the live session generation.
    pub fn is_live_generation(&self, observed: u64) -> bool {
        self.session_generation == observed
    }

    /// Publish allowed only in Syncing/Ready with installed client + matching gen.
    pub fn may_publish(&self, observed_generation: u64) -> bool {
        self.state.allows_publish()
            && self.has_client()
            && self.is_live_generation(observed_generation)
    }

    pub fn snapshot(&self) -> SupervisorSnapshot {
        SupervisorSnapshot {
            state: self.state,
            lifecycle: self.lifecycle(),
            session_generation: self.session_generation,
            has_client: self.has_client(),
            live_handles: self.live_handles(),
            last_failure: self.last_failure.clone(),
        }
    }

    /// Apply a pure lifecycle command without installing a client.
    ///
    /// Commands that require [`SupervisorCommand::InstallClient`] side effects
    /// must use [`Self::apply_with_factory`].
    pub fn apply(&mut self, command: SupervisorCommand) -> Result<(), SupervisorError> {
        self.apply_inner(command, None::<&NullClientFactory>)
    }

    /// Apply a command, supplying the sole client factory when installing.
    pub fn apply_with_factory<F: ClientFactory + ?Sized>(
        &mut self,
        command: SupervisorCommand,
        factory: &F,
    ) -> Result<(), SupervisorError> {
        self.apply_inner(command, Some(factory))
    }

    /// Record a failure with category + diagnostic id, transitioning via Fail.
    pub fn fail(
        &mut self,
        category: MatrixIpcErrorCategory,
        diagnostic_id: &'static str,
    ) -> Result<(), SupervisorError> {
        self.apply(SupervisorCommand::Fail)?;
        self.last_failure = Some(FailureInfo {
            category,
            diagnostic_id,
        });
        Ok(())
    }

    /// Pure preflight: would this command be legal from the current state?
    pub fn can_apply(&self, command: SupervisorCommand) -> Result<(), TransitionError> {
        transition::resolve(self.state, command).map(|_| ())
    }

    fn apply_inner<F: ClientFactory + ?Sized>(
        &mut self,
        command: SupervisorCommand,
        factory: Option<&F>,
    ) -> Result<(), SupervisorError> {
        let (next, effect) = transition::resolve(self.state, command)?;
        self.apply_effect(effect, factory)?;
        self.state = next;
        Ok(())
    }

    fn apply_effect<F: ClientFactory + ?Sized>(
        &mut self,
        effect: TransitionEffect,
        factory: Option<&F>,
    ) -> Result<(), SupervisorError> {
        if effect.require_client && self.client.is_none() {
            return Err(SupervisorError::ClientMissing);
        }

        if effect.install_client {
            if self.client.is_some() {
                return Err(SupervisorError::ClientAlreadyPresent);
            }
            let factory = factory.ok_or(SupervisorError::ConstructionFailed {
                category: MatrixIpcErrorCategory::SdkInvariant,
                diagnostic_id: "p2.1-install-without-factory",
            })?;
            match factory.build(self.session_generation) {
                Ok(handle) => {
                    self.client = Some(handle);
                    self.installed_total = self.installed_total.saturating_add(1);
                }
                Err(err) => {
                    return Err(SupervisorError::ConstructionFailed {
                        category: err.category,
                        diagnostic_id: err.diagnostic_id,
                    });
                }
            }
        }

        if effect.drop_client {
            self.drop_client_internal();
        }

        if effect.bump_generation {
            self.session_generation = self.session_generation.saturating_add(1);
        }

        if effect.clear_failure {
            self.last_failure = None;
        }

        Ok(())
    }

    fn drop_client_internal(&mut self) {
        if let Some(mut handle) = self.client.take() {
            handle.shutdown();
            self.shutdown_total = self.shutdown_total.saturating_add(1);
        }
    }
}

/// Convenience: open + authenticate + install + sync + ready happy path for tests.
pub fn harness_login_ready<F: ClientFactory + ?Sized>(
    supervisor: &mut MatrixSupervisor,
    factory: &F,
) -> Result<(), SupervisorError> {
    supervisor.apply(SupervisorCommand::BeginOpen)?;
    supervisor.apply(SupervisorCommand::BeginAuthenticate)?;
    supervisor.apply_with_factory(SupervisorCommand::InstallClient, factory)?;
    supervisor.apply(SupervisorCommand::BeginSync)?;
    supervisor.apply(SupervisorCommand::MarkReady)?;
    Ok(())
}

/// Convenience: open + restore path to ready.
pub fn harness_restore_ready<F: ClientFactory + ?Sized>(
    supervisor: &mut MatrixSupervisor,
    factory: &F,
) -> Result<(), SupervisorError> {
    supervisor.apply(SupervisorCommand::BeginOpen)?;
    supervisor.apply(SupervisorCommand::BeginRestore)?;
    supervisor.apply_with_factory(SupervisorCommand::InstallClient, factory)?;
    supervisor.apply(SupervisorCommand::BeginSync)?;
    supervisor.apply(SupervisorCommand::MarkReady)?;
    Ok(())
}
