//! P4.8 — Route / deep-link resolution foundation (harness).
//!
//! Pure parse/build of Synara product paths. No SDK, no production Tauri
//! commands, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p4.8-routes.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod resolve;

pub use error::RouteError;
pub use resolve::{build_path, resolve_path, RouteTarget};

/// Static marker for link / schema smoke.
pub const MATRIX_ROUTES_MARKER: &str = "matrix-routes-p4.8";

/// Touch route paths so they remain linked in non-test builds.
pub fn matrix_routes_markers() -> &'static str {
    let home = resolve_path("/home").expect("home route");
    debug_assert_eq!(home, RouteTarget::Home);
    debug_assert_eq!(build_path(&RouteTarget::Home).expect("build home"), "/home");
    debug_assert_eq!(MATRIX_ROUTES_MARKER, "matrix-routes-p4.8");
    MATRIX_ROUTES_MARKER
}

#[cfg(test)]
mod tests;
