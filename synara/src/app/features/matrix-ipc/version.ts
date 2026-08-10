/**
 * Matrix IPC protocol version and policy constants (P1.3).
 * Transport-neutral — not wired into production session bootstrap.
 */

/** Wire protocol version. Bump only with an explicit compatibility plan. */
export const MATRIX_IPC_PROTOCOL_VERSION = 1 as const;

/**
 * Maximum inclusive wire counter for session generations / sequences (R0.3 / REV-004).
 * Frozen to `Number.MAX_SAFE_INTEGER` (2^53 − 1) so Rust u64 values never lose
 * precision when crossing the JavaScript boundary.
 */
export const MAX_WIRE_COUNTER = Number.MAX_SAFE_INTEGER;

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

/** True when a JSON-encoded payload length is within policy bounds. */
export function payloadWithinBounds(jsonByteLen: number): boolean {
  return jsonByteLen <= MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
}

/**
 * True when a stream queue depth is within the documented soft bound.
 * Depth above MAX_STREAM_QUEUE_DEPTH requires backpressure/coalescing
 * within STREAM_COALESCE_WINDOW_MS (documented policy; no live queue here).
 */
export function streamQueueDepthWithinBounds(depth: number): boolean {
  return depth <= MAX_STREAM_QUEUE_DEPTH;
}

/** True when open stream count is within the documented bound. */
export function openStreamsWithinBounds(count: number): boolean {
  return count <= MAX_OPEN_STREAMS_PER_SESSION;
}

/**
 * True when `value` is a non-negative integer in the wire-safe counter range.
 * Rejects NaN, ±Infinity, fractions, negatives, and integers above MAX_WIRE_COUNTER.
 */
export function isWireCounter(value: unknown): value is number {
  return (
    typeof value === 'number' && Number.isInteger(value) && value >= 0 && value <= MAX_WIRE_COUNTER
  );
}

/** Checked successor of a wire counter, or null if it would leave the safe range. */
export function checkedNextWireCounter(last: number): number | null {
  if (!isWireCounter(last)) return null;
  if (last >= MAX_WIRE_COUNTER) return null;
  return last + 1;
}
