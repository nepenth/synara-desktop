//! Thin bridge from discovery results into P2.3 client-builder identity/config.
//!
//! Does **not** open a Client, call login, or touch session material. Only
//! rewrites the homeserver URL on an existing [`AccountIdentity`] / produces a
//! homeserver string suitable for `ClientBuildConfig::product_default`.

use super::discovery::DiscoveryResult;
use super::error::AuthError;
use super::input::normalize_homeserver_url;
use crate::app::store::{AccountIdentity, AccountIdentityError};

/// Apply a discovery result's homeserver base URL onto an existing account
/// identity (user id preserved). Validates via [`AccountIdentity::new`].
pub fn identity_with_discovered_homeserver(
    user_id: &str,
    discovery: &DiscoveryResult,
) -> Result<AccountIdentity, AuthError> {
    let hs = normalize_homeserver_url(&discovery.homeserver_base_url)?;
    AccountIdentity::new(user_id, hs.as_str()).map_err(map_identity_error)
}

/// Homeserver URL string suitable for store / client-builder config.
pub fn homeserver_url_for_client_builder(discovery: &DiscoveryResult) -> Result<String, AuthError> {
    Ok(normalize_homeserver_url(&discovery.homeserver_base_url)?.into_string())
}

fn map_identity_error(err: AccountIdentityError) -> AuthError {
    match err {
        AccountIdentityError::EmptyUserId => AuthError::InvalidInput {
            diagnostic_id: "p3.1-identity-empty-user",
            reason: "user id is empty",
        },
        AccountIdentityError::EmptyHomeserver => AuthError::InvalidInput {
            diagnostic_id: "p3.1-identity-empty-homeserver",
            reason: "homeserver url is empty",
        },
        AccountIdentityError::InvalidUserId => AuthError::InvalidInput {
            diagnostic_id: "p3.1-identity-invalid-user",
            reason: "user id is not a valid Matrix user id shape",
        },
        AccountIdentityError::InvalidHomeserver => AuthError::InvalidInput {
            diagnostic_id: "p3.1-identity-invalid-homeserver",
            reason: "homeserver url is invalid for account identity",
        },
    }
}
