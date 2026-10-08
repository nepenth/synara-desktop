import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { parseRoomIdentityWarnings, roomIdentityBanner } from '../nativeRoomIdentityWarnings';

const ROOM = '!room:example.org';

test('readbacks for another room or with malformed rows are ignored', () => {
  assert.deepEqual(parseRoomIdentityWarnings(ROOM, undefined), []);
  assert.deepEqual(
    parseRoomIdentityWarnings(ROOM, {
      roomId: '!other:example.org',
      warnings: [{ userId: '@bob:example.org', kind: 'pin_violation' }],
    }),
    []
  );
  assert.deepEqual(
    parseRoomIdentityWarnings(ROOM, {
      roomId: ROOM,
      warnings: [
        { userId: 'bob', kind: 'pin_violation' },
        { userId: '@eve:example.org', kind: 'verified' },
        { userId: '@amy:example.org', kind: 'pin_violation', displayName: '  ' },
      ],
    }),
    [{ userId: '@amy:example.org', displayName: undefined, kind: 'pin_violation' }]
  );
});

test('no warnings means no banner', () => {
  assert.equal(roomIdentityBanner([]), undefined);
});

test('a changed identity offers Dismiss and names the member', () => {
  const banner = roomIdentityBanner([
    { userId: '@bob:example.org', displayName: 'Bob', kind: 'pin_violation' },
  ]);
  assert.equal(banner?.message, "Bob's identity changed.");
  assert.equal(banner?.action, 'dismiss');
  assert.equal(banner?.actionLabel, 'Dismiss');
});

test('a verification violation wins and offers Withdraw verification', () => {
  const banner = roomIdentityBanner([
    { userId: '@amy:example.org', kind: 'pin_violation' },
    { userId: '@bob:example.org', kind: 'verification_violation' },
  ]);
  assert.equal(banner?.userId, '@bob:example.org');
  assert.equal(banner?.message, "@bob:example.org's verified identity changed.");
  assert.equal(banner?.action, 'withdraw_verification');
  assert.equal(banner?.actionLabel, 'Withdraw verification');
});

test('the banner sits above the composer and reports failures', () => {
  const view = readFileSync('src/app/features/room/RoomView.tsx', 'utf8');
  assert.ok(
    view.indexOf('<RoomIdentityWarningBanner roomId={roomId} />') < view.indexOf('<RoomInput room')
  );
  const banner = readFileSync('src/app/features/room/RoomIdentityWarningBanner.tsx', 'utf8');
  assert.match(banner, /Couldn't update this identity change\./);
  assert.match(banner, /disabled=\{pending\}/);
});
