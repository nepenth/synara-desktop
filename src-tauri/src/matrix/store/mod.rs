//! P2.2 / R0.4 — Per-account Matrix store paths and encryption-key foundation.
//!
//! Re-exports the core store harness (identity, paths, key material, vault
//! trait) and keeps the live OS credential store here in the desktop shell.

#![allow(dead_code)]
#![allow(unused_imports)]

pub use synara_core::app::store::*;

mod key_vault;
mod revision;

pub use key_vault::{KeyringStoreKeyRefs, KeyringStoreKeyVault};
pub use revision::{
    matrix_store_revision_marker, migrate_store_to_current, reset_store_for_recovery,
    StoreMigrationError, StoreResetOutcome, StoreRevisionDecision, StoreRevisionManifest,
    STORE_LAYOUT_VERSION, STORE_RECOVERY_ARCHIVE_SEGMENT, STORE_REVISION_MANIFEST_FILE,
};

#[cfg(test)]
mod tests;
