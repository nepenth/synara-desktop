/**
 * Stream topics, lifecycle states, and control payload types (P1.3).
 * Snapshot/delta bodies are topic-typed (see streamBody.ts, R0.3 / REV-005).
 */

export const STREAM_TOPICS = [
  'room_list',
  'timeline',
  'members',
  'typing',
  'receipts',
  'account_data',
  'presence',
  'notification_candidates',
  'crypto_status',
  'send_queue',
] as const;

export type StreamTopic = (typeof STREAM_TOPICS)[number];

export const STREAM_LIFECYCLE_STATES = [
  'idle',
  'subscribing',
  'snapshot_pending',
  'live',
  'resync_required',
  'unsubscribing',
  'closed',
  'failed',
] as const;

export type StreamLifecycleState = (typeof STREAM_LIFECYCLE_STATES)[number];

export const RESYNC_REASONS = [
  'sequence_gap',
  'stale_session_generation',
  'unknown_kind',
  'snapshot_required',
  'supervisor_reset',
] as const;

export type ResyncReason = (typeof RESYNC_REASONS)[number];

export const CANCEL_REASONS = [
  'client_request',
  'timeout',
  'session_ended',
  'stream_closed',
  'superseded',
] as const;

export type CancelReason = (typeof CANCEL_REASONS)[number];

export type HelloPayload = {
  clientProtocolVersion: number;
  clientName?: string;
};

export type HelloAckPayload = {
  protocolVersion: number;
  sessionGeneration: number;
};

export type SubscribePayload = {
  topic: StreamTopic;
  streamId: string;
  params?: unknown;
};

export type UnsubscribePayload = {
  streamId: string;
};

export type SubscribedPayload = {
  streamId: string;
  topic: StreamTopic;
};

export type UnsubscribedPayload = {
  streamId: string;
  resourcesReleased?: boolean;
};

export type SnapshotPayload = {
  streamId: string;
  topic: StreamTopic;
  snapshotId: string;
  /** Topic-bound domain body (validated via validateStreamTopicBody). */
  body?: unknown;
};

export type DeltaPayload = {
  streamId: string;
  topic: StreamTopic;
  idempotencyKey?: string;
  /** Topic-bound domain body (validated via validateStreamTopicBody). */
  body?: unknown;
};

export type ResyncRequiredPayload = {
  streamId?: string;
  reason: ResyncReason;
  lastAppliedSequence?: number;
  observedSequence?: number;
};

export type CancelPayload = {
  cancellationToken: string;
  reason?: CancelReason;
};

export type PingPayload = {
  nonce?: string;
};

export type PongPayload = {
  nonce?: string;
};

const TOPIC_SET = new Set<string>(STREAM_TOPICS);
const RESYNC_SET = new Set<string>(RESYNC_REASONS);
const CANCEL_SET = new Set<string>(CANCEL_REASONS);

export function isStreamTopic(value: unknown): value is StreamTopic {
  return typeof value === 'string' && TOPIC_SET.has(value);
}

export function isResyncReason(value: unknown): value is ResyncReason {
  return typeof value === 'string' && RESYNC_SET.has(value);
}

export function isCancelReason(value: unknown): value is CancelReason {
  return typeof value === 'string' && CANCEL_SET.has(value);
}
