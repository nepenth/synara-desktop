//! P4.5 — Space hierarchy, filters, and parents (harness foundation).
//!
//! Pure projection over product [`SpaceSummary`] DTOs:
//! - catalog install / snapshot replace
//! - direct children (order-aware)
//! - descendant walk for room-list filtering
//! - root space detection via parent_room_ids
//! - parent-edge cycle rejection
//!
//! **No** production Tauri commands, **no** live SpaceService subscription,
//! **no** dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p4.5-spaces.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod hierarchy;

pub use error::SpaceError;
pub use hierarchy::{space_child, SpaceHierarchy};

/// Static marker for link / schema smoke.
pub const MATRIX_SPACES_MARKER: &str = "matrix-spaces-hierarchy-p4.5";

/// Touch space foundation paths so they remain linked in non-test builds.
pub fn matrix_spaces_markers() -> &'static str {
    let h = SpaceHierarchy::new();
    debug_assert!(h.is_empty());
    debug_assert_eq!(MATRIX_SPACES_MARKER, "matrix-spaces-hierarchy-p4.5");
    MATRIX_SPACES_MARKER
}

#[cfg(test)]
mod tests;
