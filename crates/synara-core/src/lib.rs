//! # synara-core
//!
//! Transport-agnostic shared native core for Synara (desktop via Tauri, iOS via
//! uniffi). Domain modules move here by `git mv` + path updates only, keeping
//! behavior identical (P1 slices: dto, transport/ipc, task, then the app/
//! domain chunks).

// Generated from `src/synara_core.udl` by build.rs. Keep this at crate root:
// P4-2 adds only credential-free login-flow discovery to the P4-1 bootstrap.
uniffi::include_scaffolding!("synara_core");

/// Identifies the project-owned UniFFI surface without exposing a product
/// command, credential, Matrix SDK type, or platform callback prematurely.
/// P4 migration slices grow the UDL only alongside their corresponding core API.
pub fn binding_scaffold_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

mod ffi;
pub use ffi::{login_flows, LoginFlowDto, LoginFlowsError};

mod core;
pub use core::Core;

pub mod app;

pub mod dto;
pub mod platform;

pub mod task;

pub mod transport;
