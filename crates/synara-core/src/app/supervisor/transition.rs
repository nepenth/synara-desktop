//! Pure lifecycle transition table for the Matrix supervisor.
//!
//! Deterministic: `(SupervisorState, SupervisorCommand) → next state` or error.
//! Side effects (generation bump, client drop) are described by
//! [`TransitionEffect`] and applied by the actor.

use super::error::TransitionError;
use super::state::SupervisorState;

/// Commands the supervisor accepts. Product IPC will map to these later;
/// P2.1 exercises them only from unit/integration harnesses.
///
/// Overlaps intentionally with [`super::state::SupervisorEvent`] naming; the
/// command set adds `InstallClient` (sole construction path) which is not an
/// external product event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupervisorCommand {
    /// Begin opening stores / session bootstrap (Empty | LoggedOut | Failed).
    BeginOpen,
    /// Password login path after open.
    BeginAuthenticate,
    /// Restore persisted native session after open.
    BeginRestore,
    /// Install client handle via the sole factory (Opening | Authenticating | Restoring).
    InstallClient,
    /// Start sync after auth/restore (+ installed client).
    BeginSync,
    /// First product-ready signal after sync.
    MarkReady,
    /// Begin graceful stop (logout path).
    BeginStop,
    /// Logout complete; local data may remain (distinct from wipe — D-LOGOUT-WIPE).
    CompleteLogout,
    /// Begin local wipe of Matrix stores for this account.
    BeginWipe,
    /// Wipe finished; return to empty.
    CompleteWipe,
    /// Record a privacy-safe failure; drops live client handle.
    Fail,
    /// Clear failed/logged-out back to empty without wipe (harness / idle).
    ResetEmpty,
}

impl SupervisorCommand {
    pub const ALL: &'static [SupervisorCommand] = &[
        Self::BeginOpen,
        Self::BeginAuthenticate,
        Self::BeginRestore,
        Self::InstallClient,
        Self::BeginSync,
        Self::MarkReady,
        Self::BeginStop,
        Self::CompleteLogout,
        Self::BeginWipe,
        Self::CompleteWipe,
        Self::Fail,
        Self::ResetEmpty,
    ];
}

/// Side effects the actor must apply when a transition succeeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TransitionEffect {
    /// Bump `session_generation` (new logical session epoch).
    pub bump_generation: bool,
    /// Shut down and drop the installed client handle.
    pub drop_client: bool,
    /// Clear last failure info.
    pub clear_failure: bool,
    /// Record that this transition expects a client to already be installed.
    pub require_client: bool,
    /// Record that this transition installs a new client via the factory.
    pub install_client: bool,
}

/// Resolve pure transition: next lifecycle + effects, or error.
pub fn resolve(
    from: SupervisorState,
    command: SupervisorCommand,
) -> Result<(SupervisorState, TransitionEffect), TransitionError> {
    use SupervisorCommand as C;
    use SupervisorState as S;

    let illegal = |reason: &'static str| TransitionError::new(from, command, reason);

    match (from, command) {
        // --- Open / bootstrap ---
        (S::Empty | S::LoggedOut | S::Failed, C::BeginOpen) => Ok((
            S::Opening,
            TransitionEffect {
                bump_generation: true,
                drop_client: true, // belt-and-braces: no stale handle
                clear_failure: true,
                ..TransitionEffect::default()
            },
        )),

        // --- Auth vs restore (mutually exclusive after open) ---
        (S::Opening, C::BeginAuthenticate) => Ok((
            S::Authenticating,
            TransitionEffect {
                clear_failure: true,
                ..TransitionEffect::default()
            },
        )),
        (S::Opening, C::BeginRestore) => Ok((
            S::Restoring,
            TransitionEffect {
                clear_failure: true,
                ..TransitionEffect::default()
            },
        )),

        // --- Sole client construction path ---
        (S::Opening | S::Authenticating | S::Restoring, C::InstallClient) => Ok((
            from, // state unchanged until sync/ready
            TransitionEffect {
                install_client: true,
                clear_failure: true,
                ..TransitionEffect::default()
            },
        )),

        // --- Sync / ready ---
        (S::Authenticating | S::Restoring, C::BeginSync) => Ok((
            S::Syncing,
            TransitionEffect {
                require_client: true,
                clear_failure: true,
                ..TransitionEffect::default()
            },
        )),
        (S::Syncing, C::MarkReady) => Ok((
            S::Ready,
            TransitionEffect {
                require_client: true,
                clear_failure: true,
                ..TransitionEffect::default()
            },
        )),

        // --- Stop / logout (D-LOGOUT-WIPE: logout ≠ wipe) ---
        (
            S::Ready | S::Syncing | S::Authenticating | S::Restoring | S::Opening | S::Failed,
            C::BeginStop,
        ) => Ok((
            S::Stopping,
            TransitionEffect {
                ..TransitionEffect::default()
            },
        )),
        (S::Stopping, C::CompleteLogout) => Ok((
            S::LoggedOut,
            TransitionEffect {
                drop_client: true,
                clear_failure: true,
                // Retire generation so in-flight work cannot publish as live.
                bump_generation: true,
                ..TransitionEffect::default()
            },
        )),

        // --- Wipe (destructive; may start from several terminal/error states) ---
        (
            S::Ready
            | S::LoggedOut
            | S::Failed
            | S::Stopping
            | S::Syncing
            | S::Authenticating
            | S::Restoring
            | S::Opening,
            C::BeginWipe,
        ) => Ok((
            S::Wiping,
            TransitionEffect {
                // R0.5 / REV-001: drop the live client *before* destructive wipe
                // so SQLite handles cannot race against store deletion.
                drop_client: true,
                ..TransitionEffect::default()
            },
        )),
        (S::Wiping, C::CompleteWipe) => Ok((
            S::Empty,
            TransitionEffect {
                drop_client: true, // idempotent if BeginWipe already dropped
                clear_failure: true,
                bump_generation: true, // epoch after wipe so stale IPC dies
                ..TransitionEffect::default()
            },
        )),

        // --- Fail from active states including wipe I/O failure (P2.6) ---
        // Wiping is included so a failed exact-target wipe does not complete
        // the wipe epoch and never auto-deletes further data.
        (
            S::Opening
            | S::Authenticating
            | S::Restoring
            | S::Syncing
            | S::Ready
            | S::Stopping
            | S::Wiping,
            C::Fail,
        ) => Ok((
            S::Failed,
            TransitionEffect {
                drop_client: true,
                ..TransitionEffect::default()
            },
        )),

        // --- Idle reset ---
        (S::Failed | S::LoggedOut, C::ResetEmpty) => Ok((
            S::Empty,
            TransitionEffect {
                drop_client: true,
                clear_failure: true,
                ..TransitionEffect::default()
            },
        )),

        // --- Explicit illegal cases with clear reasons ---
        (S::Empty, C::BeginAuthenticate | C::BeginRestore | C::BeginSync | C::MarkReady) => {
            Err(illegal("must BeginOpen from empty before auth/sync"))
        }
        (S::Ready, C::BeginOpen | C::BeginAuthenticate | C::BeginRestore) => {
            Err(illegal("stop or wipe before opening another session"))
        }
        (S::Wiping, c) if c != C::CompleteWipe && c != C::Fail => {
            // Fail is matched earlier (P2.6 wipe I/O failure path).
            Err(illegal(
                "wipe in progress; only CompleteWipe or Fail is allowed",
            ))
        }
        (S::LoggedOut, C::CompleteLogout) => Err(illegal("already logged out")),
        (S::Empty, C::CompleteWipe | C::BeginWipe) => Err(illegal("nothing to wipe from empty")),
        (S::Authenticating, C::BeginRestore) | (S::Restoring, C::BeginAuthenticate) => {
            Err(illegal("auth and restore paths are mutually exclusive"))
        }
        (S::Opening, C::BeginSync) => Err(illegal("must authenticate or restore before sync")),
        (S::Empty | S::LoggedOut, C::InstallClient) => {
            Err(illegal("must BeginOpen before InstallClient"))
        }
        (S::Ready | S::Syncing | S::Stopping | S::Wiping | S::Failed, C::InstallClient) => {
            Err(illegal("InstallClient only during opening/auth/restore"))
        }
        _ => Err(illegal("transition not permitted")),
    }
}
