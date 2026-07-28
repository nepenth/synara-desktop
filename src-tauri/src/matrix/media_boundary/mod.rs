//! P7.7 — Matrix media ownership boundary (foundation guard).
//!
//! The native host is the sole allowed owner of Matrix authentication and
//! sensitive media capabilities. A service worker must not retain Matrix
//! credentials, decrypt encrypted media, or keep long-lived MXC caches that
//! contain secrets.
//!
//! This module is a pure policy inventory. It does not delete the existing
//! service worker, change the active Matrix backend, or perform media I/O.
//!
//! Authoritative design note:
//! `docs/matrix-rust-sdk/p7.7-sw-media-boundary.md`

#![allow(dead_code)]

/// Stable marker for link / schema smoke.
pub const MATRIX_MEDIA_BOUNDARY_MARKER: &str = "matrix-media-boundary-p7.7";

/// Capability name for retaining Matrix authentication material.
pub const SW_TOKEN_STORAGE: &str = "service-worker-token-storage";

/// Capability name for decrypting encrypted Matrix media.
pub const SW_ENCRYPTED_MEDIA_DECRYPT: &str = "service-worker-encrypted-media-decrypt";

/// Capability name for a long-lived MXC cache that contains secrets.
pub const SW_LONG_LIVED_MXC_SECRET_CACHE: &str = "service-worker-long-lived-mxc-cache-of-secrets";

/// Runtime that owns a sensitive Matrix media capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaOwner {
    /// Native Rust host process.
    HostNative,
    /// Explicit sentinel for ownership that must never be granted.
    ServiceWorkerForbidden,
}

/// One stable entry in the Matrix media ownership inventory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoundaryRule {
    pub capability: &'static str,
    pub allowed_owner: MediaOwner,
}

/// Privacy-safe ownership assertion failure.
///
/// Variants deliberately carry no caller-provided capability string.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundaryError {
    UnknownCapability,
    ForbiddenOwner,
}

const DEFAULT_RULES: [BoundaryRule; 3] = [
    BoundaryRule {
        capability: SW_TOKEN_STORAGE,
        allowed_owner: MediaOwner::HostNative,
    },
    BoundaryRule {
        capability: SW_ENCRYPTED_MEDIA_DECRYPT,
        allowed_owner: MediaOwner::HostNative,
    },
    BoundaryRule {
        capability: SW_LONG_LIVED_MXC_SECRET_CACHE,
        allowed_owner: MediaOwner::HostNative,
    },
];

/// Returns the closed inventory of sensitive Matrix capabilities.
pub fn default_rules() -> &'static [BoundaryRule] {
    &DEFAULT_RULES
}

/// Asserts that `owner` is allowed to hold the named capability.
///
/// Unknown capabilities are rejected so additions require an explicit policy
/// decision rather than silently gaining an owner.
pub fn assert_owner(capability: &str, owner: MediaOwner) -> Result<(), BoundaryError> {
    let rule = default_rules()
        .iter()
        .find(|rule| rule.capability == capability)
        .ok_or(BoundaryError::UnknownCapability)?;

    if rule.allowed_owner == owner {
        Ok(())
    } else {
        Err(BoundaryError::ForbiddenOwner)
    }
}

/// Touch policy paths so they remain linked in non-test builds.
pub fn matrix_media_boundary_markers() -> &'static str {
    debug_assert_eq!(default_rules().len(), 3);
    debug_assert!(default_rules()
        .iter()
        .all(|rule| rule.allowed_owner == MediaOwner::HostNative));
    debug_assert_eq!(MATRIX_MEDIA_BOUNDARY_MARKER, "matrix-media-boundary-p7.7");
    MATRIX_MEDIA_BOUNDARY_MARKER
}

#[cfg(test)]
mod tests;
