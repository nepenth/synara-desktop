import assert from 'node:assert/strict';
import test from 'node:test';

import {
  NATIVE_PINNED_EVENTS_SCHEMA_VERSION,
  pinnedEventCount,
  pinnedEventsWithNativeOwner,
  type NativePinnedEventsSnapshot,
} from '../nativePinnedEvents';

const okInvoke = (command: string, value: unknown) => async (requested: string) => {
  assert.equal(requested, command);
  return { available: true, value };
};

test('pinnedEventsWithNativeOwner accepts a typed pin-panel snapshot', async () => {
  const snapshot: NativePinnedEventsSnapshot = {
    schemaVersion: NATIVE_PINNED_EVENTS_SCHEMA_VERSION,
    roomId: '!room:example.org',
    eventIds: ['$one:example.org', '$two:example.org'],
    items: [
      {
        eventId: '$one:example.org',
        senderId: '@bob:example.org',
        originServerTs: 1,
        eventType: 'm.room.message',
        body: 'pinned one',
        reactions: [
          {
            key: '👍',
            count: 1,
            senders: [{ userId: '@bob:example.org', reactionEventId: '$react' }],
          },
        ],
      },
    ],
  };
  const result = await pinnedEventsWithNativeOwner(
    '!room:example.org',
    true,
    okInvoke('matrix_pinned_events', snapshot)
  );
  assert.notEqual(result, 'unavailable');
  if (result === 'unavailable') return;
  assert.equal(result.eventIds.length, 2);
  assert.equal(result.items[0].reactions?.[0].senders?.[0].reactionEventId, '$react');
});

test('pinnedEventsWithNativeOwner rejects a mismatched room', async () => {
  assert.equal(
    await pinnedEventsWithNativeOwner(
      '!room:example.org',
      true,
      okInvoke('matrix_pinned_events', {
        schemaVersion: NATIVE_PINNED_EVENTS_SCHEMA_VERSION,
        roomId: '!other:example.org',
        eventIds: [],
        items: [],
      })
    ),
    'unavailable'
  );
});

test('pinnedEventCount prefers native snapshot ids', () => {
  assert.equal(
    pinnedEventCount(
      {
        available: true,
        loading: false,
        snapshot: {
          schemaVersion: 1,
          roomId: '!room:example.org',
          eventIds: ['$a', '$b'],
          items: [],
        },
      },
      ['$js-only']
    ),
    2
  );
  assert.equal(pinnedEventCount({ available: false, loading: false, snapshot: null }, ['$js']), 1);
});
