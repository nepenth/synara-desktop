//! Privacy-safe discovery / login-flow / login errors (P3.1 + P3.2).
//!
//! Tokens, passwords, recovery keys, and raw homeserver error bodies must never
//! appear in error messages or diagnostic fields.

use std::fmt;

use crate::transport::MatrixIpcErrorCategory;

/// Failure while validating input, discovering a homeserver, listing login
/// flows, or performing password login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// User-supplied homeserver URL, server name, user id, or device name is empty/malformed.
    InvalidInput {
        diagnostic_id: &'static str,
        /// Short privacy-safe reason (no secrets / tokens).
        reason: &'static str,
    },
    /// Well-known or homeserver endpoint unreachable / offline.
    Connectivity { diagnostic_id: &'static str },
    /// Homeserver hard-unavailable (5xx / DNS permanent / refused).
    HomeserverUnavailable { diagnostic_id: &'static str },
    /// Well-known document missing (HTTP 404 / IGNORE path in product autoDiscovery).
    ///
    /// Callers may fall back to `https://{server_name}` as base URL (product IGNORE).
    WellKnownNotFound { diagnostic_id: &'static str },
    /// Well-known missing, invalid, or homeserver lacks a required capability.
    UnsupportedCapability { diagnostic_id: &'static str },
    /// Password login rejected (wrong credentials, forbidden, unknown token).
    ///
    /// Never includes the password, one-time login token, or access token.
    AuthenticationRejected { diagnostic_id: &'static str },
    /// Homeserver reports the account is deactivated.
    UserDeactivated { diagnostic_id: &'static str },
    /// Interactive authentication (UIA) required to continue login (P3.4 owns full UIA).
    InteractiveAuthRequired { diagnostic_id: &'static str },
    /// Rate limited by the homeserver during login.
    RateLimited {
        diagnostic_id: &'static str,
        /// Optional retry hint in milliseconds (never a secret).
        retry_after_ms: Option<u64>,
    },
    /// Transport / protocol invariant violated (redacted).
    SdkInvariant { diagnostic_id: &'static str },
    /// Unclassified failure with opaque diagnostic id only.
    Unknown { diagnostic_id: &'static str },
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput {
                diagnostic_id,
                reason,
            } => write!(f, "invalid auth input ({diagnostic_id}): {reason}"),
            Self::Connectivity { diagnostic_id } => {
                write!(f, "auth connectivity failure ({diagnostic_id})")
            }
            Self::HomeserverUnavailable { diagnostic_id } => {
                write!(f, "homeserver unavailable ({diagnostic_id})")
            }
            Self::WellKnownNotFound { diagnostic_id } => {
                write!(f, "well-known not found ({diagnostic_id})")
            }
            Self::UnsupportedCapability { diagnostic_id } => {
                write!(f, "unsupported homeserver capability ({diagnostic_id})")
            }
            Self::AuthenticationRejected { diagnostic_id } => {
                write!(f, "authentication rejected ({diagnostic_id})")
            }
            Self::UserDeactivated { diagnostic_id } => {
                write!(f, "user deactivated ({diagnostic_id})")
            }
            Self::InteractiveAuthRequired { diagnostic_id } => {
                write!(f, "interactive auth required ({diagnostic_id})")
            }
            Self::RateLimited {
                diagnostic_id,
                retry_after_ms,
            } => match retry_after_ms {
                Some(ms) => write!(
                    f,
                    "auth rate limited ({diagnostic_id}); retry_after_ms={ms}"
                ),
                None => write!(f, "auth rate limited ({diagnostic_id})"),
            },
            Self::SdkInvariant { diagnostic_id } => {
                write!(f, "auth invariant failure ({diagnostic_id})")
            }
            Self::Unknown { diagnostic_id } => {
                write!(f, "unknown auth failure ({diagnostic_id})")
            }
        }
    }
}

impl std::error::Error for AuthError {}

impl AuthError {
    /// Map to a stable IPC error category (plan §6.4). Never carries secrets.
    pub fn category(&self) -> MatrixIpcErrorCategory {
        match self {
            Self::InvalidInput { .. } => MatrixIpcErrorCategory::SdkInvariant,
            Self::Connectivity { .. } => MatrixIpcErrorCategory::Connectivity,
            Self::HomeserverUnavailable { .. } => MatrixIpcErrorCategory::HomeserverUnavailable,
            // Product IGNORE still yields a usable base URL after fallback; when
            // surfaced without fallback, treat as unsupported discovery.
            Self::WellKnownNotFound { .. } => MatrixIpcErrorCategory::UnsupportedCapability,
            Self::UnsupportedCapability { .. } => MatrixIpcErrorCategory::UnsupportedCapability,
            Self::AuthenticationRejected { .. } => MatrixIpcErrorCategory::AuthenticationRejected,
            Self::UserDeactivated { .. } => MatrixIpcErrorCategory::UserDeactivated,
            Self::InteractiveAuthRequired { .. } => MatrixIpcErrorCategory::InteractiveAuthRequired,
            Self::RateLimited { .. } => MatrixIpcErrorCategory::RateLimited,
            Self::SdkInvariant { .. } => MatrixIpcErrorCategory::SdkInvariant,
            Self::Unknown { .. } => MatrixIpcErrorCategory::Unknown,
        }
    }

    pub fn diagnostic_id(&self) -> &'static str {
        match self {
            Self::InvalidInput { diagnostic_id, .. }
            | Self::Connectivity { diagnostic_id }
            | Self::HomeserverUnavailable { diagnostic_id }
            | Self::WellKnownNotFound { diagnostic_id }
            | Self::UnsupportedCapability { diagnostic_id }
            | Self::AuthenticationRejected { diagnostic_id }
            | Self::UserDeactivated { diagnostic_id }
            | Self::InteractiveAuthRequired { diagnostic_id }
            | Self::RateLimited { diagnostic_id, .. }
            | Self::SdkInvariant { diagnostic_id }
            | Self::Unknown { diagnostic_id } => diagnostic_id,
        }
    }

    /// True when product autoDiscovery would apply the IGNORE fallback
    /// (`https://{server}` as homeserver base when well-known is absent).
    pub fn allows_well_known_ignore_fallback(&self) -> bool {
        matches!(self, Self::WellKnownNotFound { .. })
    }

    /// True when the Display/debug form of this error contains none of the
    /// given sensitive fragments (used by privacy unit tests).
    pub fn display_is_privacy_safe(&self, forbidden: &[&str]) -> bool {
        let text = self.to_string();
        let lower = text.to_ascii_lowercase();
        !forbidden
            .iter()
            .any(|f| !f.is_empty() && lower.contains(&f.to_ascii_lowercase()))
    }
}
