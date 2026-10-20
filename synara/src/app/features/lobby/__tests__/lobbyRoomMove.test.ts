import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { runBulkAction } from '../../../utils/bulkActions';
import { moveLobbyRoom, type LobbyRoomMoveDeps, type LobbyRoomMovePlan } from '../lobbyRoomMove';

const plan = (overrides: Partial<LobbyRoomMovePlan> = {}): LobbyRoomMovePlan => ({
  roomId: '!room:example.org',
  roomName: 'General',
  fromParentId: '!old:example.org',
  toParentId: '!new:example.org',
  toParentName: 'Engineering',
  restricted: false,
  movedContent: { via: ['example.org'] },
  reorders: [
    { roomId: '!room:example.org', content: { via: ['example.org'], order: 'b' } },
    { roomId: '!sibling:example.org', content: { via: ['example.org'], order: 'c' } },
  ],
  ...overrides,
});

const recordingDeps = (fail: Partial<Record<string, string>> = {}) => {
  const calls: string[] = [];
  const deps: LobbyRoomMoveDeps = {
    setSpaceChild: async (parentId, childId) => {
      const call = `set ${parentId} ${childId}`;
      calls.push(call);
      if (fail[call]) throw { code: 'failed', message: fail[call] };
    },
    removeSpaceChild: async (parentId, childId) => {
      const call = `remove ${parentId} ${childId}`;
      calls.push(call);
      if (fail[call]) throw { code: 'failed', message: fail[call] };
    },
    reparentRestrictedJoin: async (roomId, from, to) => {
      const call = `reparent ${roomId} ${from} ${to}`;
      calls.push(call);
      if (fail[call]) throw { code: 'failed', message: fail[call] };
    },
    runWrites: (writes, write) => runBulkAction(writes, write),
  };
  return { calls, deps };
};

test('a move adds the room to the new space before removing it from the old one', async () => {
  const { calls, deps } = recordingDeps();
  await moveLobbyRoom(plan({ restricted: true }), deps);
  assert.deepEqual(calls, [
    'set !new:example.org !room:example.org',
    'reparent !room:example.org !old:example.org !new:example.org',
    'remove !old:example.org !room:example.org',
    'set !new:example.org !sibling:example.org',
  ]);
});

test('a failed add leaves the room in its original space and says so', async () => {
  const { calls, deps } = recordingDeps({ 'set !new:example.org !room:example.org': 'nope' });
  await assert.rejects(
    () => moveLobbyRoom(plan(), deps),
    /Couldn't move General to Engineering\. It is still in its original space/
  );
  assert.deepEqual(calls, ['set !new:example.org !room:example.org']);
});

test('a failed removal reports that the room is now in both spaces', async () => {
  const { deps } = recordingDeps({ 'remove !old:example.org !room:example.org': 'nope' });
  await assert.rejects(() => moveLobbyRoom(plan(), deps), /now appears in both/);
});

test('a failed join-rule update keeps the old parent', async () => {
  const { calls, deps } = recordingDeps({
    'reparent !room:example.org !old:example.org !new:example.org': 'nope',
  });
  await assert.rejects(
    () => moveLobbyRoom(plan({ restricted: true }), deps),
    /who can join it couldn't be updated/
  );
  assert.ok(!calls.some((call) => call.startsWith('remove')));
});

test('a move with no new order keys still adds the room before removing it', async () => {
  const { calls, deps } = recordingDeps();
  await moveLobbyRoom(plan({ reorders: [] }), deps);
  assert.deepEqual(calls, [
    'set !new:example.org !room:example.org',
    'remove !old:example.org !room:example.org',
  ]);
});

test('failed sibling order writes are reported, not swallowed', async () => {
  const { deps } = recordingDeps({ 'set !new:example.org !sibling:example.org': 'nope' });
  await assert.rejects(
    () => moveLobbyRoom(plan(), deps),
    /order of 1 other room in Engineering couldn't be saved/
  );
});

test('the lobby shows move, reorder and pin failures', () => {
  const lobby = readFileSync('src/app/features/lobby/Lobby.tsx', 'utf8');
  assert.match(lobby, /await moveLobbyRoom\(/);
  assert.doesNotMatch(lobby, /await removeSpaceChild\(/);
  assert.doesNotMatch(lobby, /\.catch\(\(\) => undefined\)/);
  assert.match(lobby, /role="alert"/);
});
