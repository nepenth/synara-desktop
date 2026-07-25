import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

import {
  MATRIX_IPC_ERROR_CATEGORIES,
  MATRIX_IPC_KINDS,
  MATRIX_IPC_PROTOCOL_VERSION,
  FORBID_MEDIA_BYTES_OVER_JSON_IPC,
  MAX_ENVELOPE_PAYLOAD_JSON_BYTES,
  MAX_OPEN_STREAMS_PER_SESSION,
  MAX_STREAM_QUEUE_DEPTH,
  STREAM_COALESCE_WINDOW_MS,
  applyDeltaSequence,
  checkOpenStreams,
  checkPayloadJsonBounds,
  checkProtocolVersion,
  checkSequence,
  checkSessionGeneration,
  checkStreamQueueDepth,
  makeEnvelope,
  parseMatrixIpcEnvelope,
  parseMatrixIpcError,
  resyncPayloadForGap,
  resyncPayloadForStaleGeneration,
  transitionStreamLifecycle,
} from '../index';

/**
 * Shared fixtures live at repo-root docs/ (authoritative for Rust + TS).
 * Tests run with cwd = synara/ package root (modernization runner / local).
 */
const docsIpcDir = join(process.cwd(), '../docs/matrix-rust-sdk/ipc');
const fixtureDir = join(docsIpcDir, 'fixtures');

function loadFixture(name: string): unknown {
  const raw = readFileSync(join(fixtureDir, name), 'utf8');
  return JSON.parse(raw) as unknown;
}

function loadSchemaCatalog(): {
  protocolVersion: number;
  kinds: string[];
  errorCategories: string[];
  streamTopics: string[];
  bounds: {
    maxEnvelopePayloadJsonBytes: number;
    maxStreamQueueDepth: number;
    streamCoalesceWindowMs: number;
    maxOpenStreamsPerSession: number;
    forbidMediaBytesOverJsonIpc: boolean;
  };
} {
  const raw = readFileSync(join(docsIpcDir, 'schema_catalog_v1.json'), 'utf8');
  return JSON.parse(raw) as ReturnType<typeof loadSchemaCatalog>;
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

// ---------------------------------------------------------------------------
// P1.5 — expanded contract suite
// ---------------------------------------------------------------------------

test('all control kinds round-trip through makeEnvelope + parse', () => {
  const samples = [
    makeEnvelope(1, 0, {
      kind: 'hello',
      payload: { clientProtocolVersion: 1, clientName: 'synara-web' },
    }),
    makeEnvelope(1, 0, {
      kind: 'hello_ack',
      payload: { protocolVersion: 1, sessionGeneration: 1 },
    }),
    makeEnvelope(
      1,
      0,
      { kind: 'subscribe', payload: { topic: 'room_list', streamId: 's1', params: {} } },
      { streamId: 's1' }
    ),
    makeEnvelope(
      1,
      0,
      { kind: 'unsubscribe', payload: { streamId: 's1' } },
      { streamId: 's1' }
    ),
    makeEnvelope(
      1,
      0,
      { kind: 'subscribed', payload: { streamId: 's1', topic: 'room_list' } },
      { streamId: 's1' }
    ),
    makeEnvelope(
      1,
      0,
      {
        kind: 'unsubscribed',
        payload: { streamId: 's1', resourcesReleased: true },
      },
      { streamId: 's1' }
    ),
    makeEnvelope(
      1,
      1,
      {
        kind: 'snapshot',
        payload: {
          streamId: 's1',
          topic: 'timeline',
          snapshotId: 'snap-1',
          body: { items: [] },
        },
      },
      { streamId: 's1' }
    ),
    makeEnvelope(
      1,
      2,
      {
        kind: 'delta',
        payload: {
          streamId: 's1',
          topic: 'timeline',
          idempotencyKey: 'idem-1',
          body: { op: 'append' },
        },
      },
      { streamId: 's1' }
    ),
    makeEnvelope(
      1,
      0,
      {
        kind: 'resync_required',
        payload: resyncPayloadForGap('s1', 2, 5),
      },
      { streamId: 's1' }
    ),
    makeEnvelope(1, 0, {
      kind: 'cancel',
      payload: { cancellationToken: 'tok', reason: 'timeout' },
    }),
    makeEnvelope(1, 0, {
      kind: 'error',
      payload: { category: 'connectivity', diagnosticId: 'diag-net' },
    }),
    makeEnvelope(1, 0, { kind: 'ping', payload: { nonce: 'n' } }),
    makeEnvelope(1, 0, { kind: 'pong', payload: { nonce: 'n' } }),
  ];

  assert.equal(samples.length, MATRIX_IPC_KINDS.length);
  const seen = new Set<string>();
  for (const env of samples) {
    seen.add(env.kind);
    const parsed = parseMatrixIpcEnvelope(JSON.parse(JSON.stringify(env)));
    assert.ok(parsed, `round-trip failed for ${env.kind}`);
    assert.equal(parsed.kind, env.kind);
    assert.equal(parsed.protocolVersion, MATRIX_IPC_PROTOCOL_VERSION);
    assert.equal(parsed.sessionGeneration, env.sessionGeneration);
    assert.equal(parsed.sequence, env.sequence);
  }
  for (const k of MATRIX_IPC_KINDS) {
    assert.ok(seen.has(k), `missing round-trip sample for ${k}`);
  }
});

test('fixture valid remaining control kinds parse', () => {
  for (const [name, kind] of [
    ['valid_unsubscribe.json', 'unsubscribe'],
    ['valid_subscribed.json', 'subscribed'],
    ['valid_unsubscribed.json', 'unsubscribed'],
    ['valid_cancel.json', 'cancel'],
    ['valid_ping.json', 'ping'],
    ['valid_pong.json', 'pong'],
  ] as const) {
    const env = parseMatrixIpcEnvelope(loadFixture(name));
    assert.ok(env, `${name} must parse`);
    assert.equal(env.kind, kind);
  }
});

test('fixture invalid payloads rejected', () => {
  for (const name of [
    'invalid_missing_kind.json',
    'invalid_missing_sequence.json',
    'invalid_wrong_type_protocol_version.json',
    'invalid_unknown_topic.json',
    'invalid_unknown_error_category.json',
    'invalid_error_with_secret_field.json',
    'invalid_hello_missing_client_protocol_version.json',
    'invalid_unknown_kind.json',
    'invalid_missing_protocol_version.json',
  ]) {
    assert.equal(
      parseMatrixIpcEnvelope(loadFixture(name)),
      null,
      `${name} must be rejected`
    );
  }
});

test('bounds payload queue streams', () => {
  assert.equal(checkPayloadJsonBounds(0), null);
  assert.equal(checkPayloadJsonBounds(MAX_ENVELOPE_PAYLOAD_JSON_BYTES), null);
  const over = checkPayloadJsonBounds(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 1);
  assert.ok(over);
  assert.equal(over.category, 'sdk_invariant');
  assert.ok(over.diagnosticId?.includes('payload_too_large'));

  assert.equal(checkStreamQueueDepth(0), null);
  assert.equal(checkStreamQueueDepth(MAX_STREAM_QUEUE_DEPTH), null);
  assert.equal(checkStreamQueueDepth(MAX_STREAM_QUEUE_DEPTH + 1)?.category, 'sdk_invariant');

  assert.equal(checkOpenStreams(0), null);
  assert.equal(checkOpenStreams(MAX_OPEN_STREAMS_PER_SESSION), null);
  assert.equal(
    checkOpenStreams(MAX_OPEN_STREAMS_PER_SESSION + 1)?.category,
    'sdk_invariant'
  );
});

test('sequence gap and stale generation compose resync', () => {
  const gap = resyncPayloadForGap('stream-t', 10, 14);
  assert.equal(gap.reason, 'sequence_gap');
  assert.equal(gap.lastAppliedSequence, 10);
  assert.equal(gap.observedSequence, 14);

  const env = makeEnvelope(
    7,
    0,
    { kind: 'resync_required', payload: gap },
    { streamId: 'stream-t' }
  );
  const parsed = parseMatrixIpcEnvelope(JSON.parse(JSON.stringify(env)));
  assert.ok(parsed);
  assert.equal(parsed.kind, 'resync_required');
  if (parsed.kind === 'resync_required') {
    assert.equal(parsed.payload.reason, 'sequence_gap');
  }

  const stale = checkSessionGeneration(3, 1);
  assert.ok(stale);
  assert.equal(stale.category, 'stale_session_generation');
  assert.equal(resyncPayloadForStaleGeneration('stream-t').reason, 'stale_session_generation');

  const { outcome, event } = applyDeltaSequence(9, 4);
  assert.equal(outcome.type, 'behind');
  assert.equal(event, 'resync_needed');
});

test('schema catalog compatible with TS constants', () => {
  const catalog = loadSchemaCatalog();
  assert.equal(catalog.protocolVersion, MATRIX_IPC_PROTOCOL_VERSION);
  assert.deepEqual(catalog.kinds, [...MATRIX_IPC_KINDS]);
  assert.deepEqual(catalog.errorCategories, [...MATRIX_IPC_ERROR_CATEGORIES]);
  assert.equal(catalog.streamTopics.length, 10);
  assert.equal(
    catalog.bounds.maxEnvelopePayloadJsonBytes,
    MAX_ENVELOPE_PAYLOAD_JSON_BYTES
  );
  assert.equal(catalog.bounds.maxStreamQueueDepth, MAX_STREAM_QUEUE_DEPTH);
  assert.equal(catalog.bounds.streamCoalesceWindowMs, STREAM_COALESCE_WINDOW_MS);
  assert.equal(catalog.bounds.maxOpenStreamsPerSession, MAX_OPEN_STREAMS_PER_SESSION);
  assert.equal(catalog.bounds.forbidMediaBytesOverJsonIpc, FORBID_MEDIA_BYTES_OVER_JSON_IPC);
});

test('unknown resync and cancel reasons rejected', () => {
  assert.equal(
    parseMatrixIpcEnvelope({
      protocolVersion: 1,
      sessionGeneration: 1,
      sequence: 0,
      kind: 'resync_required',
      payload: { reason: 'not_a_reason' },
    }),
    null
  );
  assert.equal(
    parseMatrixIpcEnvelope({
      protocolVersion: 1,
      sessionGeneration: 1,
      sequence: 0,
      kind: 'cancel',
      payload: {
        cancellationToken: 't',
        reason: 'not_a_cancel_reason',
      },
    }),
    null
  );
});

test('protocol version zero and future rejected', () => {
  assert.ok(checkProtocolVersion(0));
  assert.ok(checkProtocolVersion(2));
  assert.equal(checkProtocolVersion(MATRIX_IPC_PROTOCOL_VERSION), null);
});

test('stale generation higher and lower both rejected', () => {
  assert.ok(checkSessionGeneration(5, 6));
  assert.ok(checkSessionGeneration(5, 4));
  assert.equal(checkSessionGeneration(5, 5), null);
});

test('error categories count is 21 and exhaustive list is stable', () => {
  assert.equal(MATRIX_IPC_ERROR_CATEGORIES.length, 21);
  assert.equal(parseMatrixIpcError({ category: 'not_real' }), null);
});
