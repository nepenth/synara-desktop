//! P8.7 — UTD retry / encrypted-history recovery foundation (src-tauri adapter).
//!
//! SNC-P1-5c: the UTD recovery logic now lives in the shared native core at
//! `crates/synara-core/src/app/utd_recovery` (pulled forward with the timeline
//! chunk because `app::timeline::live` has a hard type dependency on
//! [`UtdRecoveryCoordinator`]). This module keeps every
//! `crate::matrix::utd_recovery::*` path resolving with **identical behavior**
//! (same pattern as the P1.5a/b adapters).
//!
//! `tests.rs` stays here (adapter side) and exercises the re-exported core
//! types via `super::*`.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p8.7-utd-recovery.md`

#![allow(dead_code)]
#![allow(unused_imports)]

pub use synara_core::app::utd_recovery::{
    matrix_utd_recovery_markers, UtdRecoveryCoordinator, UtdRecoveryError, UtdRecoveryKind,
    UtdRecoveryPhase, UtdRecoverySession, MATRIX_UTD_RECOVERY_MARKER, MAX_EVENT_IDS_PER_BATCH,
    MAX_ROOM_SESSIONS,
};

#[cfg(test)]
mod tests;
