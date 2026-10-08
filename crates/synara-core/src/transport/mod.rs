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

#[cfg(feature = "full-app")]
mod census;
#[cfg(feature = "full-app")]
mod command;
#[cfg(feature = "full-app")]
mod envelope;
mod error;
#[cfg(feature = "full-app")]
mod protocol;
#[cfg(all(test, feature = "full-app"))]
mod registry;
#[cfg(feature = "full-app")]
mod stream;
#[cfg(feature = "full-app")]
mod stream_body;
#[cfg(feature = "full-app")]
mod version;
#[cfg(feature = "full-app")]
mod wire_counter;

#[cfg(feature = "full-app")]
pub use census::*;
#[cfg(feature = "full-app")]
pub use command::*;
#[cfg(feature = "full-app")]
pub use envelope::*;
pub use error::*;
#[cfg(feature = "full-app")]
pub use protocol::*;
#[cfg(all(test, feature = "full-app"))]
pub use registry::*;
#[cfg(feature = "full-app")]
pub use stream::*;
#[cfg(feature = "full-app")]
pub use stream_body::{validate_stream_topic_body, RoomListStreamBody, TimelineStreamBody};
#[cfg(feature = "full-app")]
pub use version::*;
#[cfg(feature = "full-app")]
pub use wire_counter::{checked_next_wire_counter, is_valid_wire_counter, MAX_WIRE_COUNTER};

#[cfg(all(test, feature = "full-app"))]
mod contract_tests;
#[cfg(all(test, feature = "full-app"))]
mod tests;
