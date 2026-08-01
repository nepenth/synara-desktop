import assert from 'node:assert/strict';
import test from 'node:test';
import { getKnownRoomsFromNativeSnapshot } from '../nativeCallWidgetOwner';

const snapshot = {
  sessionGeneration: 4,
  orderedRoomIds: ['!first:example.org', '!second:example.org'],
  rooms: [],
};

test('call widget known rooms use the native room-list snapshot', () => {
  assert.deepEqual(getKnownRoomsFromNativeSnapshot(true, snapshot), snapshot.orderedRoomIds);
});

test('call widget known rooms fail closed before a native snapshot is available', () => {
  assert.deepEqual(getKnownRoomsFromNativeSnapshot(false, snapshot), []);
  assert.deepEqual(
    getKnownRoomsFromNativeSnapshot(true, { ...snapshot, sessionGeneration: 0 }),
    []
  );
});
