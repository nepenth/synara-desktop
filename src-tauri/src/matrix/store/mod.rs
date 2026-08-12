//! P2.2 / R0.4 — Per-account Matrix store paths and encryption-key foundation.
//!
//! Derives isolated store directories from a non-secret account identity and
//! manages store encryption keys through an abstract vault:
//! - [`KeyringStoreKeyVault`] — production OS credential store (macOS/Linux)
//! - [`InMemoryStoreKeyVault`] — unit/integration harness only
//!
//! D0.1 uses these stores for production native password login. There is no
//! production sync and no dual-backend. Store open failures **never**
//! auto-delete on-disk data (plan §8.3).
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p2.2-store-paths-keys.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod identity;
mod key_material;
mod key_vault;
mod paths;
mod revision;

pub use identity::{AccountIdentity, AccountIdentityError};
pub use key_material::{
    StoreKeyId, StoreKeyMaterial, STORE_KEY_LEN, STORE_KEY_REVISION, STORE_KEY_SERVICE,
    STORE_KEY_SERVICE_V1,
};
pub use key_vault::{
    get_or_create_store_key, get_or_migrate_store_key, InMemoryStoreKeyVault, KeyringStoreKeyRefs,
    KeyringStoreKeyVault, StoreKeyVault, StoreKeyVaultError,
};
pub use paths::{
    StoreKeyCreationPolicy, StoreLayout, StorePathError, StorePaths, MATRIX_STORE_ROOT_SEGMENT,
};
pub use revision::{
    matrix_store_revision_marker, migrate_store_to_current, reset_store_for_recovery,
    StoreMigrationError, StoreResetOutcome, StoreRevisionDecision, StoreRevisionManifest,
    STORE_LAYOUT_VERSION, STORE_RECOVERY_ARCHIVE_SEGMENT, STORE_REVISION_MANIFEST_FILE,
};

/// Static marker for link / schema smoke (no network, no Client, no secrets).
pub const MATRIX_STORE_MARKER: &str = "matrix-store-paths-keys-p2.2";

/// Touch store foundation paths so they remain linked in non-test builds.
pub fn matrix_store_markers() -> &'static str {
    let _root = MATRIX_STORE_ROOT_SEGMENT;
    let _key_len = STORE_KEY_LEN;
    debug_assert_eq!(_root, "matrix");
    debug_assert_eq!(_key_len, 32);
    debug_assert_eq!(MATRIX_STORE_MARKER, "matrix-store-paths-keys-p2.2");
    MATRIX_STORE_MARKER
}

#[cfg(test)]
mod tests;
