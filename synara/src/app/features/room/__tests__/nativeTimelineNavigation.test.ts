import assert from 'node:assert/strict';
import test from 'node:test';
import {
  observeRoomLatestAfterSend,
  requestRoomLatestAfterSend,
  roomMessageSentTargetsView,
} from '../nativeTimelineNavigation';

const withWindow = (run: () => void) => {
  const previous = (globalThis as { window?: unknown }).window;
  (globalThis as { window?: unknown }).window = new EventTarget();
  try {
    run();
  } finally {
    (globalThis as { window?: unknown }).window = previous;
  }
};

test('a send targets only the view that shows its timeline', () => {
  const room = '!room:x';
  // Main room send: the room view moves; a thread view (side pane) does not.
  assert.equal(roomMessageSentTargetsView({ roomId: room }, room, undefined), true);
  assert.equal(roomMessageSentTargetsView({ roomId: room }, room, '$root'), false);
  // Thread send: only the view showing that thread moves.
  assert.equal(
    roomMessageSentTargetsView({ roomId: room, threadRootId: '$root' }, room, '$root'),
    true
  );
  assert.equal(
    roomMessageSentTargetsView({ roomId: room, threadRootId: '$root' }, room, undefined),
    false
  );
  assert.equal(
    roomMessageSentTargetsView({ roomId: room, threadRootId: '$root' }, room, '$other'),
    false
  );
  assert.equal(roomMessageSentTargetsView({ roomId: '!other:x' }, room, undefined), false);
});

test('room and thread views each react to their own sends only', () => {
  withWindow(() => {
    const calls: string[] = [];
    const stopRoom = observeRoomLatestAfterSend('!room:x', undefined, () => calls.push('room'));
    const stopPane = observeRoomLatestAfterSend('!room:x', '$root', () => calls.push('pane'));

    requestRoomLatestAfterSend('!room:x');
    requestRoomLatestAfterSend('!room:x', '$root');
    requestRoomLatestAfterSend('!room:x', '$other');
    requestRoomLatestAfterSend('!other:x');
    assert.deepEqual(calls, ['room', 'pane']);

    stopRoom();
    stopPane();
    requestRoomLatestAfterSend('!room:x');
    assert.deepEqual(calls, ['room', 'pane']);
  });
});
