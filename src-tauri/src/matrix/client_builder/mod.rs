//! P2.3 — Matrix Rust SDK client builder foundation.

pub use synara_core::app::client_builder::*;

use std::path::Path;

use synara_core::app::store::{AccountIdentity, StoreKeyMaterial};

/// `Synara-Desktop/<app version> (matrix-sdk/<pin>)`, with the desktop
/// package version rather than Core's crate version.
pub fn desktop_user_agent() -> String {
    product_user_agent(ClientProduct::Desktop, env!("CARGO_PKG_VERSION"))
}

/// The product client config with the desktop's User-Agent. Every desktop
/// session, sign-in and cleanup client is built from this.
pub fn desktop_client_config(
    app_data_root: &Path,
    identity: AccountIdentity,
    store_key: Option<StoreKeyMaterial>,
) -> Result<ClientBuildConfig, ClientBuilderError> {
    ClientBuildConfig::product_default(app_data_root, identity, store_key)?
        .with_user_agent(desktop_user_agent())
}

#[cfg(test)]
mod tests;
