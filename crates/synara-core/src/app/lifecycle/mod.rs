//! P2.6 / P3.5 / P3.6 / P3.8 — Destructive lifecycle + session + remote logout.
//!
//! Shared-core harness / foundation only until cutover. Owns the privacy-safe
//! [`LifecycleError`] domain, recovery / transition UX copy, failed-store
//! recovery policy (never auto-wipe), remote-logout state machine, and
//! exact-target account wipe. Desktop still owns logout orchestration,
//! session vault I/O, persist/restore (`matrix_sdk`), and tests.

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod recovery;
mod recovery_copy;
mod remote_logout;
mod remote_policy;
mod wipe;

pub use error::LifecycleError;
pub use recovery::{
    apply_store_failure, recovery_action_for, surface_store_corrupt, surface_store_unavailable,
    RecoveryAction, StoreFailure, StoreFailureKind,
};
pub use recovery_copy::{copy_for_remote_outcome, recovery_copy_en, RecoveryCopyKey};
pub use remote_logout::{RemoteLogoutFlow, RemoteLogoutOutcome, RemoteLogoutPhase};
pub use remote_policy::{LocalCleanupPolicy, RemoteLogoutScope};
pub use wipe::{
    assert_exact_account_root, assert_path_is_wipe_allowed, wipe_account_store, WipeReport,
    WipeTarget, WIPE_TARGET_KIND_ACCOUNT_ROOT,
};
