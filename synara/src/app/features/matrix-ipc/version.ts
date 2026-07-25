/**
 * Matrix IPC protocol version and policy constants (P1.3).
 * Transport-neutral — not wired into production session bootstrap.
 */

/** Wire protocol version. Bump only with an explicit compatibility plan. */
export const MATRIX_IPC_PROTOCOL_VERSION = 1 as const;

/** Soft upper bound for a single JSON-encoded envelope payload (bytes). */
export const MAX_ENVELOPE_PAYLOAD_JSON_BYTES = 1_048_576; // 1 MiB

/** Maximum pending messages retained per stream before backpressure/coalesce. */
export const MAX_STREAM_QUEUE_DEPTH = 256;

/** Default coalesce window for high-frequency stream deltas (milliseconds). */
export const STREAM_COALESCE_WINDOW_MS = 16;

/** Maximum concurrent open Matrix IPC streams per session generation. */
export const MAX_OPEN_STREAMS_PER_SESSION = 64;

/**
 * Large media bytes must never be serialized through JSON IPC.
 * Use binary handles / chunked native transfer APIs instead.
 */
export const FORBID_MEDIA_BYTES_OVER_JSON_IPC = true;
