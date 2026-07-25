//! Versioned Matrix IPC protocol foundation (P1.3).
//!
//! Schema/contract only: envelopes, stream lifecycle, error categories, and
//! pure protocol helpers. No `matrix_sdk` types, no live supervisor, no Tauri
//! production commands.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p1.3-matrix-ipc-schemas.md`
//! Shared fixtures: `docs/matrix-rust-sdk/ipc/fixtures/`
//!
//! Dead-code allowances: this module is intentionally not wired into production
//! command handlers yet (P1.4+). Types are part of the public crate surface for
//! later phases and are exercised by unit tests + schema markers.

#![allow(dead_code)]
#![allow(unused_imports)]

mod envelope;
mod error;
mod protocol;
mod stream;
mod version;

pub use envelope::*;
pub use error::*;
pub use protocol::*;
pub use stream::*;
pub use version::*;

#[cfg(test)]
mod tests;
