//! Matrix integration surface for the production Tauri crate.
//!
//! P1.3: versioned IPC schema foundation.
//! P1.4: Synara-owned domain DTOs (product projections for IPC bodies).
//! P2.1: Matrix supervisor actor foundation (pure lifecycle + generation).
//! No production Client session, login, sync, or Tauri command registration
//! lives here yet. No dual-backend selector.

pub mod dto;
pub mod ipc;
pub mod supervisor;

// Keep schema/DTO/supervisor symbols resolved in non-test builds (avoids
// dead-strip until production consumers land in later phases).
const _: fn() -> &'static str = matrix_ipc_schema_markers;

/// Touch Matrix IPC + domain DTO + supervisor paths so foundations remain linked.
/// Returns a static marker only — no Client builder, network, or Tauri commands.
pub fn matrix_ipc_schema_markers() -> &'static str {
    let _version = ipc::MATRIX_IPC_PROTOCOL_VERSION;
    let _kinds = ipc::MATRIX_IPC_KINDS.len();
    let _errors = ipc::MatrixIpcErrorCategory::ALL.len();
    let _topics = ipc::StreamTopic::ALL.len();
    let _forbid_media = ipc::FORBID_MEDIA_BYTES_OVER_JSON_IPC;
    let _queue = ipc::MAX_STREAM_QUEUE_DEPTH;
    let _dto = dto::MATRIX_DTO_MARKER;
    let _dto_forbid = dto::FORBID_MEDIA_BYTES_OVER_JSON_IPC;
    let _lifecycles = dto::SessionLifecycle::ALL.len();
    let _memberships = dto::Membership::ALL.len();
    let _supervisor = supervisor::matrix_supervisor_markers();
    debug_assert_eq!(_version, 1);
    debug_assert!(_kinds > 0);
    debug_assert!(_errors > 0);
    debug_assert!(_topics > 0);
    debug_assert!(_forbid_media);
    debug_assert!(_queue > 0);
    debug_assert!(_dto_forbid);
    debug_assert!(_lifecycles > 0);
    debug_assert!(_memberships > 0);
    debug_assert_eq!(_dto, "matrix-domain-dtos-p1.4");
    debug_assert_eq!(_supervisor, supervisor::MATRIX_SUPERVISOR_MARKER);
    "matrix-ipc-protocol-v1+domain-dtos-p1.4+supervisor-p2.1"
}
