//! P2.6 / P3.5 / P3.6 / P3.8 — Destructive lifecycle + session + remote logout.
//!
//! Shared-core harness / foundation only until cutover. Owns the privacy-safe
//! [`LifecycleError`] domain, recovery / transition UX copy, failed-store
//! recovery policy (never auto-wipe), remote-logout state machine, and
//! exact-target account wipe, logout / local-wipe orchestration, and the
//! session-material vault *trait* plus sealed envelope / in-memory harness.
//! Desktop still owns Keyring session I/O.

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod logout;
mod recovery;
mod recovery_copy;
mod remote_logout;
mod remote_policy;
mod session_material;
mod session_persist;
mod session_restore;
mod wipe;

pub use error::LifecycleError;
pub use logout::{perform_local_wipe, perform_logout, LogoutOutcome, WipeOutcome};
pub use recovery::{
    apply_store_failure, recovery_action_for, surface_store_corrupt, surface_store_unavailable,
    RecoveryAction, StoreFailure, StoreFailureKind,
};
pub use recovery_copy::{copy_for_remote_outcome, recovery_copy_en, RecoveryCopyKey};
pub use remote_logout::{RemoteLogoutFlow, RemoteLogoutOutcome, RemoteLogoutPhase};
pub use remote_policy::{LocalCleanupPolicy, RemoteLogoutScope};
pub use session_material::{
    clear_session_material, load_session_material, persist_session_material,
    rotate_persisted_session_tokens, HostMatrixSessionSecrets, InMemorySessionMaterialVault,
    SessionMaterial, SessionMaterialId, SessionMaterialMeta, SessionMaterialVault,
    SESSION_ENVELOPE_VERSION, SESSION_KIND_MATRIX, SESSION_MATERIAL_SERVICE,
};
pub use session_persist::{
    persist_session_after_login, session_material_from_auth_session, SessionPersistOutcome,
};
pub use session_restore::{
    has_persisted_session, matrix_session_from_host_secrets, restore_session_from_vault,
    restore_session_onto_client, SessionRestoreOutcome,
};
pub use wipe::{
    assert_exact_account_root, assert_path_is_wipe_allowed, wipe_account_store, WipeReport,
    WipeTarget, WIPE_TARGET_KIND_ACCOUNT_ROOT,
};
