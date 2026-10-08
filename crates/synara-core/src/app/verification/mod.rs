//! P8.3 — Verification request inbox + SAS display foundation (harness).
//!
//! Pure index plus live `NativeVerificationOwner`. **No SAS secrets, MAC
//! keys, recovery material, or tokens.** Display-only emoji short names
//! only. Desktop maps diagnostic ids onto Tauri command errors.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p8.3-verification.md`

mod error;
mod identity_warnings;
mod inbox;
mod live;
mod native;
mod publication;
#[cfg(test)]
mod publication_tests;
#[cfg(test)]
mod two_device_tests;

pub use error::VerificationError;
pub use identity_warnings::{
    warnings_from_states, IdentityWarningError, NativeIdentityWarningAction,
    NativeIdentityWarningKind, NativeIdentityWarningResolveRequest, NativeRoomIdentityWarning,
    NativeRoomIdentityWarnings, NativeRoomIdentityWarningsRequest,
    NATIVE_ROOM_IDENTITY_WARNINGS_SCHEMA_VERSION,
};
pub use inbox::{
    VerificationDirection, VerificationFlow, VerificationInbox, VerificationPhase, MAX_OPEN_FLOWS,
    MAX_SAS_EMOJI,
};
pub use live::{NativeVerificationOwner, NativeVerificationUpdateSignal, VerificationUpdateEmit};
pub use native::{
    capped_qr_image_data_url, compare_for_inbox, phase_rank, NativeVerificationDirection,
    NativeVerificationEmoji, NativeVerificationInbox, NativeVerificationPhase,
    NativeVerificationQr, NativeVerificationRequest, NativeVerificationSas,
    MAX_QR_IMAGE_DATA_URL_CHARS,
};

/// Tauri event: verification inbox/SAS may have changed. Signal only; UI
/// re-lists via `matrix_verification_list`. Never carries keys, MACs, or tokens.
pub const VERIFICATION_UPDATED_EVENT: &str = "matrix-verification-updated";

#[cfg(test)]
mod tests;
