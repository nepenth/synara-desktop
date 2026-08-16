//! P8.8 — Crypto-store continuity and corruption handling (harness).
//!
//! Tracks open/health/continuity of the encrypted crypto store. **Never
//! auto-wipes. Never stores keys.** Complements P2.2 paths + P2.6 recovery.
//! No dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p8.8-crypto-store.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod continuity;
mod error;

pub use continuity::{
    CryptoStoreAction, CryptoStoreContinuity, CryptoStoreHealth, CryptoStorePhase,
};
pub use error::CryptoStoreError;

/// Static marker for link / schema smoke.
pub const MATRIX_CRYPTO_STORE_MARKER: &str = "matrix-crypto-store-p8.8";

/// Touch crypto-store continuity paths so they remain linked in non-test builds.
pub fn matrix_crypto_store_markers() -> &'static str {
    let c = CryptoStoreContinuity::new(0);
    debug_assert_eq!(c.phase(), CryptoStorePhase::Closed);
    debug_assert!(c.never_auto_wipes());
    debug_assert_eq!(CryptoStoreHealth::Healthy.as_str(), "healthy");
    debug_assert_eq!(MATRIX_CRYPTO_STORE_MARKER, "matrix-crypto-store-p8.8");
    MATRIX_CRYPTO_STORE_MARKER
}

#[cfg(test)]
mod tests;
