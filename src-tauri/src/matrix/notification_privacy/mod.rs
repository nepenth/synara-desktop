//! P9.5 — Encrypted notification privacy behavior foundation (pure harness).
//!
//! Policy decisions use metadata only. This module does not accept event content,
//! perform decryption, post OS notifications, or select a Matrix backend.
//!
//! Authoritative design note:
//! `docs/matrix-rust-sdk/p9.5-encrypted-notify-privacy.md`

mod policy;

pub use policy::{
    apply_policy, CandidateMeta, EncryptionPrivacyMode, PrivacyDecision,
    REASON_ENCRYPTED_SENDER_ONLY, REASON_LOCKED_PREVIEW_NOT_SAFE, REASON_LOCKED_SAFE_REDACTED,
    REASON_UNENCRYPTED_REDACTED,
};

/// Static marker for link / schema smoke.
pub const MATRIX_NOTIFICATION_PRIVACY_MARKER: &str = "matrix-notification-privacy-p9.5";

/// Touch notification privacy paths so they remain linked in non-test builds.
pub fn matrix_notification_privacy_markers() -> &'static str {
    let decision: PrivacyDecision = apply_policy(CandidateMeta::new(false), false, false);
    let _modes = [
        EncryptionPrivacyMode::ShowSenderOnly,
        EncryptionPrivacyMode::ShowRedacted,
        EncryptionPrivacyMode::Suppress,
    ];
    let _reasons = [
        REASON_ENCRYPTED_SENDER_ONLY,
        REASON_LOCKED_PREVIEW_NOT_SAFE,
        REASON_LOCKED_SAFE_REDACTED,
        REASON_UNENCRYPTED_REDACTED,
    ];
    debug_assert!(decision.allowed);
    debug_assert_eq!(decision.mode, EncryptionPrivacyMode::ShowRedacted);
    debug_assert_eq!(_modes.len(), 3);
    debug_assert_eq!(_reasons.len(), 4);
    debug_assert_eq!(
        MATRIX_NOTIFICATION_PRIVACY_MARKER,
        "matrix-notification-privacy-p9.5"
    );
    MATRIX_NOTIFICATION_PRIVACY_MARKER
}

#[cfg(test)]
mod tests;
