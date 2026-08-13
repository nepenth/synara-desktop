//! P2.3 — Matrix Rust SDK client builder foundation (error + features surface).
//!
//! Privacy-safe client-build errors and the approved/forbidden Cargo feature
//! pins for the unauthenticated `matrix_sdk::Client` construction path.
//!
//! The construction, config, open, and SDK-handle paths remain in the desktop
//! shell (`src-tauri/src/matrix/client_builder/`); this core module is the
//! error + features surface only. D0.1 uses this builder for production native
//! password-login client; native sync remains out of scope.

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod features;

pub use error::ClientBuilderError;
pub use features::{
    APPROVED_MATRIX_SDK_FEATURES, FORBIDDEN_MATRIX_SDK_FEATURES, MATRIX_SDK_PIN_VERSION,
};
