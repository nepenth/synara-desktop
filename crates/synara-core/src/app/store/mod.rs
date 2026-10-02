//! P2.2 / R0.4 — Per-account Matrix store paths and encryption-key foundation.
//!
//! Derives isolated store directories from a non-secret account identity and
//! owns the store-key vault *trait* plus in-memory harness. Live OS credential
//! I/O (Keychain / Secret Service) stays in the desktop shell.
//!
//! D0.1 uses these stores for production native password login. There is no
//! production sync and no dual-backend. Store open failures **never**
//! auto-delete on-disk data (plan §8.3).
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p2.2-store-paths-keys.md`

mod identity;
mod key_material;
mod paths;
mod vault;

pub use identity::{AccountIdentity, AccountIdentityError};
pub use key_material::{
    StoreKeyGenError, StoreKeyId, StoreKeyMaterial, STORE_KEY_LEN, STORE_KEY_REVISION,
    STORE_KEY_SERVICE, STORE_KEY_SERVICE_V1,
};
pub use paths::{
    StoreKeyCreationPolicy, StoreLayout, StorePathError, StorePaths, MATRIX_STORE_ROOT_SEGMENT,
};
pub use vault::{
    get_or_create_store_key, get_or_migrate_store_key, InMemoryStoreKeyVault, StoreKeyVault,
    StoreKeyVaultError,
};
