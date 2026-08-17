//! P6.3 typing index foundation (harness).
//!
//! Pure [`TypingSnapshot`] projection plus live `m.typing` ownership.
//! Tauri commands stay in the desktop shell.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p6.3-typing.md`
//! Product vertical: `docs/matrix-rust-sdk/v-rooms-4-typing.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod index;
mod live;
mod native;

pub use error::TypingError;
pub use index::{TypingIndex, MAX_TYPING_USERS_PER_ROOM};
pub use live::{set_typing_notice, NativeTypingOwner, NativeTypingUpdateSignal, TypingUpdateEmit};
pub use native::NativeTypingSnapshot;

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
