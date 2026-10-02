//! P5.6 — Relations index foundation (harness).
//!
//! Pure projection of Synara [`RelationRef`] DTOs (annotations / replaces /
//! references / threads). No SDK send, no production Tauri commands, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p5.6-relations.md`

mod error;
mod index;

pub use error::RelationError;
pub use index::RelationIndex;

#[cfg(test)]
mod tests;
