//! P2.3 — Matrix Rust SDK client builder foundation.
//!
//! Configures and constructs an **unauthenticated** `matrix_sdk::Client` with:
//! homeserver URL, proxy/network policy, user agent, SQLite stores, crypto
//! settings, request timeouts, and approved Cargo features.
//!
//! D0.1 uses this builder for the production native password-login client.
//! Native sync remains out of scope and there is no dual-backend selector.
//! This module is the **only** production construction site for
//! `Client::builder` under `src-tauri/src/matrix/`.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p2.3-sdk-client-builder.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod config;
mod error;
mod features;
mod open;
mod sdk_handle;

pub use config::{
    default_user_agent, ClientBuildConfig, ClientBuildPlan, HomeserverMode, NetworkPolicy,
    TimeoutPolicy, DEFAULT_REQUEST_TIMEOUT_SECS, DEFAULT_RETRY_LIMIT,
};
pub use error::ClientBuilderError;
pub use features::{
    APPROVED_MATRIX_SDK_FEATURES, FORBIDDEN_MATRIX_SDK_FEATURES, MATRIX_SDK_PIN_VERSION,
};
pub use open::build_unauthenticated_client;
pub use sdk_handle::SdkClientHandle;

/// Static marker for link / schema smoke.
pub const MATRIX_CLIENT_BUILDER_MARKER: &str = "matrix-sdk-client-builder-p2.3";

/// Touch client-builder paths so the foundation remains linked in non-test builds.
pub fn matrix_client_builder_markers() -> &'static str {
    let _features = APPROVED_MATRIX_SDK_FEATURES.len();
    let _forbidden = FORBIDDEN_MATRIX_SDK_FEATURES.len();
    let _ua = default_user_agent();
    let _timeout = DEFAULT_REQUEST_TIMEOUT_SECS;
    let _retry = DEFAULT_RETRY_LIMIT;
    debug_assert!(_features > 0);
    debug_assert!(_forbidden > 0);
    debug_assert!(!_ua.is_empty());
    debug_assert!(_timeout > 0);
    debug_assert_eq!(MATRIX_SDK_PIN_VERSION, "0.18.0");
    debug_assert_eq!(
        MATRIX_CLIENT_BUILDER_MARKER,
        "matrix-sdk-client-builder-p2.3"
    );
    MATRIX_CLIENT_BUILDER_MARKER
}

#[cfg(test)]
mod tests;
