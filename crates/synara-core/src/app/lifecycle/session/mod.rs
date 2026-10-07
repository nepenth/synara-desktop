//! Platform-neutral session lifecycle policy shared by the desktop shell and
//! the iOS `SharedCore`.
//!
//! Shells keep their own I/O (keyring, Swift vault, app-data root, events);
//! this module owns the decisions both platforms must make the same way:
//! generation numbering, retry backoff, rejected-session retirement, bounded
//! remote logout and the credential persistence lease.

mod backoff;
mod fault;
mod generation;
mod locator;
mod logout;
mod persistence;
mod rejection;
mod rotation;

pub use backoff::RetryBackoff;
pub use fault::{HasDiagnosticId, SessionFault, SessionFaultKind};
pub use generation::SessionGenerations;
pub use locator::SessionLocator;
pub use logout::{
    bounded_remote_logout, finish_active_logout, finish_orphan_logout, finish_taken_session_logout,
    take_session_for_logout, wait_for_backup_steady_state, BackupSteadyState, PendingLogoutCleanup,
    LOGOUT_BACKUP_STEADY_STATE_TIMEOUT, VOLUNTARY_REMOTE_LOGOUT_TIMEOUT,
};
pub use persistence::{
    persist_with_client_lease, SessionPersistenceLease, SessionPersistenceOwner,
};
pub use rejection::{
    handle_authentication_rejection_tick, remote_logout_allowed,
    static_rejection_logout_diagnostic, AuthenticationRejectionWatch,
    SESSION_AUTHENTICATION_REJECTED_LOG_LINE, SESSION_REJECTION_NO_CORE_DIAGNOSTIC_ID,
};
pub use rotation::{
    install_session_rotation_callbacks, RotationDiagnostics, RotationHooks,
    SessionRotationCallbackError,
};
