//! P2.2 / R0.4 — Per-account Matrix store paths and encryption-key foundation.
//!
//! Re-exports the core store harness (identity, paths, key material, vault
//! trait, store revision migration) and keeps the live OS credential store
//! here in the desktop shell.

pub use synara_core::app::store::*;

mod key_vault;

pub use key_vault::KeyringStoreKeyVault;

#[cfg(test)]
mod tests;
