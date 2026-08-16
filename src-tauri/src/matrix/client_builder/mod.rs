//! P2.3 — Matrix Rust SDK client builder foundation.
#![allow(dead_code)]
#![allow(unused_imports)]

pub use synara_core::app::client_builder::*;

mod sdk_handle;

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
