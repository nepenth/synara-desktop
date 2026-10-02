//! P8.8 — Crypto-store continuity and corruption handling (harness).
//!
//! Tracks open/health/continuity of the encrypted crypto store. **Never
//! auto-wipes. Never stores keys.** Complements P2.2 paths + P2.6 recovery.
//! No dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p8.8-crypto-store.md`

mod continuity;
mod error;

pub use continuity::{
    CryptoStoreAction, CryptoStoreContinuity, CryptoStoreHealth, CryptoStorePhase,
};
pub use error::CryptoStoreError;

#[cfg(test)]
mod tests;
