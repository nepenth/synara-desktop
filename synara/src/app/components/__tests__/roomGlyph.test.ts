import assert from 'node:assert/strict';
import test from 'node:test';
import { roomGlyphFor } from '../room-glyph/roomGlyph';

const peer = '@alice:example.test';

test('a DM with an avatar shows the peer avatar', () => {
  assert.deepEqual(
    roomGlyphFor(
      { isDirect: true, directUserId: peer, avatarUrl: 'mxc://example.test/alice' },
      'Alice'
    ),
    { kind: 'person', userId: peer, avatarUrl: 'mxc://example.test/alice', initial: 'A' }
  );
});

test('a DM without an avatar shows the peer initial', () => {
  assert.deepEqual(roomGlyphFor({ isDirect: true, directUserId: peer }, 'Alice'), {
    kind: 'person',
    userId: peer,
    avatarUrl: undefined,
    initial: 'A',
  });
  // The peer's hero name wins over a custom room name; the localpart is last.
  const named = roomGlyphFor(
    { isDirect: true, directUserId: peer, heroes: [{ userId: peer, displayName: 'Zoe' }] },
    'Our plans'
  );
  assert.equal(named.kind === 'person' ? named.initial : undefined, 'Z');
  assert.deepEqual(roomGlyphFor({ isDirect: true, directUserId: peer, avatarUrl: '' }, ''), {
    kind: 'person',
    userId: peer,
    avatarUrl: undefined,
    initial: 'a',
  });
});

test('group and channel rooms keep the channel glyph', () => {
  assert.deepEqual(roomGlyphFor({ isDirect: false, avatarUrl: 'mxc://example.test/room' }, 'Ops'), {
    kind: 'channel',
  });
  assert.deepEqual(roomGlyphFor(undefined, 'Ops'), { kind: 'channel' });
});

test('a DM whose peer Core does not know keeps the channel glyph', () => {
  assert.deepEqual(
    roomGlyphFor(
      { isDirect: true, directUserId: undefined, avatarUrl: 'mxc://example.test/a' },
      'Alice, Bob'
    ),
    { kind: 'channel' }
  );
});
