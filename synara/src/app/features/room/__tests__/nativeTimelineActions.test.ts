import assert from 'node:assert/strict';
import test from 'node:test';

import {
  callDeclineWithNativeTimelineOwner,
  editTextWithNativeTimelineOwner,
  forwardMediaWithNativeTimelineOwner,
  forwardTextWithNativeTimelineOwner,
  isNativeTimelineForwardMedia,
  isNativeTimelineForwardTransport,
  NativePollFlightCoordinator,
  NativeReactionFlightCoordinator,
  pinWithNativeTimelineOwner,
  pollVoteWithNativeTimelineOwner,
  redactWithNativeTimelineOwner,
  reportWithNativeTimelineOwner,
  selectNativeTimelinePinAction,
  nativePollSubmission,
  toggleNativePollSelection,
  unpinWithNativeTimelineOwner,
  type NativeInvoke,
} from '../nativeTimelineActions';

const okInvoke =
  (command: string, value: unknown): NativeInvoke =>
  async (requested) => {
    assert.equal(requested, command);
    return { available: true, value };
  };

test('editTextWithNativeTimelineOwner accepts typed edit readback', async () => {
  const readback = await editTextWithNativeTimelineOwner(
    { roomId: '!room:example.org', eventId: '$one:example.org', body: 'edited' },
    true,
    okInvoke('matrix_timeline_edit_text', {
      schemaVersion: 1,
      action: 'edit_text',
      roomId: '!room:example.org',
      eventId: '$one:example.org',
      status: 'sent',
    })
  );
  assert.notEqual(readback, 'unavailable');
  if (readback === 'unavailable') return;
  assert.equal(readback.eventId, '$one:example.org');
});

test('redact and forward owners reject mismatched action kinds', async () => {
  assert.equal(
    await redactWithNativeTimelineOwner(
      { roomId: '!room:example.org', eventId: '$one:example.org' },
      true,
      okInvoke('matrix_timeline_redact', {
        schemaVersion: 1,
        action: 'edit_text',
        roomId: '!room:example.org',
        eventId: '$one:example.org',
        status: 'redacted',
      })
    ),
    'unavailable'
  );
  const forward = await forwardTextWithNativeTimelineOwner(
    {
      sourceRoomId: '!source:example.org',
      eventId: '$one:example.org',
      targetRoomId: '!target:example.org',
      confirmedEncryptionDowngrade: false,
    },
    true,
    okInvoke('matrix_timeline_forward_text', {
      schemaVersion: 1,
      action: 'forward_text',
      roomId: '!target:example.org',
      eventId: '$fwd:example.org',
      status: 'sent',
    })
  );
  assert.notEqual(forward, 'unavailable');
});

test('forwardMediaWithNativeTimelineOwner accepts typed media forward readback', async () => {
  const forward = await forwardMediaWithNativeTimelineOwner(
    {
      sourceRoomId: '!source:example.org',
      eventId: '$media:example.org',
      targetRoomId: '!target:example.org',
      confirmedEncryptionDowngrade: true,
    },
    true,
    okInvoke('matrix_timeline_forward_media', {
      schemaVersion: 1,
      action: 'forward_media',
      roomId: '!target:example.org',
      eventId: '$fwd:example.org',
      status: 'sent',
    })
  );
  assert.notEqual(forward, 'unavailable');
  if (forward === 'unavailable') return;
  assert.equal(forward.action, 'forward_media');
  assert.equal(forward.eventId, '$fwd:example.org');
});

test('native timeline action owners stay unavailable off desktop', async () => {
  const invoke: NativeInvoke = async () => {
    throw new Error('should not invoke');
  };
  assert.equal(
    await editTextWithNativeTimelineOwner(
      { roomId: '!room:example.org', eventId: '$one:example.org', body: 'x' },
      false,
      invoke
    ),
    'unavailable'
  );
});

test('report and pin owners accept typed readback statuses', async () => {
  const reported = await reportWithNativeTimelineOwner(
    { roomId: '!room:example.org', eventId: '$one:example.org', reason: 'spam' },
    true,
    okInvoke('matrix_timeline_report', {
      schemaVersion: 1,
      action: 'report',
      roomId: '!room:example.org',
      eventId: '$one:example.org',
      status: 'reported',
    })
  );
  assert.notEqual(reported, 'unavailable');
  if (reported !== 'unavailable') assert.equal(reported.status, 'reported');

  const pinned = await pinWithNativeTimelineOwner(
    { roomId: '!room:example.org', eventId: '$one:example.org' },
    true,
    okInvoke('matrix_timeline_pin', {
      schemaVersion: 1,
      action: 'pin',
      roomId: '!room:example.org',
      eventId: '$one:example.org',
      status: 'already_pinned',
    })
  );
  assert.notEqual(pinned, 'unavailable');
  if (pinned !== 'unavailable') assert.equal(pinned.status, 'already_pinned');
});

test('poll vote and call decline owners accept typed readback', async () => {
  const voted = await pollVoteWithNativeTimelineOwner(
    { roomId: '!room:example.org', eventId: '$poll:example.org', answerIds: ['a1'] },
    true,
    okInvoke('matrix_timeline_poll_vote', {
      schemaVersion: 1,
      action: 'poll_vote',
      roomId: '!room:example.org',
      eventId: '$poll:example.org',
      status: 'voted',
    })
  );
  assert.notEqual(voted, 'unavailable');
  if (voted !== 'unavailable') assert.equal(voted.status, 'voted');

  const declined = await callDeclineWithNativeTimelineOwner(
    { roomId: '!room:example.org', eventId: '$rtc:example.org' },
    true,
    okInvoke('matrix_timeline_call_decline', {
      schemaVersion: 1,
      action: 'call_decline',
      roomId: '!room:example.org',
      eventId: '$rtc:example.org',
      status: 'declined',
    })
  );
  assert.notEqual(declined, 'unavailable');
  if (declined !== 'unavailable') assert.equal(declined.status, 'declined');
});

test('product action owners reject readback for the wrong room, event, or status', async () => {
  assert.equal(
    await reportWithNativeTimelineOwner(
      { roomId: '!room:example.org', eventId: '$reported:example.org' },
      true,
      okInvoke('matrix_timeline_report', {
        schemaVersion: 1,
        action: 'report',
        roomId: '!other:example.org',
        eventId: '$reported:example.org',
        status: 'reported',
      })
    ),
    'unavailable'
  );
  assert.equal(
    await forwardTextWithNativeTimelineOwner(
      {
        sourceRoomId: '!source:example.org',
        eventId: '$source:example.org',
        targetRoomId: '!target:example.org',
        confirmedEncryptionDowngrade: false,
      },
      true,
      okInvoke('matrix_timeline_forward_text', {
        schemaVersion: 1,
        action: 'forward_text',
        roomId: '!source:example.org',
        eventId: '$forwarded:example.org',
        status: 'sent',
      })
    ),
    'unavailable'
  );
  assert.equal(
    await pollVoteWithNativeTimelineOwner(
      { roomId: '!room:example.org', eventId: '$poll:example.org', answerIds: ['a1'] },
      true,
      okInvoke('matrix_timeline_poll_vote', {
        schemaVersion: 1,
        action: 'poll_vote',
        roomId: '!room:example.org',
        eventId: '$vote:example.org',
        status: 'voted',
      })
    ),
    'unavailable'
  );
  assert.equal(
    await editTextWithNativeTimelineOwner(
      { roomId: '!room:example.org', eventId: '$one:example.org', body: 'edited' },
      true,
      okInvoke('matrix_timeline_edit_text', {
        schemaVersion: 1,
        action: 'edit_text',
        roomId: '!room:example.org',
        eventId: '$other:example.org',
        status: 'sent',
      })
    ),
    'unavailable'
  );
  await assert.rejects(
    callDeclineWithNativeTimelineOwner(
      { roomId: '!room:example.org', eventId: '$call:example.org' },
      true,
      okInvoke('matrix_timeline_call_decline', {
        schemaVersion: 1,
        action: 'call_decline',
        roomId: '!room:example.org',
        eventId: '',
        status: 'declined',
      })
    ),
    /readback did not match/
  );
});

test('unpinWithNativeTimelineOwner accepts already_unpinned status', async () => {
  const unpinned = await unpinWithNativeTimelineOwner(
    { roomId: '!room:example.org', eventId: '$one:example.org' },
    true,
    okInvoke('matrix_timeline_unpin', {
      schemaVersion: 1,
      action: 'unpin',
      roomId: '!room:example.org',
      eventId: '$one:example.org',
      status: 'already_unpinned',
    })
  );
  assert.notEqual(unpinned, 'unavailable');
  if (unpinned !== 'unavailable') assert.equal(unpinned.status, 'already_unpinned');
});

test('isNativeTimelineForwardMedia consumes only the Core-projected transport', () => {
  assert.equal(isNativeTimelineForwardTransport('text'), true);
  assert.equal(isNativeTimelineForwardTransport('media'), true);
  assert.equal(isNativeTimelineForwardTransport('bogus'), false);
  assert.equal(isNativeTimelineForwardTransport(undefined), false);
  assert.equal(isNativeTimelineForwardMedia('media'), true);
  assert.equal(isNativeTimelineForwardMedia('text'), false);
  assert.equal(isNativeTimelineForwardMedia('bogus'), false);
  assert.equal(isNativeTimelineForwardMedia(undefined), false);
});

test('selectNativeTimelinePinAction uses pin-list state not dual buttons', () => {
  assert.equal(selectNativeTimelinePinAction(false), 'pin');
  assert.equal(selectNativeTimelinePinAction(true), 'unpin');
});

test('poll selection honors maximum selections and supports clearing a vote', () => {
  const available = new Set(['a', 'b', 'c']);
  assert.deepEqual([...toggleNativePollSelection(new Set(['a', 'b']), 'c', available, 2)].sort(), [
    'a',
    'b',
  ]);
  assert.deepEqual([...toggleNativePollSelection(new Set(['a']), 'a', available, 1)], []);
  assert.deepEqual(nativePollSubmission(new Set(), new Set(['a']), available, 1, true, false), []);
});

test('poll submission stays capability-driven and rejects invalid presenter state', () => {
  const available = new Set(['a', 'b']);
  assert.equal(
    nativePollSubmission(new Set(['a']), new Set(['a']), available, 1, true, false),
    undefined
  );
  assert.equal(
    nativePollSubmission(new Set(['b']), new Set(['a']), available, 1, false, false),
    undefined
  );
  assert.equal(
    nativePollSubmission(new Set(['b']), new Set(['a']), available, 1, true, true),
    undefined
  );
  assert.equal(
    nativePollSubmission(new Set(['a', 'b']), new Set(), available, 1, true, false),
    undefined
  );
  assert.deepEqual(
    nativePollSubmission(new Set(['b']), new Set(['a']), available, 1, true, false),
    ['b']
  );
});

test('poll flight accepts projection before dispatch readback without releasing early', () => {
  const coordinator = new NativePollFlightCoordinator();
  coordinator.bindSession(10);
  coordinator.prepare('poll', ['b', 'a']);

  assert.equal(coordinator.observeProjection('poll', ['a', 'b']), false);
  assert.equal(coordinator.has('poll'), true);
  assert.equal(coordinator.settleDispatch('poll', true), true);
  assert.equal(coordinator.has('poll'), false);
});

test('poll flight accepts dispatch readback before projection and clears failures', () => {
  const coordinator = new NativePollFlightCoordinator();
  coordinator.bindSession(10);
  coordinator.prepare('poll', ['a']);

  assert.equal(coordinator.settleDispatch('poll', true), false);
  assert.equal(coordinator.observeProjection('poll', ['old']), false);
  assert.equal(coordinator.has('poll'), true);
  assert.equal(coordinator.observeProjection('poll', ['a']), true);
  assert.equal(coordinator.has('poll'), false);

  coordinator.prepare('failed', ['a']);
  assert.equal(coordinator.settleDispatch('failed', false), true);
  assert.equal(coordinator.has('failed'), false);
});

test('poll flight cannot leak across Core session generations', () => {
  const coordinator = new NativePollFlightCoordinator();
  coordinator.bindSession(10);
  coordinator.prepare('poll', ['a']);
  assert.equal(coordinator.has('poll'), true);

  coordinator.bindSession(11);
  assert.equal(coordinator.has('poll'), false);
});

test('reaction flight waits for both command and exact projection in either order', () => {
  const afterDispatch = new NativeReactionFlightCoordinator();
  afterDispatch.bindSession(10);
  afterDispatch.prepare('10\0!room\0$event\0reaction:✅', '✅', true);
  assert.equal(afterDispatch.settleDispatch('10\0!room\0$event\0reaction:✅', true), false);
  assert.deepEqual(afterDispatch.observeEventProjection('10\0!room\0$event\0reaction:', []), []);
  assert.deepEqual(
    afterDispatch.observeEventProjection('10\0!room\0$event\0reaction:', [
      { key: '✅', own: true },
    ]),
    ['10\0!room\0$event\0reaction:✅']
  );

  const beforeDispatch = new NativeReactionFlightCoordinator();
  beforeDispatch.bindSession(10);
  beforeDispatch.prepare('reaction', '✅', false);
  assert.deepEqual(
    beforeDispatch.observeEventProjection('reaction', [{ key: '✅', own: false }]),
    []
  );
  assert.equal(beforeDispatch.has('reaction'), true);
  assert.equal(beforeDispatch.settleDispatch('reaction', true), true);
  assert.equal(beforeDispatch.has('reaction'), false);
});

test('reaction flight clears on failure and session transition', () => {
  const coordinator = new NativeReactionFlightCoordinator();
  coordinator.bindSession(10);
  coordinator.prepare('failed', '✅', true);
  assert.equal(coordinator.settleDispatch('failed', false), true);
  assert.equal(coordinator.has('failed'), false);

  coordinator.prepare('old', '✅', true);
  coordinator.bindSession(11);
  assert.equal(coordinator.has('old'), false);
});
