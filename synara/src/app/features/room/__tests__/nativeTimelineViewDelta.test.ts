import assert from 'node:assert/strict';
import test from 'node:test';

import { readFileSync } from 'node:fs';
import {
  applyNativeTimelineViewDelta,
  filterNativeForwardTargets,
  isNativeTimelineEventPinned,
  isNativeTimelineReadbackStale,
  canAcceptNativeTimelineFollowReadback,
  nativeThreadFocusEventId,
  nativeTimelineCommandError,
  editedFormattedBodyForSubmit,
  nativeForwardEncryptionDecision,
  shouldAttachFormattedBody,
  type NativeTimelineViewRow,
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
  pinnedEventIds: ['$already-pinned:example.org'],
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

test('poll and sticker rows preserve Core relation and reaction presentation fields', () => {
  const capabilities = {
    react: true,
    reply: true,
    edit: false,
    redact: true,
    report: true,
    pin: true,
    forward: true,
    vote: false,
    declineCall: false,
  };
  const relations = {
    reply: {
      eventId: '$parent:example.org',
      senderId: '@alice:example.org',
      senderName: 'Alice',
      body: 'Parent body',
    },
    threadRoot: '$root:example.org',
    thread: {
      rootEventId: '$root:example.org',
      replyCount: 2,
      latestEventId: '$latest:example.org',
    },
    reactions: [{ key: '✅', count: 2, own: true }],
  };
  const rows: NativeTimelineViewRow[] = [
    {
      kind: 'poll',
      itemId: 'poll-item',
      eventId: '$poll:example.org',
      senderId: '@alice:example.org',
      senderName: 'Alice',
      originServerTs: 1,
      capabilities,
      question: 'Continue?',
      closed: false,
      maxSelections: 1,
      answers: [{ id: 'yes', text: 'Yes', voteCount: 1, own: true }],
      ...relations,
    },
    {
      kind: 'sticker',
      event: {
        itemId: 'sticker-item',
        eventId: '$sticker:example.org',
        senderId: '@bob:example.org',
        senderName: 'Bob',
        originServerTs: 2,
        capabilities,
      },
      media: { handleId: 'timeline-media:sticker' },
      ...relations,
    },
  ];

  for (const row of rows) {
    if (row.kind !== 'poll' && row.kind !== 'sticker') assert.fail('unexpected row kind');
    assert.equal(row.reply?.eventId, '$parent:example.org');
    assert.equal(row.threadRoot, '$root:example.org');
    assert.equal(row.thread?.latestEventId, '$latest:example.org');
    assert.deepEqual(row.reactions, [{ key: '✅', count: 2, own: true }]);
  }
});

test('applies pagination and pin-list metadata and rejects empty batches', () => {
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

  const pinned = applyNativeTimelineViewDelta(baseSnapshot(), {
    schemaVersion: 1,
    sessionGeneration: 2,
    streamId: 'live:!room:example.org:1',
    roomId: '!room:example.org',
    revision: 4,
    ops: [],
    pinnedEventIds: ['$new-pin:example.org'],
  });
  assert.ok(pinned);
  assert.deepEqual(pinned.pinnedEventIds, ['$new-pin:example.org']);

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

test('native command rejections surface the native diagnostic (no paper-over)', () => {
  // Tauri v2 rejects a serialized MatrixAuthCommandError as a plain object;
  // it is never an Error instance, so the catch previously collapsed every
  // native failure to the generic "Native timeline open failed." literal.
  const structured = nativeTimelineCommandError({
    code: 'Unknown',
    message: 'The native Matrix timeline is unavailable.',
    diagnosticId: 'v-timeline-normal-unread-frontier-unavailable',
  });
  assert.equal(structured.message, 'The native Matrix timeline is unavailable.');
  assert.equal(
    (structured as Error & { diagnosticId?: string }).diagnosticId,
    'v-timeline-normal-unread-frontier-unavailable'
  );
  assert.ok(structured instanceof Error);

  const NotFound = nativeTimelineCommandError({
    code: 'NotFound',
    message: 'The native Matrix timeline is not available.',
    diagnosticId: 'd0.3-timeline-room-not-found',
  });
  assert.equal(NotFound.message, 'The native Matrix timeline is not available.');

  const passthrough = new Error('native boom');
  assert.equal(nativeTimelineCommandError(passthrough), passthrough);

  assert.equal(nativeTimelineCommandError('string reject').message, 'string reject');
  assert.equal(nativeTimelineCommandError(undefined).message, 'Native timeline open failed.');
  assert.equal(nativeTimelineCommandError(null).message, 'Native timeline open failed.');
  assert.equal(
    nativeTimelineCommandError({ diagnosticId: 'd0.3-timeline-open-failed' }).message,
    'The native Matrix timeline is unavailable.'
  );
});

test('pin and forward pure helpers gate native presenter UX', () => {
  assert.equal(isNativeTimelineEventPinned(['$a'], '$a'), true);
  assert.equal(isNativeTimelineEventPinned(['$a'], '$b'), false);
  assert.equal(isNativeTimelineEventPinned(undefined, '$a'), false);

  assert.equal(shouldAttachFormattedBody('hello', '<p>hello</p>'), true);
  assert.equal(shouldAttachFormattedBody('hello', 'hello'), false);
  assert.equal(shouldAttachFormattedBody('hello', '  '), false);

  assert.equal(
    nativeThreadFocusEventId({
      rootEventId: '$root',
      replyCount: 2,
      latestEventId: '$latest',
    }),
    '$latest'
  );
  assert.equal(nativeThreadFocusEventId({ rootEventId: '$root', replyCount: 0 }), '$root');

  const targets = filterNativeForwardTargets(
    [
      {
        roomId: '!source:example.org',
        name: 'Source',
        encryptionStatus: 'unknown',
        isSpace: false,
      },
      {
        roomId: '!space:example.org',
        name: 'Space',
        encryptionStatus: 'not_encrypted',
        isSpace: true,
      },
      { roomId: '!target:example.org', name: 'Target room', encryptionStatus: 'not_encrypted' },
      { roomId: '!secure:example.org', name: 'Secure', encryptionStatus: 'encrypted' },
    ],
    '!source:example.org',
    'target'
  );
  assert.equal(targets.length, 1);
  assert.equal(targets[0]?.roomId, '!target:example.org');
  assert.equal(nativeForwardEncryptionDecision('encrypted', 'not_encrypted'), 'confirm_downgrade');
  assert.equal(nativeForwardEncryptionDecision('encrypted', 'encrypted'), 'proceed');
  assert.equal(nativeForwardEncryptionDecision('not_encrypted', 'not_encrypted'), 'proceed');
  assert.equal(nativeForwardEncryptionDecision('not_encrypted', 'encrypted'), 'proceed');
  assert.equal(nativeForwardEncryptionDecision('unknown', 'not_encrypted'), 'unavailable');
  assert.equal(nativeForwardEncryptionDecision('encrypted', 'unknown'), 'unavailable');
  assert.equal(nativeForwardEncryptionDecision(undefined, 'not_encrypted'), 'unavailable');
  assert.equal(nativeForwardEncryptionDecision('encrypted', undefined), 'unavailable');
});

test('editing plain text cannot silently reuse stale Matrix HTML', () => {
  assert.equal(
    editedFormattedBodyForSubmit('old', 'new', '<strong>old</strong>', false),
    undefined
  );
  assert.equal(
    editedFormattedBodyForSubmit('old', 'new', '<strong>new</strong>', true),
    '<strong>new</strong>'
  );
  assert.equal(
    editedFormattedBodyForSubmit('old', 'old', '<strong>old</strong>', false),
    '<strong>old</strong>'
  );
});

test('timeline open is not aborted when event listen is unavailable', () => {
  const source = readFileSync('src/app/features/room/nativeTimelineView.ts', 'utf8');
  assert.match(source, /listenTauriEvent/);
  assert.match(source, /matrix_timeline_open/);
  assert.match(source, /matrix_timeline_snapshot/);
  assert.doesNotMatch(source, /if \(disposed \|\| !unlisten\)/);
});

test('follow-live accepts a placement change at the same SDK revision', () => {
  const unread: NativeTimelineViewSnapshot = {
    ...baseSnapshot(),
    position: { kind: 'unread', anchor_event_id: '$older:example.org' },
  };
  const followed: NativeTimelineViewSnapshot = { ...unread, position: { kind: 'live_bottom' } };
  assert.equal(canAcceptNativeTimelineFollowReadback(unread, followed), true);
  assert.equal(canAcceptNativeTimelineFollowReadback(unread, { ...followed, revision: 2 }), false);
  assert.equal(
    canAcceptNativeTimelineFollowReadback(unread, { ...followed, roomId: '!other:example.org' }),
    false
  );
  assert.equal(
    canAcceptNativeTimelineFollowReadback(unread, { ...followed, sessionGeneration: 3 }),
    false
  );
  assert.equal(canAcceptNativeTimelineFollowReadback(unread, unread), false);
});

test('equal-or-older readbacks are stale for the same stream, not a lost stream', () => {
  const current = baseSnapshot();
  assert.equal(isNativeTimelineReadbackStale(current, { ...current, revision: 2 }), true);
  assert.equal(isNativeTimelineReadbackStale(current, { ...current, revision: 3 }), true);
  assert.equal(isNativeTimelineReadbackStale(current, { ...current, revision: 4 }), false);
  assert.equal(
    isNativeTimelineReadbackStale(current, {
      ...current,
      roomId: '!other:example.org',
      revision: 2,
    }),
    false
  );
  assert.equal(isNativeTimelineReadbackStale(undefined, current), false);
});

test('setReadState keeps the current snapshot when a successful mark_read readback is stale', () => {
  const source = readFileSync('src/app/features/room/nativeTimelineView.ts', 'utf8');
  const setReadState = source
    .split('const setReadState = useCallback(')[1]
    ?.split('const jumpLatest = useCallback')[0];
  assert.ok(setReadState);
  assert.match(setReadState, /isNativeTimelineReadbackStale\(snapshotRef\.current, next\)/);
  assert.match(setReadState, /if \(acceptSnapshot\(next\)\) \{\s*return;/);
  assert.doesNotMatch(
    setReadState,
    /if \(!result\.available \|\| !result\.value \|\| !acceptSnapshot\(result\.value\.snapshot\)\)/
  );
});
