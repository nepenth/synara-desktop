//! P6.3 typing index foundation (harness).
//!
//! Pure [`TypingSnapshot`] projection plus live `m.typing` ownership.
//! Tauri commands stay in the desktop shell.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p6.3-typing.md`
//! Product vertical: `docs/matrix-rust-sdk/v-rooms-4-typing.md`

mod error;
mod index;
mod live;
mod native;

pub use error::TypingError;
pub use index::{TypingIndex, MAX_TYPING_USERS_PER_ROOM};
pub use live::{set_typing_notice, NativeTypingOwner, NativeTypingUpdateSignal, TypingUpdateEmit};
pub use native::NativeTypingSnapshot;

#[cfg(test)]
mod tests;
