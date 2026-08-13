//! Supervisor lifecycle states and high-level product events.
//!
//! Wire-identical names to [`crate::dto::SessionLifecycle`] so product
//! snapshots can project actor state without inventing a second vocabulary.
//! Commands that drive the pure transition table live in
//! [`super::transition::SupervisorCommand`].

use crate::dto::SessionLifecycle;

/// High-level Matrix client lifecycle under the single-owner supervisor.
///
/// Values match plan P2.1 / `SessionLifecycle` wire names (`snake_case` on wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupervisorState {
    /// No session ownership; idle bootstrap surface.
    Empty,
    /// Opening store paths / preparing client scaffolding (no live Client yet).
    Opening,
    /// Interactive authentication in progress.
    Authenticating,
    /// Restoring a previously persisted session.
    Restoring,
    /// Initial or catch-up sync in progress.
    Syncing,
    /// Session live; product may consume projections for this generation.
    Ready,
    /// Graceful stop / logout teardown in progress.
    Stopping,
    /// Explicit logout completed; local stores may still exist for wipe.
    LoggedOut,
    /// Terminal failure for the current epoch; requires reset/open/wipe.
    Failed,
    /// Local store wipe in progress.
    Wiping,
}

impl SupervisorState {
    pub const ALL: &'static [SupervisorState] = &[
        Self::Empty,
        Self::Opening,
        Self::Authenticating,
        Self::Restoring,
        Self::Syncing,
        Self::Ready,
        Self::Stopping,
        Self::LoggedOut,
        Self::Failed,
        Self::Wiping,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Opening => "opening",
            Self::Authenticating => "authenticating",
            Self::Restoring => "restoring",
            Self::Syncing => "syncing",
            Self::Ready => "ready",
            Self::Stopping => "stopping",
            Self::LoggedOut => "logged_out",
            Self::Failed => "failed",
            Self::Wiping => "wiping",
        }
    }

    /// Whether product/stream publish is allowed for the live generation.
    pub fn allows_publish(self) -> bool {
        matches!(self, Self::Syncing | Self::Ready)
    }

    /// Whether a session-owning epoch is active (including in-flight setup).
    pub fn is_session_epoch_active(self) -> bool {
        matches!(
            self,
            Self::Opening
                | Self::Authenticating
                | Self::Restoring
                | Self::Syncing
                | Self::Ready
                | Self::Stopping
                | Self::Wiping
        )
    }
}

impl From<SupervisorState> for SessionLifecycle {
    fn from(value: SupervisorState) -> Self {
        match value {
            SupervisorState::Empty => Self::Empty,
            SupervisorState::Opening => Self::Opening,
            SupervisorState::Authenticating => Self::Authenticating,
            SupervisorState::Restoring => Self::Restoring,
            SupervisorState::Syncing => Self::Syncing,
            SupervisorState::Ready => Self::Ready,
            SupervisorState::Stopping => Self::Stopping,
            SupervisorState::LoggedOut => Self::LoggedOut,
            SupervisorState::Failed => Self::Failed,
            SupervisorState::Wiping => Self::Wiping,
        }
    }
}

impl From<SessionLifecycle> for SupervisorState {
    fn from(value: SessionLifecycle) -> Self {
        match value {
            SessionLifecycle::Empty => Self::Empty,
            SessionLifecycle::Opening => Self::Opening,
            SessionLifecycle::Authenticating => Self::Authenticating,
            SessionLifecycle::Restoring => Self::Restoring,
            SessionLifecycle::Syncing => Self::Syncing,
            SessionLifecycle::Ready => Self::Ready,
            SessionLifecycle::Stopping => Self::Stopping,
            SessionLifecycle::LoggedOut => Self::LoggedOut,
            SessionLifecycle::Failed => Self::Failed,
            SessionLifecycle::Wiping => Self::Wiping,
        }
    }
}

/// High-level product/lifecycle signals (documentation + tests).
///
/// The actor applies [`super::transition::SupervisorCommand`] values, which
/// include construction-path commands (`InstallClient`) not listed here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupervisorEvent {
    Open,
    BeginAuthenticate,
    BeginRestore,
    AuthenticateSucceeded,
    RestoreSucceeded,
    SyncReady,
    Stop,
    StopCompleted,
    Logout,
    LogoutCompleted,
    Wipe,
    WipeCompleted,
    Fail,
    Reset,
}

impl SupervisorEvent {
    pub const ALL: &'static [SupervisorEvent] = &[
        Self::Open,
        Self::BeginAuthenticate,
        Self::BeginRestore,
        Self::AuthenticateSucceeded,
        Self::RestoreSucceeded,
        Self::SyncReady,
        Self::Stop,
        Self::StopCompleted,
        Self::Logout,
        Self::LogoutCompleted,
        Self::Wipe,
        Self::WipeCompleted,
        Self::Fail,
        Self::Reset,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::BeginAuthenticate => "begin_authenticate",
            Self::BeginRestore => "begin_restore",
            Self::AuthenticateSucceeded => "authenticate_succeeded",
            Self::RestoreSucceeded => "restore_succeeded",
            Self::SyncReady => "sync_ready",
            Self::Stop => "stop",
            Self::StopCompleted => "stop_completed",
            Self::Logout => "logout",
            Self::LogoutCompleted => "logout_completed",
            Self::Wipe => "wipe",
            Self::WipeCompleted => "wipe_completed",
            Self::Fail => "fail",
            Self::Reset => "reset",
        }
    }
}
