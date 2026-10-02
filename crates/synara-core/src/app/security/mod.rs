//! P8.1 — Security / crypto status projection foundation (harness).
//!
//! Pure projection of Synara [`SecurityStatus`] DTOs. **No keys, recovery
//! secrets, or tokens.** No SDK crypto APIs, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p8.1-security-status.md`

mod error;
mod status;

pub use error::SecurityError;
pub use status::SecurityStatusStore;

#[cfg(test)]
mod tests;
