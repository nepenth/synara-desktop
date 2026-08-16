//! P8.3 — Verification request inbox + SAS display foundation (harness).
//!
//! Pure index plus live `NativeVerificationOwner`. **No SAS secrets, MAC
//! keys, recovery material, or tokens.** Display-only emoji short names
//! only. Desktop maps diagnostic ids onto Tauri command errors.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p8.3-verification.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod inbox;
mod live;
mod native;

pub use error::VerificationError;
pub use inbox::{
    VerificationDirection, VerificationFlow, VerificationInbox, VerificationPhase, MAX_OPEN_FLOWS,
    MAX_SAS_EMOJI,
};
pub use live::NativeVerificationOwner;
pub use native::{
    phase_rank, NativeVerificationDirection, NativeVerificationEmoji, NativeVerificationInbox,
    NativeVerificationPhase, NativeVerificationRequest, NativeVerificationSas,
};

/// Static marker for link / schema smoke.
pub const MATRIX_VERIFICATION_MARKER: &str = "matrix-verification-p8.3";

/// Touch verification paths so they remain linked in non-test builds.
pub fn matrix_verification_markers() -> &'static str {
    let inbox = VerificationInbox::new(0);
    debug_assert!(inbox.is_empty());
    debug_assert_eq!(inbox.len(), 0);
    debug_assert!(!inbox.has_pending_attention());
    debug_assert_eq!(MATRIX_VERIFICATION_MARKER, "matrix-verification-p8.3");
    MATRIX_VERIFICATION_MARKER
}

#[cfg(test)]
mod tests;
