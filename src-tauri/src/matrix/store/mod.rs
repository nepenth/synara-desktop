//! P2.2 / R0.4 — Per-account Matrix store paths and encryption-key foundation.
//!
//! Re-exports the core store harness (identity, paths, key material, vault
//! trait) and keeps the live OS credential store here in the desktop shell.

pub use synara_core::app::store::*;

mod key_vault;
mod revision;

pub use key_vault::KeyringStoreKeyVault;

pub use revision::{migrate_store_to_current, reset_store_for_recovery, StoreMigrationError};

#[cfg(test)]
pub use revision::STORE_RECOVERY_ARCHIVE_SEGMENT;

#[cfg(test)]
mod tests;
