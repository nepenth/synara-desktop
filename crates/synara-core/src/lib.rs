//! # synara-core
//!
//! Transport-agnostic shared native core for Synara (desktop via Tauri, iOS via
//! uniffi). Domain modules move here by `git mv` + path updates only, keeping
//! behavior identical (P1 slices: dto, transport/ipc, task, then the app/
//! domain chunks).

// Generated from `src/synara_core.udl` by build.rs. Keep this at crate root:
// P4-3 adds only a safe Core session-projection mirror to the credential-free
// P4-2 login-flow surface.
uniffi::include_scaffolding!("synara_core");

/// Identifies the project-owned UniFFI surface without exposing a product
/// command, credential, Matrix SDK type, or platform callback prematurely.
/// P4 migration slices grow the UDL only alongside their corresponding core API.
pub fn binding_scaffold_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

mod ffi;
pub use ffi::{
    login_flows, register_flows, LoginFlowDto, LoginFlowsError, RegisterFlowsDto,
    RegisterFlowsError, RegisterFlowsStatus, RegisterUiaFlowDto,
};

mod session_projection_ffi;
pub use session_projection_ffi::{
    SessionProjection, SessionProjectionCore, SessionProjectionError, SessionProjectionLifecycle,
};

mod shared_core_ffi;
pub use shared_core_ffi::{
    IosSecretVault, IosSecretVaultError, SessionLoginDto, SessionLoginError, SessionRestoreDto,
    SessionRestoreError, SharedCore,
};

mod core;
pub use core::Core;

pub mod app;
pub use app::room_list::{
    room_activity_recovery_required, room_unread_presentation, RoomActivityPreviousState,
    RoomUnreadMembership, RoomUnreadPresentationDto,
};

pub mod dto;
pub mod platform;

pub mod task;

pub mod transport;
