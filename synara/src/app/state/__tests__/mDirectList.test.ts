import assert from 'node:assert/strict';
import test from 'node:test';
import { mDirectRoomsFromNativeSnapshot } from '../mDirectList';

test('native m.direct projection builds room-id set', () => {
  const rooms = mDirectRoomsFromNativeSnapshot([
    '!dm:example.org',
    '!other:example.org',
    '!dm:example.org',
  ]);
  assert.equal(rooms.size, 2);
  assert.ok(rooms.has('!dm:example.org'));
  assert.ok(rooms.has('!other:example.org'));
});
