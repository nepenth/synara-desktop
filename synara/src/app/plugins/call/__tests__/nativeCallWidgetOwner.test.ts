import assert from 'node:assert/strict';
import test from 'node:test';
import {
  getKnownRoomsFromNativeSnapshot,
  throwNativeCallWidgetCapabilityUnavailable,
} from '../nativeCallWidgetOwner';

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

test('call widget media capabilities fail closed without a native owner', () => {
  assert.throws(
    () => throwNativeCallWidgetCapabilityUnavailable('media config'),
    /Native Matrix call widget media config is unavailable/
  );
  assert.throws(
    () => throwNativeCallWidgetCapabilityUnavailable('media download'),
    /Native Matrix call widget media download is unavailable/
  );
});
