//! P4.8 — Route / deep-link resolution foundation (harness).
//!
//! Pure parse/build of Synara product paths. No SDK, no production Tauri
//! commands, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p4.8-routes.md`

mod error;
mod resolve;

pub use error::RouteError;
pub use resolve::{build_path, resolve_path, RouteTarget};

#[cfg(test)]
mod tests;
