//! P2.2 / R0.4 — Per-account Matrix store paths and encryption-key foundation.
//!
//! Derives isolated store directories from a non-secret account identity.
//! Store encryption keys remain in the desktop shell (OS credential store /
//! keyring); this core module is identity + path layout + markers only.
//!
//! D0.1 uses these stores for production native password login. There is no
//! production sync and no dual-backend. Store open failures **never**
//! auto-delete on-disk data (plan §8.3).
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p2.2-store-paths-keys.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod identity;
mod paths;

pub use identity::{AccountIdentity, AccountIdentityError};
pub use paths::{StoreLayout, StorePathError, StorePaths, MATRIX_STORE_ROOT_SEGMENT};

/// Static marker for link / schema smoke (no network, no Client, no secrets).
pub const MATRIX_STORE_MARKER: &str = "matrix-store-paths-keys-p2.2";

/// Touch store foundation paths so they remain linked in non-test builds.
pub fn matrix_store_markers() -> &'static str {
    let _root = MATRIX_STORE_ROOT_SEGMENT;
    debug_assert_eq!(_root, "matrix");
    debug_assert_eq!(MATRIX_STORE_MARKER, "matrix-store-paths-keys-p2.2");
    MATRIX_STORE_MARKER
}
