//! Supervisor transition and construction errors (privacy-safe).

use super::state::SupervisorState;
use super::transition::SupervisorCommand;
use crate::transport::MatrixIpcErrorCategory;

/// Illegal or rejected lifecycle transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionError {
    pub from: SupervisorState,
    pub command: SupervisorCommand,
    pub reason: &'static str,
}

impl TransitionError {
    pub fn new(from: SupervisorState, command: SupervisorCommand, reason: &'static str) -> Self {
        Self {
            from,
            command,
            reason,
        }
    }
}

impl std::fmt::Display for TransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "illegal supervisor transition: {} + {:?} ({})",
            self.from.as_str(),
            self.command,
            self.reason
        )
    }
}

impl std::error::Error for TransitionError {}

/// Supervisor-level failure (construction, wipe, etc.).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupervisorError {
    Transition(TransitionError),
    /// Client already installed; sole-owner invariant forbids a second handle.
    ClientAlreadyPresent,
    /// Command requires an installed client handle and none exists.
    ClientMissing,
    /// Factory refused to build (harness / future store/crypto errors).
    ConstructionFailed {
        category: MatrixIpcErrorCategory,
        diagnostic_id: &'static str,
    },
    /// Attempted to construct a client outside the supervisor (test hook).
    ConstructionBypassedSupervisor,
}

impl std::fmt::Display for SupervisorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transition(e) => write!(f, "{e}"),
            Self::ClientAlreadyPresent => {
                write!(f, "client handle already present (sole-owner invariant)")
            }
            Self::ClientMissing => write!(f, "client handle missing"),
            Self::ConstructionFailed {
                category,
                diagnostic_id,
            } => write!(
                f,
                "client construction failed ({category:?}, {diagnostic_id})"
            ),
            Self::ConstructionBypassedSupervisor => {
                write!(f, "client construction must go through MatrixSupervisor")
            }
        }
    }
}

impl std::error::Error for SupervisorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transition(e) => Some(e),
            _ => None,
        }
    }
}

impl From<TransitionError> for SupervisorError {
    fn from(value: TransitionError) -> Self {
        Self::Transition(value)
    }
}
