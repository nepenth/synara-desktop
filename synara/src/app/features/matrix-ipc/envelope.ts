/**
 * Versioned Matrix IPC envelope and exhaustive kind union (P1.3).
 *
 * Unknown kinds: reject at the command boundary. For stream events with
 * invalid generation/sequence, signal resync_required (see protocol helpers).
 */

import type { MatrixIpcError } from './error';
import { parseMatrixIpcError } from './error';
import type {
  CancelPayload,
  DeltaPayload,
  HelloAckPayload,
  HelloPayload,
  PingPayload,
  PongPayload,
  ResyncRequiredPayload,
  SnapshotPayload,
  SubscribePayload,
  SubscribedPayload,
  UnsubscribePayload,
  UnsubscribedPayload,
} from './stream';
import { isCancelReason, isResyncReason, isStreamTopic } from './stream';
import { validateStreamTopicBody } from './streamBody';
import { MATRIX_IPC_PROTOCOL_VERSION, isWireCounter } from './version';

export const MATRIX_IPC_KINDS = [
  'hello',
  'hello_ack',
  'subscribe',
  'unsubscribe',
  'subscribed',
  'unsubscribed',
  'snapshot',
  'delta',
  'resync_required',
  'cancel',
  'error',
  'ping',
  'pong',
] as const;

export type MatrixIpcKind = (typeof MATRIX_IPC_KINDS)[number];

const KIND_SET = new Set<string>(MATRIX_IPC_KINDS);

export function isMatrixIpcKind(value: unknown): value is MatrixIpcKind {
  return typeof value === 'string' && KIND_SET.has(value);
}

/** Discriminated message union (adjacent tag: kind + payload). */
export type MatrixIpcMessage =
  | { kind: 'hello'; payload: HelloPayload }
  | { kind: 'hello_ack'; payload: HelloAckPayload }
  | { kind: 'subscribe'; payload: SubscribePayload }
  | { kind: 'unsubscribe'; payload: UnsubscribePayload }
  | { kind: 'subscribed'; payload: SubscribedPayload }
  | { kind: 'unsubscribed'; payload: UnsubscribedPayload }
  | { kind: 'snapshot'; payload: SnapshotPayload }
  | { kind: 'delta'; payload: DeltaPayload }
  | { kind: 'resync_required'; payload: ResyncRequiredPayload }
  | { kind: 'cancel'; payload: CancelPayload }
  | { kind: 'error'; payload: MatrixIpcError }
  | { kind: 'ping'; payload: PingPayload }
  | { kind: 'pong'; payload: PongPayload };

/** Versioned IPC envelope (plan §6.3). */
export type MatrixIpcEnvelope = {
  protocolVersion: number;
  sessionGeneration: number;
  streamId?: string;
  sequence: number;
  requestId?: string;
} & MatrixIpcMessage;

function isObject(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

function requireString(o: Record<string, unknown>, key: string): string | null {
  const v = o[key];
  return typeof v === 'string' ? v : null;
}

function parseMessage(kind: MatrixIpcKind, payload: unknown): MatrixIpcMessage | null {
  if (!isObject(payload) && kind !== 'ping' && kind !== 'pong') {
    // ping/pong may have empty object; still require object for consistency
  }
  if (!isObject(payload)) return null;

  switch (kind) {
    case 'hello': {
      if (typeof payload.clientProtocolVersion !== 'number') return null;
      return {
        kind,
        payload: {
          clientProtocolVersion: payload.clientProtocolVersion,
          clientName: typeof payload.clientName === 'string' ? payload.clientName : undefined,
        },
      };
    }
    case 'hello_ack': {
      if (typeof payload.protocolVersion !== 'number') return null;
      if (!isWireCounter(payload.sessionGeneration)) return null;
      return {
        kind,
        payload: {
          protocolVersion: payload.protocolVersion,
          sessionGeneration: payload.sessionGeneration,
        },
      };
    }
    case 'subscribe': {
      if (!isStreamTopic(payload.topic)) return null;
      const streamId = requireString(payload, 'streamId');
      if (!streamId) return null;
      return {
        kind,
        payload: {
          topic: payload.topic,
          streamId,
          params: payload.params,
        },
      };
    }
    case 'unsubscribe': {
      const streamId = requireString(payload, 'streamId');
      if (!streamId) return null;
      return { kind, payload: { streamId } };
    }
    case 'subscribed': {
      if (!isStreamTopic(payload.topic)) return null;
      const streamId = requireString(payload, 'streamId');
      if (!streamId) return null;
      return { kind, payload: { streamId, topic: payload.topic } };
    }
    case 'unsubscribed': {
      const streamId = requireString(payload, 'streamId');
      if (!streamId) return null;
      return {
        kind,
        payload: {
          streamId,
          resourcesReleased:
            typeof payload.resourcesReleased === 'boolean' ? payload.resourcesReleased : undefined,
        },
      };
    }
    case 'snapshot': {
      if (!isStreamTopic(payload.topic)) return null;
      const streamId = requireString(payload, 'streamId');
      const snapshotId = requireString(payload, 'snapshotId');
      if (!streamId || !snapshotId) return null;
      const body = payload.body === undefined ? {} : payload.body;
      if (!validateStreamTopicBody(payload.topic, body)) return null;
      return {
        kind,
        payload: {
          streamId,
          topic: payload.topic,
          snapshotId,
          body,
        },
      };
    }
    case 'delta': {
      if (!isStreamTopic(payload.topic)) return null;
      const streamId = requireString(payload, 'streamId');
      if (!streamId) return null;
      const body = payload.body === undefined ? {} : payload.body;
      if (!validateStreamTopicBody(payload.topic, body)) return null;
      return {
        kind,
        payload: {
          streamId,
          topic: payload.topic,
          idempotencyKey:
            typeof payload.idempotencyKey === 'string' ? payload.idempotencyKey : undefined,
          body,
        },
      };
    }
    case 'resync_required': {
      if (!isResyncReason(payload.reason)) return null;
      if (
        payload.lastAppliedSequence !== undefined &&
        !isWireCounter(payload.lastAppliedSequence)
      ) {
        return null;
      }
      if (payload.observedSequence !== undefined && !isWireCounter(payload.observedSequence)) {
        return null;
      }
      return {
        kind,
        payload: {
          streamId: typeof payload.streamId === 'string' ? payload.streamId : undefined,
          reason: payload.reason,
          lastAppliedSequence: isWireCounter(payload.lastAppliedSequence)
            ? payload.lastAppliedSequence
            : undefined,
          observedSequence: isWireCounter(payload.observedSequence)
            ? payload.observedSequence
            : undefined,
        },
      };
    }
    case 'cancel': {
      const cancellationToken = requireString(payload, 'cancellationToken');
      if (!cancellationToken) return null;
      if (payload.reason !== undefined && !isCancelReason(payload.reason)) return null;
      return {
        kind,
        payload: {
          cancellationToken,
          reason: isCancelReason(payload.reason) ? payload.reason : undefined,
        },
      };
    }
    case 'error': {
      const err = parseMatrixIpcError(payload);
      if (!err) return null;
      return { kind, payload: err };
    }
    case 'ping':
      return {
        kind,
        payload: {
          nonce: typeof payload.nonce === 'string' ? payload.nonce : undefined,
        },
      };
    case 'pong':
      return {
        kind,
        payload: {
          nonce: typeof payload.nonce === 'string' ? payload.nonce : undefined,
        },
      };
    default: {
      // Exhaustiveness: unknown kinds rejected by isMatrixIpcKind.
      const _exhaustive: never = kind;
      return _exhaustive;
    }
  }
}

/**
 * Parse and validate a JSON value as a Matrix IPC envelope.
 * Returns null when the envelope is invalid or the kind is unknown (reject policy).
 * R0.3: counters must be wire-safe; stream-scoped kinds require matching streamId.
 */
export function parseMatrixIpcEnvelope(value: unknown): MatrixIpcEnvelope | null {
  if (!isObject(value)) return null;
  if (typeof value.protocolVersion !== 'number') return null;
  if (!isWireCounter(value.sessionGeneration)) return null;
  if (!isWireCounter(value.sequence)) return null;
  if (value.streamId !== undefined && typeof value.streamId !== 'string') return null;
  if (value.requestId !== undefined && typeof value.requestId !== 'string') return null;
  if (!isMatrixIpcKind(value.kind)) return null;
  if (!isObject(value.payload)) return null;

  const message = parseMessage(value.kind, value.payload);
  if (!message) return null;

  const streamId = typeof value.streamId === 'string' ? value.streamId : undefined;
  if (!streamIdAuthorityOk(message, streamId)) return null;

  return {
    protocolVersion: value.protocolVersion,
    sessionGeneration: value.sessionGeneration,
    streamId,
    sequence: value.sequence,
    requestId: typeof value.requestId === 'string' ? value.requestId : undefined,
    ...message,
  };
}

/** R0.3 / REV-005: single authoritative stream id for stream-scoped kinds. */
function streamIdAuthorityOk(message: MatrixIpcMessage, envelopeStreamId?: string): boolean {
  const payloadStreamId = payloadStreamIdOf(message);
  switch (message.kind) {
    case 'subscribe':
    case 'unsubscribe':
    case 'subscribed':
    case 'unsubscribed':
    case 'snapshot':
    case 'delta':
      return (
        typeof envelopeStreamId === 'string' &&
        typeof payloadStreamId === 'string' &&
        envelopeStreamId === payloadStreamId
      );
    case 'resync_required':
      if (payloadStreamId === undefined) return true;
      return envelopeStreamId === payloadStreamId;
    default:
      return true;
  }
}

function payloadStreamIdOf(message: MatrixIpcMessage): string | undefined {
  switch (message.kind) {
    case 'subscribe':
    case 'unsubscribe':
    case 'subscribed':
    case 'unsubscribed':
    case 'snapshot':
    case 'delta':
      return message.payload.streamId;
    case 'resync_required':
      return message.payload.streamId;
    default:
      return undefined;
  }
}

/** Build a typed envelope with the current protocol version. */
export function makeEnvelope(
  sessionGeneration: number,
  sequence: number,
  message: MatrixIpcMessage,
  opts?: { streamId?: string; requestId?: string },
): MatrixIpcEnvelope {
  if (!isWireCounter(sessionGeneration) || !isWireCounter(sequence)) {
    throw new Error('makeEnvelope: sessionGeneration/sequence must be wire-safe counters');
  }
  const env: MatrixIpcEnvelope = {
    protocolVersion: MATRIX_IPC_PROTOCOL_VERSION,
    sessionGeneration,
    sequence,
    streamId: opts?.streamId,
    requestId: opts?.requestId,
    ...message,
  };
  if (!streamIdAuthorityOk(message, opts?.streamId)) {
    throw new Error('makeEnvelope: stream-scoped kinds require matching streamId');
  }
  return env;
}
