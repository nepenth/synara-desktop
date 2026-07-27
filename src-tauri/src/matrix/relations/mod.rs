//! P5.6 — Relations index foundation (harness).
//!
//! Pure projection of Synara [`RelationRef`] DTOs (annotations / replaces /
//! references / threads). No SDK send, no production Tauri commands, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p5.6-relations.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod index;

pub use error::RelationError;
pub use index::RelationIndex;

/// Static marker for link / schema smoke.
pub const MATRIX_RELATIONS_MARKER: &str = "matrix-relations-p5.6";

/// Touch relation paths so they remain linked in non-test builds.
pub fn matrix_relations_markers() -> &'static str {
    let idx = RelationIndex::new(0);
    debug_assert!(idx.is_empty());
    debug_assert_eq!(idx.annotation_count(), 0);
    debug_assert_eq!(MATRIX_RELATIONS_MARKER, "matrix-relations-p5.6");
    MATRIX_RELATIONS_MARKER
}

#[cfg(test)]
mod tests;
