//! P2.3 — Matrix Rust SDK client builder foundation (error + features + config).
//!
//! Privacy-safe client-build errors, approved/forbidden Cargo feature pins,
//! and the live [`build_unauthenticated_client`] constructor. The desktop
//! shell projects its errors into the native auth command contract.

mod config;
mod error;
mod features;
mod open;

pub use config::{
    default_user_agent, ClientBuildConfig, ClientBuildPlan, HomeserverMode, NetworkPolicy,
    TimeoutPolicy, DEFAULT_REQUEST_TIMEOUT_SECS, DEFAULT_RETRY_LIMIT,
};
pub use error::{ClientBuilderError, FactoryError};
pub use features::{
    forbidden_requested_features, requested_cargo_features, APPROVED_MATRIX_SDK_FEATURES,
    FORBIDDEN_MATRIX_SDK_FEATURES, MATRIX_SDK_PIN_VERSION,
};
pub use open::{build_memory_only_client, build_unauthenticated_client};
