//! P5.7 — Poll and room state/membership projection foundation (harness).
//!
//! Pure, generation-stamped indexes of poll summary rows and simple room state
//! summary rows. No SDK timeline wiring, send, production Tauri commands, or
//! dual-backend behavior.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p5.7-poll-state-projection.md`

mod error;
mod index;
mod model;

pub use error::ProjectionError;
pub use index::{PollIndex, StateProjectionIndex};
pub use model::{PollProjection, StateProjectionKind, StateProjectionRow};

#[cfg(test)]
mod tests;
