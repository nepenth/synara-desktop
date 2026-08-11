//! Product-neutral sync activity phase (SNC-P1-5a seam).
//!
//! Moved from src-tauri `matrix/diagnostics/health.rs` into the shared core so
//! the core sync module can depend on it without an src-tauri import cycle.
//! `health.rs` re-exports this name so every `crate::matrix::diagnostics::
//! SyncPhase` path keeps resolving identically.

use serde::{Deserialize, Serialize};

/// High-level sync activity phase (product-neutral; no homeserver details).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncPhase {
    Idle,
    CatchingUp,
    Live,
    Reconnecting,
    Paused,
    Failed,
}

impl SyncPhase {
    pub const ALL: &'static [SyncPhase] = &[
        Self::Idle,
        Self::CatchingUp,
        Self::Live,
        Self::Reconnecting,
        Self::Paused,
        Self::Failed,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::CatchingUp => "catching_up",
            Self::Live => "live",
            Self::Reconnecting => "reconnecting",
            Self::Paused => "paused",
            Self::Failed => "failed",
        }
    }
}
