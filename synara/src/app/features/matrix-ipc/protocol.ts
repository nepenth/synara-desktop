/**
 * Pure Matrix IPC protocol helpers (P1.3 + P1.5 bounds).
 * No transport, no Tauri invoke, no matrix-js-sdk.
 */

import type { MatrixIpcError } from './error';
import type { ResyncRequiredPayload, StreamLifecycleState } from './stream';
import {
  MATRIX_IPC_PROTOCOL_VERSION,
  MAX_ENVELOPE_PAYLOAD_JSON_BYTES,
  MAX_OPEN_STREAMS_PER_SESSION,
  MAX_STREAM_QUEUE_DEPTH,
  checkedNextWireCounter,
  isWireCounter,
} from './version';

export type SequenceOutcome =
  | { type: 'accept'; nextLastApplied: number }
  | { type: 'duplicate'; lastApplied: number }
  | { type: 'gap'; lastApplied: number; observed: number }
  | { type: 'behind'; lastApplied: number; observed: number };

/**
 * Check an incoming sequence against the last applied sequence on a stream.
 *
 * R0.3 / REV-004:
 * - Counters must be wire-safe integers (`isWireCounter`); out-of-range → gap.
 * - After a snapshot is applied, lastApplied is the snapshot sequence.
 * - Deltas must arrive as checked last+1 (never overflow past MAX_WIRE_COUNTER).
 * - Exact equal sequence → duplicate (idempotent ignore).
 * - Greater than expected next → gap → emit resync_required.
 */
export function checkSequence(
  lastApplied: number | null | undefined,
  incoming: number,
): SequenceOutcome {
  if (!isWireCounter(incoming)) {
    return {
      type: 'gap',
      lastApplied: isWireCounter(lastApplied) ? lastApplied : 0,
      observed: typeof incoming === 'number' ? incoming : Number.NaN,
    };
  }
  if (lastApplied === null || lastApplied === undefined) {
    return { type: 'accept', nextLastApplied: incoming };
  }
  if (!isWireCounter(lastApplied)) {
    return { type: 'gap', lastApplied, observed: incoming };
  }
  if (incoming === lastApplied) {
    return { type: 'duplicate', lastApplied };
  }
  const expected = checkedNextWireCounter(lastApplied);
  if (expected === null) {
    // last is already MAX_WIRE_COUNTER: only exact duplicate is safe.
    return { type: 'gap', lastApplied, observed: incoming };
  }
  if (incoming === expected) {
    return { type: 'accept', nextLastApplied: incoming };
  }
  if (incoming > expected) {
    return { type: 'gap', lastApplied, observed: incoming };
  }
  return { type: 'behind', lastApplied, observed: incoming };
}

export function checkSessionGeneration(
  liveGeneration: number,
  envelopeGeneration: number,
): MatrixIpcError | null {
  if (liveGeneration === envelopeGeneration) return null;
  return {
    category: 'stale_session_generation',
    diagnosticId: `stale_gen:live=${liveGeneration}:msg=${envelopeGeneration}`,
  };
}

export function checkProtocolVersion(protocolVersion: number): MatrixIpcError | null {
  if (protocolVersion === MATRIX_IPC_PROTOCOL_VERSION) return null;
  return {
    category: 'unsupported_capability',
    diagnosticId: `protocol_version:got=${protocolVersion}:want=${MATRIX_IPC_PROTOCOL_VERSION}`,
  };
}

export function resyncPayloadForGap(
  streamId: string,
  lastApplied: number,
  observed: number,
): ResyncRequiredPayload {
  return {
    streamId,
    reason: 'sequence_gap',
    lastAppliedSequence: lastApplied,
    observedSequence: observed,
  };
}

export function resyncPayloadForStaleGeneration(streamId?: string): ResyncRequiredPayload {
  return {
    streamId,
    reason: 'stale_session_generation',
  };
}

export type StreamLifecycleEvent =
  | 'subscribe_requested'
  | 'subscribed_ack'
  | 'snapshot_applied'
  | 'delta_applied'
  | 'duplicate_delta'
  | 'resync_needed'
  | 'unsubscribe_requested'
  | 'unsubscribed_ack'
  | 'resources_released'
  | 'failed';

/**
 * Pure lifecycle transition helper (no live supervisor).
 * Returns null if the transition is not allowed.
 */
export function transitionStreamLifecycle(
  current: StreamLifecycleState,
  event: StreamLifecycleEvent,
): StreamLifecycleState | null {
  const key = `${current}|${event}`;
  const table: Record<string, StreamLifecycleState> = {
    'idle|subscribe_requested': 'subscribing',
    'subscribing|subscribed_ack': 'snapshot_pending',
    'subscribing|failed': 'failed',
    'snapshot_pending|snapshot_applied': 'live',
    'snapshot_pending|resync_needed': 'resync_required',
    'snapshot_pending|failed': 'failed',
    'live|delta_applied': 'live',
    'live|duplicate_delta': 'live',
    'live|resync_needed': 'resync_required',
    'live|unsubscribe_requested': 'unsubscribing',
    'live|failed': 'failed',
    'resync_required|subscribe_requested': 'subscribing',
    'resync_required|unsubscribe_requested': 'unsubscribing',
    'unsubscribing|unsubscribed_ack': 'closed',
    'unsubscribing|resources_released': 'closed',
    'failed|subscribe_requested': 'subscribing',
    'closed|subscribe_requested': 'subscribing',
    'closed|resources_released': 'closed',
    'failed|resources_released': 'failed',
  };
  return table[key] ?? null;
}

export function applyDeltaSequence(
  lastApplied: number | null | undefined,
  incoming: number,
): { outcome: SequenceOutcome; event: StreamLifecycleEvent } {
  const outcome = checkSequence(lastApplied, incoming);
  let event: StreamLifecycleEvent;
  switch (outcome.type) {
    case 'accept':
      event = 'delta_applied';
      break;
    case 'duplicate':
      event = 'duplicate_delta';
      break;
    case 'gap':
    case 'behind':
      event = 'resync_needed';
      break;
    default: {
      const _e: never = outcome;
      throw new Error(`unreachable: ${_e}`);
    }
  }
  return { outcome, event };
}

/**
 * Reject JSON envelope payload bodies that exceed the soft size bound.
 * Oversized bodies must use chunking / out-of-band handles.
 */
export function checkPayloadJsonBounds(byteLen: number): MatrixIpcError | null {
  if (byteLen <= MAX_ENVELOPE_PAYLOAD_JSON_BYTES) return null;
  return {
    category: 'sdk_invariant',
    diagnosticId: `payload_too_large:got=${byteLen}:max=${MAX_ENVELOPE_PAYLOAD_JSON_BYTES}`,
  };
}

/** Reject stream queues that would exceed the retained-depth bound. */
export function checkStreamQueueDepth(depth: number): MatrixIpcError | null {
  if (depth <= MAX_STREAM_QUEUE_DEPTH) return null;
  return {
    category: 'sdk_invariant',
    diagnosticId: `stream_queue_depth:got=${depth}:max=${MAX_STREAM_QUEUE_DEPTH}`,
  };
}

/** Reject opening more concurrent streams than the per-session bound allows. */
export function checkOpenStreams(count: number): MatrixIpcError | null {
  if (count <= MAX_OPEN_STREAMS_PER_SESSION) return null;
  return {
    category: 'sdk_invariant',
    diagnosticId: `open_streams:got=${count}:max=${MAX_OPEN_STREAMS_PER_SESSION}`,
  };
}
