import assert from 'node:assert/strict';
import test from 'node:test';

import {
  editTextWithNativeTimelineOwner,
  forwardTextWithNativeTimelineOwner,
  pinWithNativeTimelineOwner,
  redactWithNativeTimelineOwner,
  reportWithNativeTimelineOwner,
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
