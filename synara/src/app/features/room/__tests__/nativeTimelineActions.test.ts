import assert from 'node:assert/strict';
import test from 'node:test';

import {
  callDeclineWithNativeTimelineOwner,
  editTextWithNativeTimelineOwner,
  forwardMediaWithNativeTimelineOwner,
  forwardTextWithNativeTimelineOwner,
  isNativeTimelineForwardMedia,
  pinWithNativeTimelineOwner,
  pollVoteWithNativeTimelineOwner,
  redactWithNativeTimelineOwner,
  reportWithNativeTimelineOwner,
  selectNativeTimelinePinAction,
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
      eventId: '$two:example.org',
      status: 'sent',
    })
  );
  assert.notEqual(readback, 'unavailable');
  if (readback === 'unavailable') return;
  assert.equal(readback.eventId, '$two:example.org');
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
      eventId: '$vote:example.org',
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
      eventId: '$decline:example.org',
      status: 'declined',
    })
  );
  assert.notEqual(declined, 'unavailable');
  if (declined !== 'unavailable') assert.equal(declined.status, 'declined');
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

test('isNativeTimelineForwardMedia routes sticker and media rows only', () => {
  assert.equal(isNativeTimelineForwardMedia({ kind: 'sticker' }), true);
  assert.equal(
    isNativeTimelineForwardMedia({ kind: 'message', messageType: 'image', hasMedia: true }),
    true
  );
  assert.equal(
    isNativeTimelineForwardMedia({ kind: 'message', messageType: 'text', hasMedia: false }),
    false
  );
  assert.equal(
    isNativeTimelineForwardMedia({ kind: 'message', messageType: 'image', hasMedia: false }),
    false
  );
  assert.equal(
    isNativeTimelineForwardMedia({ kind: 'message', messageType: 'notice', hasMedia: true }),
    false
  );
});

test('selectNativeTimelinePinAction uses pin-list state not dual buttons', () => {
  assert.equal(selectNativeTimelinePinAction(false), 'pin');
  assert.equal(selectNativeTimelinePinAction(true), 'unpin');
});
