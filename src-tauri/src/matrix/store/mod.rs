//! P2.2 / R0.4 — Per-account Matrix store paths and encryption-key foundation.
//!
//! Re-exports the core store harness (identity + paths + markers) and keeps the
//! OS credential store / keyring encryption-key foundation here in the desktop
//! shell. Keyring / OS credential store stays desktop; core has no secrets.

#![allow(dead_code)]
#![allow(unused_imports)]

pub use synara_core::app::store::*;

mod key_material;
mod key_vault;

pub use key_material::{StoreKeyId, StoreKeyMaterial, STORE_KEY_LEN, STORE_KEY_SERVICE};
pub use key_vault::{
    get_or_create_store_key, InMemoryStoreKeyVault, KeyringStoreKeyRefs, KeyringStoreKeyVault,
    StoreKeyVault, StoreKeyVaultError,
};

#[cfg(test)]
mod tests;
