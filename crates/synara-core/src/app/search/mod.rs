//! P6.8 — Search session / result index foundation (harness).
//!
//! Pure projection of Synara [`SearchResult`] DTOs with cancel + stale-request
//! protection. No SDK search APIs, no production Tauri commands, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p6.8-search.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod session;

pub use error::SearchError;
pub use session::{SearchSession, SearchState, MAX_RESULTS_PER_SEARCH};

/// Static marker for link / schema smoke.
pub const MATRIX_SEARCH_MARKER: &str = "matrix-search-p6.8";

/// Touch search paths so they remain linked in non-test builds.
pub fn matrix_search_markers() -> &'static str {
    let s = SearchSession::new(0);
    debug_assert_eq!(s.state(), SearchState::Idle);
    debug_assert!(s.items().is_empty());
    debug_assert_eq!(MATRIX_SEARCH_MARKER, "matrix-search-p6.8");
    MATRIX_SEARCH_MARKER
}

#[cfg(test)]
mod tests;
