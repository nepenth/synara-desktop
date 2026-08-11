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

/// Return true when a JSON-encoded payload length is within policy bounds.
///
/// Contract only — supervisors enforce this at the boundary before enqueue.
/// Oversized payloads must be chunked or transferred out-of-band (handles).
#[inline]
pub fn payload_within_bounds(json_byte_len: usize) -> bool {
    json_byte_len <= MAX_ENVELOPE_PAYLOAD_JSON_BYTES
}

/// Return true when a stream queue depth is within the documented soft bound.
///
/// Depth above `MAX_STREAM_QUEUE_DEPTH` requires backpressure and/or coalescing
/// within `STREAM_COALESCE_WINDOW_MS` (documented policy; no live queue here).
#[inline]
pub fn stream_queue_depth_within_bounds(depth: usize) -> bool {
    depth <= MAX_STREAM_QUEUE_DEPTH
}

/// Return true when the number of open streams is within the documented bound.
#[inline]
pub fn open_streams_within_bounds(count: usize) -> bool {
    count <= MAX_OPEN_STREAMS_PER_SESSION
}
