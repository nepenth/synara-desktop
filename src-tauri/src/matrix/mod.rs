//! Matrix integration surface for the production Tauri crate.
//!
//! P1.3: versioned IPC schema foundation only. No production Client session,
//! login, sync, or Tauri command registration lives here yet.

pub mod ipc;

// Keep IPC schema symbols resolved in non-test builds (avoids dead-strip of the
// foundation module until production consumers land in later phases).
const _: fn() -> &'static str = matrix_ipc_schema_markers;

/// Touch Matrix IPC schema paths so the foundation remains linked.
/// Returns a static marker only — no Client, network, or Tauri commands.
pub fn matrix_ipc_schema_markers() -> &'static str {
    let _version = ipc::MATRIX_IPC_PROTOCOL_VERSION;
    let _kinds = ipc::MATRIX_IPC_KINDS.len();
    let _errors = ipc::MatrixIpcErrorCategory::ALL.len();
    let _topics = ipc::StreamTopic::ALL.len();
    let _forbid_media = ipc::FORBID_MEDIA_BYTES_OVER_JSON_IPC;
    let _queue = ipc::MAX_STREAM_QUEUE_DEPTH;
    debug_assert_eq!(_version, 1);
    debug_assert!(_kinds > 0);
    debug_assert!(_errors > 0);
    debug_assert!(_topics > 0);
    debug_assert!(_forbid_media);
    debug_assert!(_queue > 0);
    "matrix-ipc-protocol-v1 schema foundation"
}
