//! P2.2 / R0.4 — Per-account Matrix store paths and encryption-key foundation.
//!
//! Re-exports the core store harness (identity, paths, key material, vault
//! trait) and keeps the live OS credential store here in the desktop shell.

#![allow(dead_code)]
#![allow(unused_imports)]

pub use synara_core::app::store::*;

mod key_vault;

pub use key_vault::{KeyringStoreKeyRefs, KeyringStoreKeyVault};

#[cfg(test)]
mod tests;
