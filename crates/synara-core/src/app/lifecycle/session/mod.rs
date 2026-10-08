//! Platform-neutral session lifecycle shared by the desktop shell and the iOS
//! `SharedCore`.
//!
//! Shells keep their own I/O (keyring, Swift vault, app-data root, events) and
//! owner construction; this module owns what both platforms must do the same
//! way: the session slot and its transition gate ([`SessionLifecycleOwner`]),
//! generation numbering, retry backoff and periodic maintenance,
//! rejected-session retirement, bounded remote logout and the teardown policy,
//! install rollback, the credential persistence lease, rotation callbacks, the
//! durable logout locator, sync recovery after wake, the store-recovery
//! capability and the canonical Core owner wiring.

mod backoff;
mod fault;
mod generation;
mod install;
mod journal;
mod locator;
mod locator_file;
mod logout;
mod maintenance;
mod owner;
mod owners;
mod persistence;
mod recover;
mod rejection;
mod rotation;
mod store_recovery;

pub use backoff::RetryBackoff;
pub use fault::{HasDiagnosticId, SessionFault, SessionFaultKind};
pub use generation::SessionGenerations;
pub use install::{
    accept_authenticated_identity, finish_install_rollback, finish_session_preparation,
    finish_session_wiring, with_generation_bound_acceptance, SessionInstallOrigin,
};
pub use journal::{
    is_journal_diagnostic, record_session_rotation_outcome, rotation_persist_diagnostic,
    ROTATION_PERSIST_FAILED_DIAGNOSTIC_ID,
};
pub use locator::SessionLocator;
pub use locator_file::{
    active_locator_path, clear_native_logout_material, clear_persisted_logout_material,
    ensure_logout_retry_locator, ensure_logout_retry_locator_with_directory_sync,
    locator_account_identity, read_active_locator, remove_active_locator, write_active_locator,
    ACTIVE_SESSION_FILE, MATRIX_DATA_DIR,
};
pub use logout::{
    bounded_remote_logout, finish_active_logout, finish_orphan_logout, finish_taken_session_logout,
    run_local_logout, take_session_for_logout, wait_for_backup_steady_state, BackupSteadyState,
    CredentialCleanup, LogoutPolicy, PendingLogoutCleanup, TeardownOrder,
    LOGOUT_BACKUP_STEADY_STATE_TIMEOUT, VOLUNTARY_REMOTE_LOGOUT_TIMEOUT,
};
pub use maintenance::{MaintenanceReport, RejectedGeneration, SessionMaintenance};
pub use owner::{InstalledSession, LogoutOutcome, SessionInstallGuard, SessionLifecycleOwner};
pub use owners::{attach_owner_set, OwnerKind, SessionOwnerSet, OWNER_ATTACH_ORDER};
pub use persistence::{
    persist_with_client_lease, SessionPersistenceLease, SessionPersistenceOwner,
};
pub use recover::{recover_installed_session_owner, RecoverGate};
pub use rejection::{
    handle_authentication_rejection_tick, remote_logout_allowed,
    static_rejection_logout_diagnostic, AuthenticationRejectionWatch,
    SESSION_AUTHENTICATION_REJECTED_LOG_LINE, SESSION_REJECTION_NO_CORE_DIAGNOSTIC_ID,
};
pub use rotation::{
    install_session_rotation_callbacks, RotationDiagnostics, RotationHooks,
    SessionRotationCallbackError,
};
pub use store_recovery::{
    is_recoverable_store_login_diagnostic, StoreRecoveryState,
    STORE_RECOVERY_CONFIRMATION_REQUIRED, STORE_RECOVERY_TYPED_CONFIRMATION_TEXT,
};
