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

mod census;
mod command;
mod envelope;
mod error;
mod protocol;
mod registry;
mod stream;
mod stream_body;
mod version;
mod wire_counter;

pub use census::*;
pub use command::*;
pub use envelope::*;
pub use error::*;
pub use protocol::*;
pub use registry::*;
pub use stream::*;
pub use stream_body::{validate_stream_topic_body, RoomListStreamBody, TimelineStreamBody};
pub use version::*;
pub use wire_counter::{checked_next_wire_counter, is_valid_wire_counter, MAX_WIRE_COUNTER};

#[cfg(test)]
mod contract_tests;
#[cfg(test)]
mod tests;
