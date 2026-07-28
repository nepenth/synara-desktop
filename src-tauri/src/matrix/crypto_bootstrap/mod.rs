//! P8.9 — Crypto bootstrap readiness coordinator (harness).
//!
//! Post-login checklist for dogfood sole-owner flip. No keys, recovery
//! secrets, or tokens. No SDK crypto APIs, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p8.9-crypto-bootstrap.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod coordinator;
mod error;

pub use coordinator::{
    BootstrapPhase, BootstrapStep, CryptoBootstrapCoordinator, MAX_PENDING_STEPS,
    MAX_STEP_LABEL_CHARS,
};
pub use error::CryptoBootstrapError;

/// Static marker for link / schema smoke.
pub const MATRIX_CRYPTO_BOOTSTRAP_MARKER: &str = "matrix-crypto-bootstrap-p8.9";

/// Touch crypto-bootstrap paths so they remain linked in non-test builds.
pub fn matrix_crypto_bootstrap_markers() -> &'static str {
    let c = CryptoBootstrapCoordinator::new(0);
    debug_assert_eq!(c.phase(), BootstrapPhase::Idle);
    debug_assert!(!c.is_dogfood_ready());
    debug_assert_eq!(
        MATRIX_CRYPTO_BOOTSTRAP_MARKER,
        "matrix-crypto-bootstrap-p8.9"
    );
    MATRIX_CRYPTO_BOOTSTRAP_MARKER
}

#[cfg(test)]
mod tests;
