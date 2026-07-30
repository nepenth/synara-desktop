import assert from 'node:assert/strict';
import test from 'node:test';

import {
  applyNativeTimelineViewDelta,
  type NativeTimelineViewSnapshot,
} from '../nativeTimelineView';

const baseSnapshot = (): NativeTimelineViewSnapshot => ({
  schemaVersion: 1,
  sessionGeneration: 2,
  roomId: '!room:example.org',
  revision: 3,
  position: { kind: 'live_bottom' },
  pagination: { backward: 'available', forward: 'available' },
  readState: { isMarkedUnread: true },
  rows: [],
  capabilities: {
    markRead: true,
    markUnread: true,
    paginateBackward: true,
    paginateForward: true,
  },
});

test('applies metadata-only read-frontier deltas without row ops', () => {
  const next = applyNativeTimelineViewDelta(baseSnapshot(), {
    schemaVersion: 1,
    sessionGeneration: 2,
    streamId: 'live:!room:example.org:1',
    roomId: '!room:example.org',
    revision: 4,
    ops: [],
    readState: {
      ownReadEventId: '$frontier:example.org',
      isMarkedUnread: false,
    },
  });
  assert.ok(next);
  assert.equal(next.revision, 4);
  assert.equal(next.readState.ownReadEventId, '$frontier:example.org');
  assert.equal(next.readState.isMarkedUnread, false);
  assert.equal(next.pagination.backward, 'available');
});

test('applies pagination metadata and rejects empty batches', () => {
  const next = applyNativeTimelineViewDelta(baseSnapshot(), {
    schemaVersion: 1,
    sessionGeneration: 2,
    streamId: 'live:!room:example.org:1',
    roomId: '!room:example.org',
    revision: 4,
    ops: [],
    pagination: { backward: 'exhausted', forward: 'available' },
  });
  assert.ok(next);
  assert.equal(next.pagination.backward, 'exhausted');
  assert.equal(
    applyNativeTimelineViewDelta(baseSnapshot(), {
      schemaVersion: 1,
      sessionGeneration: 2,
      streamId: 'live:!room:example.org:1',
      roomId: '!room:example.org',
      revision: 4,
      ops: [],
    }),
    undefined
  );
});
