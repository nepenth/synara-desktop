//! P2.3 — Matrix Rust SDK client builder foundation (error + features + config).
//!
//! Privacy-safe client-build errors, approved/forbidden Cargo feature pins,
//! and the live [`build_unauthenticated_client`] constructor. The desktop
//! shell keeps SdkClientHandle wiring only.

#![allow(dead_code)]
#![allow(unused_imports)]

mod config;
mod error;
mod features;
mod open;

pub use config::{
    default_user_agent, ClientBuildConfig, ClientBuildPlan, HomeserverMode, NetworkPolicy,
    TimeoutPolicy, DEFAULT_REQUEST_TIMEOUT_SECS, DEFAULT_RETRY_LIMIT,
};
pub use error::ClientBuilderError;
pub use features::{
    APPROVED_MATRIX_SDK_FEATURES, FORBIDDEN_MATRIX_SDK_FEATURES, MATRIX_SDK_PIN_VERSION,
};
pub use open::build_unauthenticated_client;
