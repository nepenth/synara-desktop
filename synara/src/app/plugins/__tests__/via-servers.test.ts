import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { getViaServers, getViaServersForMembers, type ViaServerRoom } from '../via-servers';
import {
  clearNativeRoomStateProjections,
  publishNativeRoomCreatorsProjection,
} from '../../features/matrix-dto/nativeRoomStateProjection';
import {
  resetSessionBootstrapForTests,
  setSessionBootstrapResult,
} from '../../state/sessionBootstrap';

const roomId = '!room:example.org';

test('via-server aggregation preserves power-first order, population ties, deduplication, and cap', () => {
  assert.deepEqual(
    getViaServersForMembers('@owner:alpha.example', [
      { userId: '@one:alpha.example' },
      { userId: '@two:beta.example' },
      { userId: '@three:beta.example' },
      { userId: '@four:gamma.example' },
      { userId: '@five:delta.example' },
    ]),
    ['alpha.example', 'beta.example', 'gamma.example']
  );
});

test('native via-server population uses the injected native owner and never reads SDK members', async () => {
  const originalWindow = globalThis.window;
  const room = {
    roomId,
    getMembers: () => {
      throw new Error('native via-server route must not read SDK members');
    },
  } as unknown as ViaServerRoom;
  const calls: Array<{ roomId: string; nativeSession: boolean }> = [];

  (globalThis as any).window = { __SYNARA_DESKTOP__: { platform: 'tauri' } };
  setSessionBootstrapResult({
    source: 'native',
    session: {
      baseUrl: 'https://matrix.example.org',
      userId: '@alice:example.org',
      deviceId: 'DEVICE',
    },
  });
  publishNativeRoomCreatorsProjection(roomId, 7, ['@owner:power.example']);

  try {
    const via = await getViaServers(room, async (requestedRoomId, nativeSession) => {
      calls.push({ roomId: requestedRoomId, nativeSession });
      return [
        { userId: '@one:alpha.example' },
        { userId: '@two:alpha.example' },
        { userId: '@three:beta.example' },
        { userId: '@four:gamma.example' },
      ];
    });

    assert.deepEqual(via, ['power.example', 'alpha.example', 'beta.example']);
    assert.deepEqual(calls, [{ roomId, nativeSession: true }]);
  } finally {
    clearNativeRoomStateProjections();
    resetSessionBootstrapForTests();
    (globalThis as any).window = originalWindow;
  }
});

test('native via-server member failure is terminal and has no web fallback', async () => {
  const originalWindow = globalThis.window;
  let sdkReads = 0;
  const room = {
    roomId,
    getMembers: () => {
      sdkReads += 1;
      return [];
    },
  } as unknown as ViaServerRoom;

  (globalThis as any).window = { __SYNARA_DESKTOP__: { platform: 'tauri' } };
  setSessionBootstrapResult({ source: 'native' });

  try {
    await assert.rejects(
      getViaServers(room, async () => undefined),
      /Native Matrix room members are unavailable/
    );
    assert.equal(sdkReads, 0);
  } finally {
    resetSessionBootstrapForTests();
    (globalThis as any).window = originalWindow;
  }
});

test('via-server owner and all callers keep the native read before effects', () => {
  const viaServers = readFileSync('src/app/plugins/via-servers.ts', 'utf8');
  assert.doesNotMatch(viaServers, /from ['"]matrix-js-sdk['"]/);
  assert.match(viaServers, /readRoomMembersWithNativeOwner/);
  const nativeBranch = viaServers.match(/if \(nativeSession\) \{([\s\S]*?)\n\s*\} else \{/);
  assert.ok(nativeBranch, 'expected explicit native and web member branches');
  assert.match(nativeBranch[1], /await readNativeMembers\(room\.roomId, true\)/);
  assert.doesNotMatch(nativeBranch[1], /getMembers|getJoinedMembers|catch|\[\]/);
  assert.match(viaServers, /members = room\.getMembers\(\)\?\.map/);

  const awaitedCallers = [
    ['src/app/pages/client/space/Space.tsx', /await getViaServers\(room\)/],
    ['src/app/pages/client/space/Space.tsx', /const via = await getViaServers\(currentRoom\)/],
    ['src/app/pages/client/sidebar/SpaceTabs.tsx', /await getViaServers\(room\)/],
    ['src/app/features/add-existing/AddExisting.tsx', /via: await getViaServers\(room\)/],
    ['src/app/features/room-nav/RoomNavItem.tsx', /await getViaServers\(room\)/],
    [
      'src/app/components/editor/autocomplete/RoomMentionAutocomplete.tsx',
      /await getViaServers\(mentionRoom\)/,
    ],
    ['src/app/features/room/RoomTombstone.tsx', /const via = await getViaServers\(currentRoom\)/],
    ['src/app/features/room/message/Message.tsx', /const viaServers = await getViaServers\(room\)/],
    ['src/app/features/room/RoomViewHeader.tsx', /await getViaServers\(room\)/],
  ] as const;

  for (const [path, pattern] of awaitedCallers) {
    assert.match(readFileSync(path, 'utf8'), pattern, `${path} must await getViaServers`);
  }

  const addExisting = readFileSync('src/app/features/add-existing/AddExisting.tsx', 'utf8');
  assert.match(addExisting, /const roomsWithVia = await Promise\.all/);
  assert.match(addExisting, /rateLimitedActions\(roomsWithVia/);

  for (const path of [
    'src/app/pages/client/space/Space.tsx',
    'src/app/features/room/RoomTombstone.tsx',
  ]) {
    const source = readFileSync(path, 'utf8');
    assert.match(source, /if \(!currentRoom\) throw new Error\('Source room is unavailable\.'\)/);
    assert.doesNotMatch(source, /currentRoom \? getViaServers\(currentRoom\) : \[\]/);
  }
});
