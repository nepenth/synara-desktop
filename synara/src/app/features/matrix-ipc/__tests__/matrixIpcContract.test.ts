/**
 * P1.5 — Expanded IPC protocol contract tests (TypeScript mirror).
 *
 * Shared fixtures under docs/matrix-rust-sdk/ipc/fixtures/ are authoritative
 * for both Rust and TS. No matrix-js-sdk, no production session bootstrap.
 */

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

import {
  MATRIX_IPC_ERROR_CATEGORIES,
  MATRIX_IPC_KINDS,
  MATRIX_IPC_PROTOCOL_VERSION,
  MAX_ENVELOPE_PAYLOAD_JSON_BYTES,
  MAX_OPEN_STREAMS_PER_SESSION,
  MAX_STREAM_QUEUE_DEPTH,
  STREAM_COALESCE_WINDOW_MS,
  FORBID_MEDIA_BYTES_OVER_JSON_IPC,
  applyDeltaSequence,
  checkProtocolVersion,
  checkSequence,
  checkSessionGeneration,
  makeEnvelope,
  openStreamsWithinBounds,
  parseMatrixIpcEnvelope,
  parseMatrixIpcError,
  payloadWithinBounds,
  resyncPayloadForGap,
  resyncPayloadForStaleGeneration,
  streamQueueDepthWithinBounds,
  transitionStreamLifecycle,
} from '../index';

import { parseRoomSummary } from '../../matrix-dto/index';

const fixtureDir = join(process.cwd(), '../docs/matrix-rust-sdk/ipc/fixtures');

function loadFixture(name: string): unknown {
  const raw = readFileSync(join(fixtureDir, name), 'utf8');
  return JSON.parse(raw) as unknown;
}

function loadRaw(name: string): string {
  return readFileSync(join(fixtureDir, name), 'utf8');
}

const VALID_FIXTURES = [
  'valid_hello.json',
  'valid_hello_ack.json',
  'valid_subscribe.json',
  'valid_unsubscribe.json',
  'valid_subscribed.json',
  'valid_unsubscribed.json',
  'valid_snapshot.json',
  'valid_delta.json',
  'valid_resync_required.json',
  'valid_resync_stale_generation.json',
  'valid_cancel.json',
  'valid_error_rate_limited.json',
  'valid_error_stale_session.json',
  'valid_ping.json',
  'valid_pong.json',
  'valid_snapshot_with_room_summary_body.json',
] as const;

const INVALID_FIXTURES = [
  'invalid_unknown_kind.json',
  'invalid_missing_protocol_version.json',
  'invalid_missing_session_generation.json',
  'invalid_missing_sequence.json',
  'invalid_missing_kind.json',
  'invalid_missing_payload.json',
  'invalid_wrong_type_protocol_version.json',
  'invalid_wrong_type_sequence.json',
  'invalid_unknown_error_category.json',
  'invalid_unknown_topic.json',
  'invalid_subscribe_missing_stream_id.json',
  'invalid_hello_missing_client_protocol_version.json',
  'invalid_unknown_resync_reason.json',
  'invalid_error_with_secret_field.json',
] as const;

// ---------------------------------------------------------------------------
// Policy constants / bounds
// ---------------------------------------------------------------------------

test('P1.5 policy constants exact values', () => {
  assert.equal(MATRIX_IPC_PROTOCOL_VERSION, 1);
  assert.equal(MAX_ENVELOPE_PAYLOAD_JSON_BYTES, 1_048_576);
  assert.equal(MAX_STREAM_QUEUE_DEPTH, 256);
  assert.equal(STREAM_COALESCE_WINDOW_MS, 16);
  assert.equal(MAX_OPEN_STREAMS_PER_SESSION, 64);
  assert.equal(FORBID_MEDIA_BYTES_OVER_JSON_IPC, true);
  assert.equal(MATRIX_IPC_KINDS.length, 13);
  assert.equal(MATRIX_IPC_ERROR_CATEGORIES.length, 21);
});

test('P1.5 payload size bounds policy', () => {
  assert.equal(payloadWithinBounds(0), true);
  assert.equal(payloadWithinBounds(MAX_ENVELOPE_PAYLOAD_JSON_BYTES), true);
  assert.equal(payloadWithinBounds(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 1), false);
});

test('P1.5 stream queue and open stream bounds policy', () => {
  assert.equal(streamQueueDepthWithinBounds(0), true);
  assert.equal(streamQueueDepthWithinBounds(MAX_STREAM_QUEUE_DEPTH), true);
  assert.equal(streamQueueDepthWithinBounds(MAX_STREAM_QUEUE_DEPTH + 1), false);
  assert.equal(openStreamsWithinBounds(0), true);
  assert.equal(openStreamsWithinBounds(MAX_OPEN_STREAMS_PER_SESSION), true);
  assert.equal(openStreamsWithinBounds(MAX_OPEN_STREAMS_PER_SESSION + 1), false);
});

test('P1.5 typical fixture envelope within payload bounds', () => {
  const raw = loadRaw('valid_snapshot.json');
  assert.equal(payloadWithinBounds(Buffer.byteLength(raw, 'utf8')), true);
});

// ---------------------------------------------------------------------------
// Exhaustive kind + error-category round trips
// ---------------------------------------------------------------------------

test('P1.5 all kinds constructible via makeEnvelope and re-parse', () => {
  const samples: Array<{ kind: (typeof MATRIX_IPC_KINDS)[number]; payload: unknown }> = [
    {
      kind: 'hello',
      payload: { clientProtocolVersion: 1, clientName: 'synara-web' },
    },
    {
      kind: 'hello_ack',
      payload: { protocolVersion: 1, sessionGeneration: 1 },
    },
    {
      kind: 'subscribe',
      payload: { topic: 'room_list', streamId: 's1', params: {} },
    },
    { kind: 'unsubscribe', payload: { streamId: 's1' } },
    { kind: 'subscribed', payload: { streamId: 's1', topic: 'room_list' } },
    {
      kind: 'unsubscribed',
      payload: { streamId: 's1', resourcesReleased: true },
    },
    {
      kind: 'snapshot',
      payload: {
        streamId: 's1',
        topic: 'timeline',
        snapshotId: 'snap-1',
        body: { items: [] },
      },
    },
    {
      kind: 'delta',
      payload: {
        streamId: 's1',
        topic: 'timeline',
        idempotencyKey: 'idem-1',
        body: { op: 'upsert' },
      },
    },
    {
      kind: 'resync_required',
      payload: {
        streamId: 's1',
        reason: 'sequence_gap',
        lastAppliedSequence: 2,
        observedSequence: 5,
      },
    },
    {
      kind: 'cancel',
      payload: { cancellationToken: 'tok', reason: 'timeout' },
    },
    {
      kind: 'error',
      payload: { category: 'connectivity', diagnosticId: 'diag-c' },
    },
    { kind: 'ping', payload: { nonce: 'n' } },
    { kind: 'pong', payload: { nonce: 'n' } },
  ];

  assert.equal(samples.length, MATRIX_IPC_KINDS.length);

  for (let i = 0; i < samples.length; i++) {
    const sample = samples[i]!;
    assert.equal(sample.kind, MATRIX_IPC_KINDS[i]);
    const env = makeEnvelope(
      1,
      i,
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      { kind: sample.kind, payload: sample.payload } as any,
      { requestId: `req-${i}` }
    );
    assert.equal(env.protocolVersion, MATRIX_IPC_PROTOCOL_VERSION);
    assert.equal(env.kind, sample.kind);
    const parsed = parseMatrixIpcEnvelope(env);
    assert.ok(parsed, `kind ${sample.kind} must re-parse`);
    assert.equal(parsed.kind, sample.kind);
    assert.equal(parsed.sequence, i);
  }
});

test('P1.5 all error categories parse and remain privacy-safe', () => {
  for (const category of MATRIX_IPC_ERROR_CATEGORIES) {
    const err = parseMatrixIpcError({
      category,
      diagnosticId: 'diag-test',
      retryAfterMs: 1000,
    });
    assert.ok(err, `category ${category} must parse`);
    assert.equal(err.category, category);
    assert.equal(
      parseMatrixIpcError({
        category,
        accessToken: 's3cret',
      }),
      null,
      `category ${category} must reject secret fields`
    );
  }
});

// ---------------------------------------------------------------------------
// Shared fixtures — valid + round trip
// ---------------------------------------------------------------------------

test('P1.5 all valid fixtures parse and re-parse', () => {
  for (const name of VALID_FIXTURES) {
    const value = loadFixture(name);
    const env = parseMatrixIpcEnvelope(value);
    assert.ok(env, `${name} must parse`);
    assert.equal(env.protocolVersion, MATRIX_IPC_PROTOCOL_VERSION);
    assert.ok(MATRIX_IPC_KINDS.includes(env.kind));
    // Round-trip via JSON stringify
    const again = parseMatrixIpcEnvelope(JSON.parse(JSON.stringify(env)));
    assert.ok(again, `${name} must re-parse after stringify`);
    assert.equal(again.kind, env.kind);
    assert.equal(again.sequence, env.sequence);
    assert.equal(again.sessionGeneration, env.sessionGeneration);
  }
});

test('P1.5 lifecycle control kind fixtures', () => {
  assert.equal(parseMatrixIpcEnvelope(loadFixture('valid_unsubscribe.json'))?.kind, 'unsubscribe');
  assert.equal(parseMatrixIpcEnvelope(loadFixture('valid_subscribed.json'))?.kind, 'subscribed');
  assert.equal(
    parseMatrixIpcEnvelope(loadFixture('valid_unsubscribed.json'))?.kind,
    'unsubscribed'
  );
  assert.equal(parseMatrixIpcEnvelope(loadFixture('valid_cancel.json'))?.kind, 'cancel');
  assert.equal(parseMatrixIpcEnvelope(loadFixture('valid_ping.json'))?.kind, 'ping');
  assert.equal(parseMatrixIpcEnvelope(loadFixture('valid_pong.json'))?.kind, 'pong');
});

test('P1.5 snapshot body composes with P1.4 RoomSummary DTO', () => {
  const env = parseMatrixIpcEnvelope(
    loadFixture('valid_snapshot_with_room_summary_body.json')
  );
  assert.ok(env);
  assert.equal(env.kind, 'snapshot');
  if (env.kind !== 'snapshot') return;
  const body = env.payload.body as { rooms?: unknown[] };
  assert.ok(Array.isArray(body.rooms));
  assert.equal(body.rooms?.length, 1);
  const summary = parseRoomSummary(body.rooms![0]);
  assert.ok(summary);
  assert.equal(summary.roomId, '!room:example.org');
  assert.equal(summary.membership, 'join');
});

// ---------------------------------------------------------------------------
// Shared fixtures — invalid / unknown variants
// ---------------------------------------------------------------------------

test('P1.5 all invalid fixtures rejected', () => {
  for (const name of INVALID_FIXTURES) {
    const env = parseMatrixIpcEnvelope(loadFixture(name));
    assert.equal(env, null, `${name} must be rejected`);
  }
});

test('P1.5 unknown kind rejected at boundary', () => {
  assert.equal(
    parseMatrixIpcEnvelope({
      protocolVersion: 1,
      sessionGeneration: 1,
      sequence: 0,
      kind: 'login',
      payload: {},
    }),
    null
  );
  assert.equal(
    parseMatrixIpcEnvelope({
      protocolVersion: 1,
      sessionGeneration: 1,
      sequence: 0,
      kind: 'future_experimental_kind',
      payload: { anything: true },
    }),
    null
  );
});

test('P1.5 unknown error category / topic / resync reason rejected', () => {
  assert.equal(
    parseMatrixIpcError({ category: 'not_a_real_category', diagnosticId: 'x' }),
    null
  );
  assert.equal(
    parseMatrixIpcEnvelope({
      protocolVersion: 1,
      sessionGeneration: 1,
      sequence: 0,
      kind: 'subscribe',
      payload: { topic: 'not_a_real_topic', streamId: 's' },
    }),
    null
  );
  assert.equal(
    parseMatrixIpcEnvelope({
      protocolVersion: 1,
      sessionGeneration: 1,
      sequence: 0,
      kind: 'resync_required',
      payload: { reason: 'not_a_resync_reason' },
    }),
    null
  );
});

// ---------------------------------------------------------------------------
// Invalid payloads — missing required fields / wrong types
// ---------------------------------------------------------------------------

test('P1.5 missing required envelope fields rejected', () => {
  assert.equal(
    parseMatrixIpcEnvelope({
      sessionGeneration: 1,
      sequence: 0,
      kind: 'ping',
      payload: {},
    }),
    null
  );
  assert.equal(
    parseMatrixIpcEnvelope({
      protocolVersion: 1,
      sequence: 0,
      kind: 'ping',
      payload: {},
    }),
    null
  );
  assert.equal(
    parseMatrixIpcEnvelope({
      protocolVersion: 1,
      sessionGeneration: 1,
      kind: 'ping',
      payload: {},
    }),
    null
  );
  assert.equal(
    parseMatrixIpcEnvelope({
      protocolVersion: 1,
      sessionGeneration: 1,
      sequence: 0,
      payload: {},
    }),
    null
  );
  assert.equal(
    parseMatrixIpcEnvelope({
      protocolVersion: 1,
      sessionGeneration: 1,
      sequence: 0,
      kind: 'ping',
    }),
    null
  );
});

test('P1.5 wrong types rejected', () => {
  assert.equal(
    parseMatrixIpcEnvelope({
      protocolVersion: '1',
      sessionGeneration: 1,
      sequence: 0,
      kind: 'ping',
      payload: {},
    }),
    null
  );
  assert.equal(
    parseMatrixIpcEnvelope({
      protocolVersion: 1,
      sessionGeneration: 1,
      sequence: '0',
      kind: 'ping',
      payload: {},
    }),
    null
  );
  assert.equal(
    parseMatrixIpcEnvelope({
      protocolVersion: 1,
      sessionGeneration: true,
      sequence: 0,
      kind: 'ping',
      payload: {},
    }),
    null
  );
  assert.equal(
    parseMatrixIpcEnvelope({
      protocolVersion: 1,
      sessionGeneration: 1,
      sequence: 0,
      kind: 'ping',
      payload: 'not-an-object',
    }),
    null
  );
  assert.equal(
    parseMatrixIpcEnvelope({
      protocolVersion: 1,
      sessionGeneration: 1,
      sequence: 0,
      kind: 'ping',
      payload: [],
    }),
    null
  );
});

test('P1.5 invalid kind-specific required fields rejected', () => {
  assert.equal(
    parseMatrixIpcEnvelope({
      protocolVersion: 1,
      sessionGeneration: 1,
      sequence: 0,
      kind: 'hello',
      payload: { clientName: 'synara-web' },
    }),
    null
  );
  assert.equal(
    parseMatrixIpcEnvelope({
      protocolVersion: 1,
      sessionGeneration: 1,
      sequence: 0,
      kind: 'subscribe',
      payload: { topic: 'room_list' },
    }),
    null
  );
  assert.equal(
    parseMatrixIpcEnvelope({
      protocolVersion: 1,
      sessionGeneration: 1,
      sequence: 1,
      kind: 'snapshot',
      payload: { streamId: 's1', topic: 'room_list', body: {} },
    }),
    null
  );
  assert.equal(
    parseMatrixIpcEnvelope({
      protocolVersion: 1,
      sessionGeneration: 1,
      sequence: 0,
      kind: 'cancel',
      payload: { reason: 'timeout' },
    }),
    null
  );
});

// ---------------------------------------------------------------------------
// Sequence gaps → resync_required path
// ---------------------------------------------------------------------------

test('P1.5 sequence gap produces resync_required payload and lifecycle event', () => {
  const { outcome, event } = applyDeltaSequence(2, 5);
  assert.equal(outcome.type, 'gap');
  if (outcome.type === 'gap') {
    assert.equal(outcome.lastApplied, 2);
    assert.equal(outcome.observed, 5);
  }
  assert.equal(event, 'resync_needed');

  const payload = resyncPayloadForGap('stream-room-list-1', 2, 5);
  assert.deepEqual(payload, {
    streamId: 'stream-room-list-1',
    reason: 'sequence_gap',
    lastAppliedSequence: 2,
    observedSequence: 5,
  });

  const env = makeEnvelope(1, 0, {
    kind: 'resync_required',
    payload,
  }, { streamId: 'stream-room-list-1' });
  const parsed = parseMatrixIpcEnvelope(env);
  assert.ok(parsed);
  assert.equal(parsed.kind, 'resync_required');

  // Fixture parity
  const fixtureEnv = parseMatrixIpcEnvelope(loadFixture('valid_resync_required.json'));
  assert.ok(fixtureEnv);
  assert.equal(fixtureEnv.kind, 'resync_required');
  if (fixtureEnv.kind === 'resync_required') {
    assert.equal(fixtureEnv.payload.reason, 'sequence_gap');
    assert.equal(fixtureEnv.payload.lastAppliedSequence, 2);
    assert.equal(fixtureEnv.payload.observedSequence, 5);
  }
});

test('P1.5 behind sequence forces resync event', () => {
  const { outcome, event } = applyDeltaSequence(10, 3);
  assert.equal(outcome.type, 'behind');
  assert.equal(event, 'resync_needed');
});

test('P1.5 gap drives lifecycle to resync_required then resubscribe', () => {
  let state = transitionStreamLifecycle('live', 'resync_needed');
  assert.equal(state, 'resync_required');
  state = transitionStreamLifecycle(state!, 'subscribe_requested');
  assert.equal(state, 'subscribing');
});

test('P1.5 ordered snapshot then deltas; duplicate is idempotent', () => {
  let last: number | null = null;
  const snap = checkSequence(last, 1);
  assert.equal(snap.type, 'accept');
  if (snap.type === 'accept') last = snap.nextLastApplied;
  for (const seq of [2, 3, 4]) {
    const r = checkSequence(last, seq);
    assert.equal(r.type, 'accept', `seq ${seq}`);
    if (r.type === 'accept') last = r.nextLastApplied;
  }
  assert.equal(checkSequence(last, 4).type, 'duplicate');
  assert.equal(checkSequence(last, 9).type, 'gap');
});

// ---------------------------------------------------------------------------
// Stale session generation rejection
// ---------------------------------------------------------------------------

test('P1.5 stale session generation rejected and resync payload', () => {
  assert.equal(checkSessionGeneration(7, 7), null);

  const err = checkSessionGeneration(7, 3);
  assert.ok(err);
  assert.equal(err.category, 'stale_session_generation');
  assert.ok(err.diagnosticId);

  const err2 = checkSessionGeneration(7, 99);
  assert.ok(err2);
  assert.equal(err2.category, 'stale_session_generation');

  const payload = resyncPayloadForStaleGeneration('stream-timeline-1');
  assert.equal(payload.reason, 'stale_session_generation');
  assert.equal(payload.streamId, 'stream-timeline-1');

  const fixtureEnv = parseMatrixIpcEnvelope(
    loadFixture('valid_resync_stale_generation.json')
  );
  assert.ok(fixtureEnv);
  assert.equal(fixtureEnv.kind, 'resync_required');
  if (fixtureEnv.kind === 'resync_required') {
    assert.equal(fixtureEnv.payload.reason, 'stale_session_generation');
  }

  const errEnv = parseMatrixIpcEnvelope(loadFixture('valid_error_stale_session.json'));
  assert.ok(errEnv);
  assert.equal(errEnv.kind, 'error');
  if (errEnv.kind === 'error') {
    assert.equal(errEnv.payload.category, 'stale_session_generation');
  }
});

test('P1.5 envelope generation mismatch vs live session', () => {
  const env = parseMatrixIpcEnvelope(loadFixture('valid_delta.json'));
  assert.ok(env);
  // Fixture generation is 1; live 5 → reject
  assert.ok(checkSessionGeneration(5, env.sessionGeneration));
  assert.equal(checkSessionGeneration(env.sessionGeneration, env.sessionGeneration), null);
});

// ---------------------------------------------------------------------------
// Protocol version
// ---------------------------------------------------------------------------

test('P1.5 unsupported protocol version rejected', () => {
  assert.equal(checkProtocolVersion(1), null);
  assert.equal(checkProtocolVersion(0)?.category, 'unsupported_capability');
  assert.equal(checkProtocolVersion(2)?.category, 'unsupported_capability');
});

// ---------------------------------------------------------------------------
// Fixture inventory JSON validity
// ---------------------------------------------------------------------------

test('P1.5 fixture inventory files are valid JSON objects', () => {
  for (const name of [...VALID_FIXTURES, ...INVALID_FIXTURES]) {
    const value = loadFixture(name);
    assert.equal(typeof value, 'object');
    assert.ok(value !== null);
    assert.equal(Array.isArray(value), false);
  }
});
