//! Remote / server logout + coordinated session end (P3.8 harness foundation).
//!
//! Pure state machine for remote logout request sequencing and local cleanup
//! policy. Host adapters will call SDK logout APIs later. **No tokens, no
//! dual-backend, no production Tauri commands.**

pub use super::remote_policy::{LocalCleanupPolicy, RemoteLogoutScope};
use super::LifecycleError;

/// Phase of a remote logout attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RemoteLogoutPhase {
    Idle,
    /// Host is calling server logout (network in flight).
    RequestingRemote,
    /// Remote succeeded or was skipped; local logout/wipe still pending.
    LocalCleanupPending,
    /// Fully complete (remote + local policy applied).
    Complete,
    /// Failed; may retry after clear_failure.
    Failed,
}

impl RemoteLogoutPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::RequestingRemote => "requesting_remote",
            Self::LocalCleanupPending => "local_cleanup_pending",
            Self::Complete => "complete",
            Self::Failed => "failed",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Complete | Self::Failed | Self::Idle)
    }

    pub fn is_busy(self) -> bool {
        matches!(self, Self::RequestingRemote | Self::LocalCleanupPending)
    }
}

/// Privacy-safe outcome of a completed remote-logout flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteLogoutOutcome {
    pub scope: RemoteLogoutScope,
    pub remote_succeeded: bool,
    pub remote_skipped: bool,
    pub local_policy: LocalCleanupPolicy,
    pub local_cleanup_applied: bool,
    pub session_generation: u64,
}

/// Session-generation-stamped remote logout coordinator.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoteLogoutFlow {
    session_generation: u64,
    phase: RemoteLogoutPhase,
    scope: Option<RemoteLogoutScope>,
    local_policy: LocalCleanupPolicy,
    /// When true, host may complete without a successful remote call
    /// (offline / already invalid access token).
    allow_skip_remote: bool,
    remote_succeeded: bool,
    remote_skipped: bool,
    local_cleanup_applied: bool,
    failure_diagnostic_id: Option<&'static str>,
    attempts: u32,
}

impl RemoteLogoutFlow {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            phase: RemoteLogoutPhase::Idle,
            scope: None,
            local_policy: LocalCleanupPolicy::LogoutRetainStores,
            allow_skip_remote: true,
            remote_succeeded: false,
            remote_skipped: false,
            local_cleanup_applied: false,
            failure_diagnostic_id: None,
            attempts: 0,
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn phase(&self) -> RemoteLogoutPhase {
        self.phase
    }

    pub fn scope(&self) -> Option<RemoteLogoutScope> {
        self.scope
    }

    pub fn local_policy(&self) -> LocalCleanupPolicy {
        self.local_policy
    }

    pub fn failure_diagnostic_id(&self) -> Option<&'static str> {
        self.failure_diagnostic_id
    }

    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    pub fn is_busy(&self) -> bool {
        self.phase.is_busy()
    }

    pub fn set_allow_skip_remote(&mut self, allow: bool) {
        self.allow_skip_remote = allow;
    }

    pub fn set_local_policy(&mut self, policy: LocalCleanupPolicy) -> Result<(), LifecycleError> {
        if self.phase.is_busy() {
            return Err(LifecycleError::InvalidTarget {
                diagnostic_id: "p3.8-busy-cannot-set-policy",
            });
        }
        self.local_policy = policy;
        Ok(())
    }

    /// Begin remote logout. Rejects when already in flight.
    pub fn begin(&mut self, scope: RemoteLogoutScope) -> Result<(), LifecycleError> {
        if self.phase.is_busy() {
            return Err(LifecycleError::InvalidTarget {
                diagnostic_id: "p3.8-remote-logout-busy",
            });
        }
        if self.phase == RemoteLogoutPhase::Complete {
            return Err(LifecycleError::InvalidTarget {
                diagnostic_id: "p3.8-already-complete",
            });
        }
        self.scope = Some(scope);
        self.remote_succeeded = false;
        self.remote_skipped = false;
        self.local_cleanup_applied = false;
        self.failure_diagnostic_id = None;
        self.attempts = self.attempts.saturating_add(1);
        self.phase = RemoteLogoutPhase::RequestingRemote;
        Ok(())
    }

    /// Host reports remote API succeeded.
    pub fn complete_remote(&mut self) -> Result<(), LifecycleError> {
        if self.phase != RemoteLogoutPhase::RequestingRemote {
            return Err(LifecycleError::InvalidTarget {
                diagnostic_id: "p3.8-complete-remote-not-requesting",
            });
        }
        self.remote_succeeded = true;
        self.remote_skipped = false;
        self.phase = RemoteLogoutPhase::LocalCleanupPending;
        Ok(())
    }

    /// Host skips remote (offline / no valid session) when policy allows.
    pub fn skip_remote(&mut self, diagnostic_id: &'static str) -> Result<(), LifecycleError> {
        if self.phase != RemoteLogoutPhase::RequestingRemote {
            return Err(LifecycleError::InvalidTarget {
                diagnostic_id: "p3.8-skip-remote-not-requesting",
            });
        }
        if !self.allow_skip_remote {
            return Err(LifecycleError::InvalidTarget {
                diagnostic_id: "p3.8-skip-remote-disallowed",
            });
        }
        validate_diagnostic(diagnostic_id)?;
        self.remote_succeeded = false;
        self.remote_skipped = true;
        self.failure_diagnostic_id = Some(diagnostic_id);
        self.phase = RemoteLogoutPhase::LocalCleanupPending;
        Ok(())
    }

    /// Fail remote request; stays Failed until clear_failure (no local cleanup yet).
    pub fn fail_remote(&mut self, diagnostic_id: &'static str) -> Result<(), LifecycleError> {
        if self.phase != RemoteLogoutPhase::RequestingRemote {
            return Err(LifecycleError::InvalidTarget {
                diagnostic_id: "p3.8-fail-remote-not-requesting",
            });
        }
        validate_diagnostic(diagnostic_id)?;
        self.phase = RemoteLogoutPhase::Failed;
        self.failure_diagnostic_id = Some(diagnostic_id);
        Ok(())
    }

    /// Host applied local logout/wipe (calls into P2.6 paths).
    pub fn complete_local_cleanup(&mut self) -> Result<RemoteLogoutOutcome, LifecycleError> {
        if self.phase != RemoteLogoutPhase::LocalCleanupPending {
            return Err(LifecycleError::InvalidTarget {
                diagnostic_id: "p3.8-local-cleanup-not-pending",
            });
        }
        self.local_cleanup_applied = true;
        self.phase = RemoteLogoutPhase::Complete;
        Ok(RemoteLogoutOutcome {
            scope: self.scope.unwrap_or(RemoteLogoutScope::ThisDevice),
            remote_succeeded: self.remote_succeeded,
            remote_skipped: self.remote_skipped,
            local_policy: self.local_policy,
            local_cleanup_applied: true,
            session_generation: self.session_generation,
        })
    }

    /// Clear Failed so begin() can retry.
    pub fn clear_failure(&mut self) -> Result<(), LifecycleError> {
        if self.phase != RemoteLogoutPhase::Failed {
            return Err(LifecycleError::InvalidTarget {
                diagnostic_id: "p3.8-not-failed",
            });
        }
        self.phase = RemoteLogoutPhase::Idle;
        self.failure_diagnostic_id = None;
        Ok(())
    }

    /// Retire to a new session generation; cancel in-flight work.
    pub fn retire_generation(&mut self, new_generation: u64) {
        let was_busy = self.phase.is_busy();
        self.session_generation = new_generation;
        self.scope = None;
        self.remote_succeeded = false;
        self.remote_skipped = false;
        self.local_cleanup_applied = false;
        self.attempts = 0;
        self.local_policy = LocalCleanupPolicy::LogoutRetainStores;
        if was_busy {
            self.phase = RemoteLogoutPhase::Failed;
            self.failure_diagnostic_id = Some("p3.8-stale-generation-cancelled");
        } else {
            self.phase = RemoteLogoutPhase::Idle;
            self.failure_diagnostic_id = None;
        }
    }
}

fn validate_diagnostic(diagnostic_id: &'static str) -> Result<(), LifecycleError> {
    if diagnostic_id.is_empty() {
        return Err(LifecycleError::InvalidTarget {
            diagnostic_id: "p3.8-empty-failure-id",
        });
    }
    let lower = diagnostic_id.to_ascii_lowercase();
    if lower.contains("access_token")
        || lower.contains("refresh_token")
        || lower.contains("password")
        || lower.contains("secret")
        || lower.contains("private_key")
    {
        return Err(LifecycleError::InvalidTarget {
            diagnostic_id: "p3.8-forbidden-diagnostic",
        });
    }
    Ok(())
}
