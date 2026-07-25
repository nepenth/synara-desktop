import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

import {
  MATRIX_IPC_ERROR_CATEGORIES,
  MATRIX_IPC_KINDS,
  MATRIX_IPC_PROTOCOL_VERSION,
  FORBID_MEDIA_BYTES_OVER_JSON_IPC,
  MAX_STREAM_QUEUE_DEPTH,
  applyDeltaSequence,
  checkProtocolVersion,
  checkSequence,
  checkSessionGeneration,
  makeEnvelope,
  parseMatrixIpcEnvelope,
  parseMatrixIpcError,
  resyncPayloadForGap,
  transitionStreamLifecycle,
} from '../index';

/**
 * Shared fixtures live at repo-root docs/ (authoritative for Rust + TS).
 * Tests run with cwd = synara/ package root (modernization runner / local).
 */
const fixtureDir = join(process.cwd(), '../docs/matrix-rust-sdk/ipc/fixtures');

function loadFixture(name: string): unknown {
  const raw = readFileSync(join(fixtureDir, name), 'utf8');
  return JSON.parse(raw) as unknown;
}

test('protocol version and policy constants', () => {
  assert.equal(MATRIX_IPC_PROTOCOL_VERSION, 1);
  assert.equal(FORBID_MEDIA_BYTES_OVER_JSON_IPC, true);
  assert.ok(MAX_STREAM_QUEUE_DEPTH > 0);
  assert.equal(MATRIX_IPC_KINDS.length, 13);
  assert.equal(MATRIX_IPC_ERROR_CATEGORIES.length, 21);
});

test('fixture valid_hello parses', () => {
  const env = parseMatrixIpcEnvelope(loadFixture('valid_hello.json'));
  assert.ok(env);
  assert.equal(env.kind, 'hello');
  assert.equal(env.protocolVersion, 1);
  assert.equal(env.sessionGeneration, 1);
  assert.equal(env.sequence, 0);
  if (env.kind === 'hello') {
    assert.equal(env.payload.clientProtocolVersion, 1);
  }
});

test('fixture valid_hello_ack parses', () => {
  const env = parseMatrixIpcEnvelope(loadFixture('valid_hello_ack.json'));
  assert.ok(env);
  assert.equal(env.kind, 'hello_ack');
});

test('fixture valid_subscribe / snapshot / delta (snapshot then deltas)', () => {
  const sub = parseMatrixIpcEnvelope(loadFixture('valid_subscribe.json'));
  assert.ok(sub);
  assert.equal(sub.kind, 'subscribe');
  assert.equal(sub.streamId, 'stream-room-list-1');

  const snap = parseMatrixIpcEnvelope(loadFixture('valid_snapshot.json'));
  assert.ok(snap);
  assert.equal(snap.kind, 'snapshot');
  assert.equal(snap.sequence, 1);

  const delta = parseMatrixIpcEnvelope(loadFixture('valid_delta.json'));
  assert.ok(delta);
  assert.equal(delta.kind, 'delta');
  assert.equal(delta.sequence, 2);

  // Ordered model: snapshot seq 1, delta 2 accepted after last=1
  assert.deepEqual(checkSequence(1, 2), { type: 'accept', nextLastApplied: 2 });
});

test('fixture valid_error_rate_limited is privacy-safe', () => {
  const env = parseMatrixIpcEnvelope(loadFixture('valid_error_rate_limited.json'));
  assert.ok(env);
  assert.equal(env.kind, 'error');
  if (env.kind === 'error') {
    assert.equal(env.payload.category, 'rate_limited');
    assert.equal(env.payload.retryAfterMs, 5000);
    assert.ok(env.payload.diagnosticId);
  }
  const raw = JSON.stringify(loadFixture('valid_error_rate_limited.json'));
  assert.equal(raw.includes('access_token'), false);
  assert.equal(raw.includes('recovery_key'), false);
});

test('fixture valid_resync_required parses', () => {
  const env = parseMatrixIpcEnvelope(loadFixture('valid_resync_required.json'));
  assert.ok(env);
  assert.equal(env.kind, 'resync_required');
  if (env.kind === 'resync_required') {
    assert.equal(env.payload.reason, 'sequence_gap');
  }
});

test('fixture invalid_unknown_kind rejected', () => {
  const env = parseMatrixIpcEnvelope(loadFixture('invalid_unknown_kind.json'));
  assert.equal(env, null);
});

test('fixture invalid_missing_protocol_version rejected', () => {
  const env = parseMatrixIpcEnvelope(
    loadFixture('invalid_missing_protocol_version.json')
  );
  assert.equal(env, null);
});

test('stale generation rejection', () => {
  assert.equal(checkSessionGeneration(5, 5), null);
  const err = checkSessionGeneration(5, 4);
  assert.ok(err);
  assert.equal(err.category, 'stale_session_generation');
});

test('protocol version check', () => {
  assert.equal(checkProtocolVersion(1), null);
  const err = checkProtocolVersion(99);
  assert.ok(err);
  assert.equal(err.category, 'unsupported_capability');
});

test('sequence accept / duplicate / gap / behind', () => {
  assert.deepEqual(checkSequence(null, 1), {
    type: 'accept',
    nextLastApplied: 1,
  });
  assert.deepEqual(checkSequence(1, 2), { type: 'accept', nextLastApplied: 2 });
  assert.deepEqual(checkSequence(2, 2), { type: 'duplicate', lastApplied: 2 });
  assert.deepEqual(checkSequence(2, 5), {
    type: 'gap',
    lastApplied: 2,
    observed: 5,
  });
  assert.deepEqual(checkSequence(5, 3), {
    type: 'behind',
    lastApplied: 5,
    observed: 3,
  });
});

test('snapshot then ordered deltas and gap forces resync event', () => {
  let last: number | null = null;
  const snap = checkSequence(last, 1);
  assert.equal(snap.type, 'accept');
  if (snap.type === 'accept') last = snap.nextLastApplied;

  for (const seq of [2, 3]) {
    const r = checkSequence(last, seq);
    assert.equal(r.type, 'accept');
    if (r.type === 'accept') last = r.nextLastApplied;
  }

  assert.equal(checkSequence(last, 3).type, 'duplicate');

  const { outcome, event } = applyDeltaSequence(last, 6);
  assert.equal(outcome.type, 'gap');
  assert.equal(event, 'resync_needed');
  assert.deepEqual(resyncPayloadForGap('stream-1', 3, 6), {
    streamId: 'stream-1',
    reason: 'sequence_gap',
    lastAppliedSequence: 3,
    observedSequence: 6,
  });
});

test('stream lifecycle transitions including unsubscribe cleanup', () => {
  let state = transitionStreamLifecycle('idle', 'subscribe_requested');
  assert.equal(state, 'subscribing');
  state = transitionStreamLifecycle(state!, 'subscribed_ack');
  assert.equal(state, 'snapshot_pending');
  state = transitionStreamLifecycle(state!, 'snapshot_applied');
  assert.equal(state, 'live');
  state = transitionStreamLifecycle(state!, 'duplicate_delta');
  assert.equal(state, 'live');
  state = transitionStreamLifecycle(state!, 'unsubscribe_requested');
  assert.equal(state, 'unsubscribing');
  state = transitionStreamLifecycle(state!, 'resources_released');
  assert.equal(state, 'closed');
  assert.equal(transitionStreamLifecycle('idle', 'delta_applied'), null);
});

test('makeEnvelope sets protocol version', () => {
  const env = makeEnvelope(1, 0, {
    kind: 'ping',
    payload: { nonce: 'x' },
  });
  assert.equal(env.protocolVersion, MATRIX_IPC_PROTOCOL_VERSION);
  assert.equal(env.kind, 'ping');
  const parsed = parseMatrixIpcEnvelope(env);
  assert.ok(parsed);
});

test('error parser rejects secret-looking fields', () => {
  assert.equal(
    parseMatrixIpcError({
      category: 'authentication_rejected',
      accessToken: 's3cret',
    }),
    null
  );
  assert.ok(
    parseMatrixIpcError({
      category: 'unknown',
      diagnosticId: 'diag-only',
    })
  );
});
