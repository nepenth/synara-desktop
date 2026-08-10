//! Reconnect / restart decision table for SyncService (P4.1).
//!
//! Pure policy: given observed readiness + operator intent, decide the next
//! SyncService action. Does not perform network I/O.

use super::readiness::SyncReadiness;

/// Operator or lifecycle intent that may require a reconnect decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyncIntent {
    /// Bootstrap after login/restore — start sync if not already running.
    Bootstrap,
    /// App / network returned; recover from offline/error/terminated.
    Recover,
    /// Explicit user or supervisor pause (logout path / background).
    Pause,
    /// Hard stop (logout / wipe / generation bump).
    Shutdown,
    /// Periodic tick / health observer with no new intent.
    Observe,
}

/// Next action the owner should take on the SyncService.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReconnectAction {
    /// No-op; leave service as-is.
    None,
    /// Call `SyncService::start` (safe if already Running per SDK).
    Start,
    /// Call `SyncService::stop`.
    Stop,
    /// Stop then start (offline recovery / hard restart).
    Restart,
}

impl ReconnectAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Restart => "restart",
        }
    }
}

/// Decide reconnect action from readiness + intent.
///
/// Policy notes (aligned with matrix-sdk-ui SyncService docs):
/// - `start` is idempotent when already Running.
/// - Offline: `start` exits offline probe and retries; we prefer Start over
///   full Restart unless explicit Recover after Failed.
/// - Failed / Terminated: Recover → Start (SDK cleans up then restarts).
/// - Shutdown always stops when not already idle/unconfigured.
pub fn decide_reconnect(readiness: SyncReadiness, intent: SyncIntent) -> ReconnectAction {
    use ReconnectAction as A;
    use SyncIntent as I;
    use SyncReadiness as R;

    match intent {
        I::Shutdown => match readiness {
            R::Unconfigured | R::Idle | R::Terminated | R::Failed => A::None,
            R::Running | R::Offline => A::Stop,
        },
        I::Pause => match readiness {
            R::Running | R::Offline => A::Stop,
            R::Unconfigured | R::Idle | R::Terminated | R::Failed => A::None,
        },
        I::Bootstrap => match readiness {
            R::Unconfigured => A::None, // must build first
            R::Idle | R::Terminated | R::Failed | R::Offline => A::Start,
            R::Running => A::None,
        },
        I::Recover => match readiness {
            R::Unconfigured => A::None,
            R::Running => A::None,
            R::Idle | R::Terminated | R::Offline => A::Start,
            // Hard error: explicit restart path (stop if needed + start).
            R::Failed => A::Restart,
        },
        I::Observe => A::None,
    }
}

/// Whether the decision table treats this readiness as restartable without rebuild.
pub fn is_restartable(readiness: SyncReadiness) -> bool {
    !matches!(readiness, SyncReadiness::Unconfigured)
}
