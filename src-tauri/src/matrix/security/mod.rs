//! P8.1 — Security / crypto status projection foundation (harness).
//!
//! Pure projection of Synara [`SecurityStatus`] DTOs. **No keys, recovery
//! secrets, or tokens.** No SDK crypto APIs, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p8.1-security-status.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod status;

pub use error::SecurityError;
pub use status::SecurityStatusStore;

/// Static marker for link / schema smoke.
pub const MATRIX_SECURITY_MARKER: &str = "matrix-security-p8.1";

/// Touch security paths so they remain linked in non-test builds.
pub fn matrix_security_markers() -> &'static str {
    let store = SecurityStatusStore::new(0);
    debug_assert!(!store.needs_attention());
    debug_assert_eq!(store.session_generation(), 0);
    debug_assert_eq!(MATRIX_SECURITY_MARKER, "matrix-security-p8.1");
    MATRIX_SECURITY_MARKER
}

#[cfg(test)]
mod tests;
