//! SQLite store builder surface probes (`sqlite` feature).
//!
//! Profile: `profile-sqlite`.
//! Compile-only; never opens a database or uses a passphrase at runtime.
//! Passphrase parameters appear only in inert type signatures.

use std::path::Path;

use matrix_sdk::{ClientBuilder, SqliteStoreConfig};

/// Probe IDs compiled under `profile-sqlite`.
pub const PROBE_IDS: &[&str] = &[
    "P0.3c-client-builder-sqlite-store",
    "P0.3c-client-builder-sqlite-store-with-cache-path",
    "P0.3c-client-builder-sqlite-store-with-config",
];

/// P0.3c-client-builder-sqlite-store
///
/// Source: `crates/matrix-sdk/src/client/builder/mod.rs` L255.
/// Signature accepts an optional passphrase; this probe never supplies secrets
/// or opens a store.
pub fn probe_client_builder_sqlite_store() {
    fn _shape(builder: ClientBuilder, path: &Path, passphrase: Option<&str>) -> ClientBuilder {
        builder.sqlite_store(path, passphrase)
    }
    let _ = _shape;
}

/// P0.3c-client-builder-sqlite-store-with-cache-path
pub fn probe_client_builder_sqlite_store_with_cache_path() {
    fn _shape(
        builder: ClientBuilder,
        path: &Path,
        cache_path: &Path,
        passphrase: Option<&str>,
    ) -> ClientBuilder {
        builder.sqlite_store_with_cache_path(path, cache_path, passphrase)
    }
    let _ = _shape;
}

/// P0.3c-client-builder-sqlite-store-with-config
///
/// Full config path using `SqliteStoreConfig` when the feature is enabled.
pub fn probe_client_builder_sqlite_store_with_config() {
    fn _shape(
        builder: ClientBuilder,
        config: SqliteStoreConfig,
        cache_path: Option<&Path>,
    ) -> ClientBuilder {
        builder.sqlite_store_with_config_and_cache_path(config, cache_path)
    }
    let _ = _shape;
}

/// Run every sqlite probe (compile-only; no store open).
pub fn run_all() {
    probe_client_builder_sqlite_store();
    probe_client_builder_sqlite_store_with_cache_path();
    probe_client_builder_sqlite_store_with_config();
}
