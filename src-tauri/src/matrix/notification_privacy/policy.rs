//! Metadata-only notification privacy policy.

/// Privacy level to use for a notification candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EncryptionPrivacyMode {
    /// The notification may identify the sender, but must not include event text.
    ShowSenderOnly,
    /// The notification must use a generic, content-free preview.
    ShowRedacted,
    /// The notification must not be emitted.
    Suppress,
}

/// Content-free metadata used by [`apply_policy`].
///
/// `lock_screen_safe_preview` is an assertion by the candidate producer that a
/// generic, content-free preview is available. It does not authorize sender or
/// event text while the app is locked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CandidateMeta {
    pub lock_screen_safe_preview: bool,
}

impl CandidateMeta {
    pub const fn new(lock_screen_safe_preview: bool) -> Self {
        Self {
            lock_screen_safe_preview,
        }
    }
}

/// Metadata-only result of applying notification privacy policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrivacyDecision {
    pub allowed: bool,
    pub mode: EncryptionPrivacyMode,
    /// Stable diagnostic identifier. Contains no candidate or event data.
    pub reason_diagnostic_id: &'static str,
}

pub const REASON_LOCKED_PREVIEW_NOT_SAFE: &str = "p9.5-locked-preview-not-safe";
pub const REASON_LOCKED_SAFE_REDACTED: &str = "p9.5-locked-safe-preview-redacted";
pub const REASON_ENCRYPTED_SENDER_ONLY: &str = "p9.5-encrypted-sender-only";
pub const REASON_UNENCRYPTED_REDACTED: &str = "p9.5-unencrypted-preview-redacted";

/// Apply privacy policy to content-free candidate metadata.
///
/// Locked state takes precedence over room encryption:
///
/// - an unsafe locked-screen preview is suppressed;
/// - a safe locked-screen preview is always redacted;
/// - an encrypted-room preview while unlocked is sender-only;
/// - an unencrypted-room preview while unlocked is redacted.
pub const fn apply_policy(
    candidate_meta: CandidateMeta,
    room_is_encrypted: bool,
    app_locked: bool,
) -> PrivacyDecision {
    if app_locked {
        if candidate_meta.lock_screen_safe_preview {
            return PrivacyDecision {
                allowed: true,
                mode: EncryptionPrivacyMode::ShowRedacted,
                reason_diagnostic_id: REASON_LOCKED_SAFE_REDACTED,
            };
        }

        return PrivacyDecision {
            allowed: false,
            mode: EncryptionPrivacyMode::Suppress,
            reason_diagnostic_id: REASON_LOCKED_PREVIEW_NOT_SAFE,
        };
    }

    if room_is_encrypted {
        PrivacyDecision {
            allowed: true,
            mode: EncryptionPrivacyMode::ShowSenderOnly,
            reason_diagnostic_id: REASON_ENCRYPTED_SENDER_ONLY,
        }
    } else {
        PrivacyDecision {
            allowed: true,
            mode: EncryptionPrivacyMode::ShowRedacted,
            reason_diagnostic_id: REASON_UNENCRYPTED_REDACTED,
        }
    }
}
