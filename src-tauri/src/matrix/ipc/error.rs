//! Stable Synara Matrix IPC error categories (plan §6.4).
//!
//! Privacy: tokens, credentials, recovery keys, event plaintext, raw push
//! payloads, and decrypted media must never appear in error fields. Use
//! opaque `diagnostic_id` codes only for diagnostics.

use serde::{Deserialize, Serialize};

/// Stable error categories — never pass raw SDK / homeserver strings as the
/// category discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatrixIpcErrorCategory {
    /// Access token invalid, missing, or rejected.
    AuthenticationRejected,
    /// Account deactivated on the homeserver.
    UserDeactivated,
    /// Interactive authentication (UIA) required to continue.
    InteractiveAuthRequired,
    /// Forbidden / insufficient power level.
    Forbidden,
    /// Rate limited; may include `retry_after_ms`.
    RateLimited,
    /// Local connectivity / offline.
    Connectivity,
    /// Homeserver unreachable or returning hard unavailability.
    HomeserverUnavailable,
    /// Homeserver lacks a required capability.
    UnsupportedCapability,
    /// Encrypted store is locked (e.g. passphrase / OS key material).
    StoreLocked,
    /// Store integrity failure.
    StoreCorrupt,
    /// Store missing or cannot be opened.
    StoreUnavailable,
    /// Crypto subsystem failure (non-recovery, non-verification).
    CryptoFailure,
    /// Recovery key / secret-storage / key-backup failure.
    RecoveryFailure,
    /// Device / user verification failure.
    VerificationFailure,
    /// Media exceeds size limits.
    MediaTooLarge,
    /// Media type or encoding unsupported.
    MediaUnsupported,
    /// Media decryption failed.
    MediaDecryptFailed,
    /// Operation cancelled by client or supervisor.
    Cancellation,
    /// Envelope `sessionGeneration` does not match the live session.
    StaleSessionGeneration,
    /// Internal SDK or bridge invariant violated.
    SdkInvariant,
    /// Unclassified failure; must carry a privacy-safe `diagnostic_id`.
    Unknown,
}

impl MatrixIpcErrorCategory {
    /// All stable categories (for exhaustiveness tests / docs).
    pub const ALL: &'static [MatrixIpcErrorCategory] = &[
        Self::AuthenticationRejected,
        Self::UserDeactivated,
        Self::InteractiveAuthRequired,
        Self::Forbidden,
        Self::RateLimited,
        Self::Connectivity,
        Self::HomeserverUnavailable,
        Self::UnsupportedCapability,
        Self::StoreLocked,
        Self::StoreCorrupt,
        Self::StoreUnavailable,
        Self::CryptoFailure,
        Self::RecoveryFailure,
        Self::VerificationFailure,
        Self::MediaTooLarge,
        Self::MediaUnsupported,
        Self::MediaDecryptFailed,
        Self::Cancellation,
        Self::StaleSessionGeneration,
        Self::SdkInvariant,
        Self::Unknown,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::AuthenticationRejected => "authentication_rejected",
            Self::UserDeactivated => "user_deactivated",
            Self::InteractiveAuthRequired => "interactive_auth_required",
            Self::Forbidden => "forbidden",
            Self::RateLimited => "rate_limited",
            Self::Connectivity => "connectivity",
            Self::HomeserverUnavailable => "homeserver_unavailable",
            Self::UnsupportedCapability => "unsupported_capability",
            Self::StoreLocked => "store_locked",
            Self::StoreCorrupt => "store_corrupt",
            Self::StoreUnavailable => "store_unavailable",
            Self::CryptoFailure => "crypto_failure",
            Self::RecoveryFailure => "recovery_failure",
            Self::VerificationFailure => "verification_failure",
            Self::MediaTooLarge => "media_too_large",
            Self::MediaUnsupported => "media_unsupported",
            Self::MediaDecryptFailed => "media_decrypt_failed",
            Self::Cancellation => "cancellation",
            Self::StaleSessionGeneration => "stale_session_generation",
            Self::SdkInvariant => "sdk_invariant",
            Self::Unknown => "unknown",
        }
    }
}

/// Privacy-safe error payload carried by `kind: "error"` envelopes.
///
/// Explicitly **excludes** fields for tokens, credentials, recovery keys,
/// event plaintext, raw push payloads, or decrypted media bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixIpcError {
    pub category: MatrixIpcErrorCategory,
    /// Optional short, privacy-safe summary suitable for logs/UI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Opaque diagnostic code (never includes secrets or event bodies).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic_id: Option<String>,
    /// Suggested client retry delay for rate limits / transient failures.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
    /// Correlates to the request that failed, when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl MatrixIpcError {
    pub fn new(category: MatrixIpcErrorCategory) -> Self {
        Self {
            category,
            message: None,
            diagnostic_id: None,
            retry_after_ms: None,
            request_id: None,
        }
    }

    pub fn with_diagnostic(mut self, diagnostic_id: impl Into<String>) -> Self {
        self.diagnostic_id = Some(diagnostic_id.into());
        self
    }

    pub fn with_retry_after_ms(mut self, ms: u64) -> Self {
        self.retry_after_ms = Some(ms);
        self
    }
}
