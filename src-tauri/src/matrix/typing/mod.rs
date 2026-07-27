//! P6.3 — Typing index foundation (harness).
//!
//! Pure projection of Synara [`TypingSnapshot`] DTOs. No SDK typing send,
//! no production Tauri commands, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p6.3-typing.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod index;

pub use error::TypingError;
pub use index::{TypingIndex, MAX_TYPING_USERS_PER_ROOM};

/// Static marker for link / schema smoke.
pub const MATRIX_TYPING_MARKER: &str = "matrix-typing-p6.3";

/// Touch typing paths so they remain linked in non-test builds.
pub fn matrix_typing_markers() -> &'static str {
    let idx = TypingIndex::new(0);
    debug_assert!(idx.is_empty());
    debug_assert_eq!(MAX_TYPING_USERS_PER_ROOM, 32);
    debug_assert_eq!(MATRIX_TYPING_MARKER, "matrix-typing-p6.3");
    MATRIX_TYPING_MARKER
}

#[cfg(test)]
mod tests;
