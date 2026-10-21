import assert from 'node:assert/strict';
import test from 'node:test';
import { nativeTimelineArrivals } from '../nativeTimelineAnnouncements';
import type { NativeTimelineViewRow } from '../nativeTimelineView';

const message = (
  id: string,
  senderId: string,
  body: string,
  messageType = 'text'
): NativeTimelineViewRow =>
  ({
    kind: 'message',
    itemId: id,
    eventId: id,
    senderId,
    senderName: senderId === '@me:x' ? 'Me' : 'Alice',
    originServerTs: 1,
    body,
    messageType,
    capabilities: {},
  }) as NativeTimelineViewRow;
const divider = { kind: 'date_separator', itemId: 'd', timestampMs: 1 } as NativeTimelineViewRow;

test('the first snapshot announces nothing (D58)', () => {
  assert.deepEqual(nativeTimelineArrivals(undefined, [message('$1', '@a:x', 'hi')], '@me:x'), {
    tailEventId: '$1',
  });
});

test('new messages from others are announced, own messages are not', () => {
  const rows = [
    message('$1', '@a:x', 'old'),
    divider,
    message('$2', '@me:x', 'mine'),
    message('$3', '@a:x', 'hello\nthere'),
    message('$4', '@a:x', 'pic', 'image'),
  ];
  assert.deepEqual(nativeTimelineArrivals('$1', rows, '@me:x'), {
    tailEventId: '$4',
    announcement: 'Alice: hello there. Alice sent an image',
  });
  assert.deepEqual(nativeTimelineArrivals('$4', rows, '@me:x'), { tailEventId: '$4' });
  assert.deepEqual(nativeTimelineArrivals('$1', rows.slice(0, 3), '@me:x'), { tailEventId: '$2' });
});

test('a jump to a window without the previous tail announces nothing', () => {
  assert.deepEqual(nativeTimelineArrivals('$gone', [message('$9', '@a:x', 'later')], '@me:x'), {
    tailEventId: '$9',
  });
});

test('a burst announces only the last three, clipped', () => {
  const rows = [message('$0', '@a:x', 'seen')];
  for (let index = 1; index <= 5; index += 1) {
    rows.push(message(`$${index}`, '@a:x', index === 5 ? 'x'.repeat(400) : `m${index}`));
  }
  const { announcement } = nativeTimelineArrivals('$0', rows, '@me:x');
  assert.ok(announcement?.startsWith('Alice: m3. Alice: m4. Alice: x'));
  assert.ok((announcement?.length ?? 0) < 220);
});
