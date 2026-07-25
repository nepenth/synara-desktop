//! Protocol version and hard policy constants for Matrix IPC.

/// Wire protocol version. Bump only with an explicit compatibility plan.
pub const MATRIX_IPC_PROTOCOL_VERSION: u32 = 1;

/// Soft upper bound for a single JSON-encoded envelope payload (bytes).
/// Larger domain bodies must use chunking or out-of-band handles (P1.4+).
pub const MAX_ENVELOPE_PAYLOAD_JSON_BYTES: usize = 1_048_576; // 1 MiB

/// Maximum pending messages retained per stream before backpressure/coalesce.
pub const MAX_STREAM_QUEUE_DEPTH: usize = 256;

/// Default coalesce window for high-frequency stream deltas (milliseconds).
/// Implementations may coalesce compatible deltas within this window.
pub const STREAM_COALESCE_WINDOW_MS: u64 = 16;

/// Maximum concurrent open Matrix IPC streams per session generation.
pub const MAX_OPEN_STREAMS_PER_SESSION: usize = 64;

/// Large media bytes must never be serialized through JSON IPC.
/// Use binary handles / chunked native transfer APIs instead.
pub const FORBID_MEDIA_BYTES_OVER_JSON_IPC: bool = true;
