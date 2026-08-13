//! P2.6 / P3.5 / P3.6 / P3.8 — Destructive lifecycle + session + remote logout.
//!
//! Shared-core harness / foundation only until cutover. This module currently
//! holds the recovery / transition UX copy catalog (P3.8 harness foundation)
//! extracted from the desktop shell. Desktop-owned lifecycle modules (error,
//! logout, recovery, remote_logout, session_material, session_persist,
//! session_restore, wipe, tests) remain in `src-tauri/src/matrix/lifecycle/`
//! because they depend on desktop host stores or `matrix_sdk`.

#![allow(dead_code)]
#![allow(unused_imports)]

mod recovery_copy;
mod remote_policy;

pub use recovery_copy::{copy_for_remote_outcome, recovery_copy_en, RecoveryCopyKey};
pub use remote_policy::{LocalCleanupPolicy, RemoteLogoutScope};
