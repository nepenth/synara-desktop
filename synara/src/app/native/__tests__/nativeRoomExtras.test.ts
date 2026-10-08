import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { DesktopInvokeResult } from '../../utils/desktop';
import {
  bulkRedact,
  checkAliasAvailability,
  createLocalAlias,
  deleteLocalAlias,
  fetchLocalAliases,
  fetchMutualRooms,
  parseAliasAvailability,
  parseBulkRedact,
  parseRoomUpgrade,
  setNativeRoomExtrasInvokeForTests,
  upgradeRoom,
} from '../nativeRoomExtras';

const ok = (value: unknown): DesktopInvokeResult<unknown> => ({ available: true, value });

type Call = { command: string; args?: Record<string, unknown> };

const record = (replies: Record<string, unknown>) => {
  const calls: Call[] = [];
  setNativeRoomExtrasInvokeForTests(async (command, args) => {
    calls.push({ command, args });
    return ok(replies[command]);
  });
  return calls;
};

test('mutual rooms and local aliases use the native commands and parse the lists', async () => {
  const calls = record({
    matrix_user_mutual_rooms: { sessionGeneration: 1, userId: '@b:x', roomIds: ['!a:x'] },
    matrix_room_local_aliases: { sessionGeneration: 1, roomId: '!a:x', aliases: ['#a:x'] },
    matrix_room_alias_create: null,
    matrix_room_alias_delete: null,
  });
  assert.deepEqual(await fetchMutualRooms('@b:x'), ['!a:x']);
  assert.deepEqual(await fetchLocalAliases('!a:x'), ['#a:x']);
  await createLocalAlias('#new:x', '!a:x');
  await deleteLocalAlias('#old:x');
  assert.deepEqual(calls, [
    { command: 'matrix_user_mutual_rooms', args: { userId: '@b:x' } },
    { command: 'matrix_room_local_aliases', args: { roomId: '!a:x' } },
    { command: 'matrix_room_alias_create', args: { alias: '#new:x', roomId: '!a:x' } },
    { command: 'matrix_room_alias_delete', args: { alias: '#old:x' } },
  ]);
});

test('malformed list replies are errors, not empty lists', async () => {
  record({ matrix_user_mutual_rooms: { roomIds: [1] } });
  await assert.rejects(fetchMutualRooms('@b:x'), /Invalid mutual rooms/);
});

test('alias availability accepts only the closed values', async () => {
  assert.equal(parseAliasAvailability({ alias: '#a:x', availability: 'taken' }), 'taken');
  assert.equal(parseAliasAvailability({ alias: '#a:x', availability: 'unknown' }), undefined);
  record({ matrix_room_alias_check: { alias: '#a:x', availability: 'available' } });
  assert.equal(await checkAliasAvailability('#a:x'), 'available');
});

test('room upgrade returns the replacement and forwards extra creators', async () => {
  assert.equal(parseRoomUpgrade({ roomId: '!a:x', replacementRoomId: 'nope' }), undefined);
  const calls = record({
    matrix_room_upgrade: { roomId: '!a:x', replacementRoomId: '!b:x' },
  });
  assert.equal(await upgradeRoom('!a:x', '12', ['@c:x']), '!b:x');
  assert.equal(await upgradeRoom('!a:x', '11', []), '!b:x');
  assert.deepEqual(calls[0].args, {
    roomId: '!a:x',
    newVersion: '12',
    additionalCreators: ['@c:x'],
  });
  assert.equal(calls[1].args?.additionalCreators, undefined);
});

test('bulk redaction sends an integer timestamp and validates the counts', async () => {
  assert.equal(
    parseBulkRedact({ roomId: '!a:x', scanned: -1, redacted: 0, failed: 0, truncated: false }),
    undefined
  );
  const calls = record({
    matrix_room_bulk_redact: {
      roomId: '!a:x',
      scanned: 40,
      redacted: 3,
      failed: 0,
      truncated: false,
    },
  });
  const result = await bulkRedact({
    roomId: '!a:x',
    userIds: ['@spam:x'],
    sinceTs: 1_700_000_000_000.7,
    reason: 'spam',
  });
  assert.equal(result.redacted, 3);
  assert.deepEqual(calls[0].args, {
    roomId: '!a:x',
    userIds: ['@spam:x'],
    sinceTs: 1_700_000_000_000,
    eventTypes: undefined,
    reason: 'spam',
  });
});

test('commands fail closed when the desktop shell is unavailable', async () => {
  setNativeRoomExtrasInvokeForTests(async () => ({ available: false }));
  await assert.rejects(fetchMutualRooms('@b:x'), /unavailable/);
  await assert.rejects(upgradeRoom('!a:x', '11'), /unavailable/);
});
