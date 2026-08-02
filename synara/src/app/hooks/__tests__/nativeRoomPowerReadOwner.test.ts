import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  readRoomPowerLevelsWithNativeOwner,
  type NativeRoomPowerLevelsInvoke,
} from '../nativeRoomPowerLevelsOwner';
import {
  readRoomCreatorsWithNativeOwner,
  type NativeRoomCreatorsInvoke,
} from '../nativeRoomCreatorsOwner';
import {
  readRoomPowerLevelTagsWithNativeOwner,
  type NativeRoomPowerLevelTagsInvoke,
} from '../nativeRoomPowerLevelTagsOwner';
import { NATIVE_UNAVAILABLE_POWER_LEVELS } from '../usePowerLevels';
import { NATIVE_UNAVAILABLE_POWER_LEVEL_TAGS } from '../usePowerLevelTags';
import { getRoomPermissionsAPI } from '../useRoomPermissions';
import {
  clearNativeRoomStateProjections,
  getNativeHighestPowerUserId,
  getNativeRoomStateProjection,
  getNativeSpecialUsers,
  publishNativeRoomCreatorsProjection,
  publishNativeRoomPowerLevelsProjection,
} from '../../features/matrix-dto/nativeRoomStateProjection';

const roomId = '!room:example.org';
const loggedIn = {
  available: true as const,
  value: { status: 'logged_in', sessionGeneration: 7 },
};

test('native power loading is explicitly fail-closed, including creator bypasses', () => {
  const permissions = getRoomPermissionsAPI(
    new Set(['@alice:example.org']),
    NATIVE_UNAVAILABLE_POWER_LEVELS
  );

  assert.equal(permissions.event('m.room.message', '@alice:example.org'), false);
  assert.equal(permissions.stateEvent('m.room.name', '@alice:example.org'), false);
  assert.equal(permissions.action('invite', '@alice:example.org'), false);
  assert.equal(permissions.notificationAction('room', '@alice:example.org'), false);
  assert.deepEqual(NATIVE_UNAVAILABLE_POWER_LEVELS, { nativeUnavailable: true });
});

test('lobby header keeps native power loading fail-closed', () => {
  const source = readFileSync('src/app/features/lobby/Lobby.tsx', 'utf8');

  assert.match(source, /<LobbyHeader\s+showProfile=\{!onTop\}\s+powerLevels=\{spacePowerLevels\}/);
  assert.doesNotMatch(source, /roomsPowerLevels\.get\(space\.roomId\)\s*\?\?\s*\{\}/);
});

test('power-level tags never read the JS state-event backend on native sessions', () => {
  const source = readFileSync('src/app/hooks/usePowerLevelTags.ts', 'utf8');

  assert.match(source, /const nativeSession = isNativeMatrixSession\(\);/);
  assert.match(source, /useStateEvent\(room, StateEvent\.PowerLevelTags, '', !nativeSession\)/);
  assert.match(source, /readRoomPowerLevelTagsWithNativeOwner/);
  assert.match(source, /NATIVE_UNAVAILABLE_POWER_LEVEL_TAGS/);
  assert.doesNotMatch(source, /const content = nativeSession \? undefined/);
});

test('native power-level read owner validates the session and exact snapshot contract', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativeRoomPowerLevelsInvoke = async (command, args) => {
    calls.push({ command, args });
    if (command === 'matrix_session_snapshot') return loggedIn;
    return {
      available: true,
      value: {
        status: 'ok',
        roomId,
        eventType: 'm.room.power_levels',
        stateKey: '',
        sessionGeneration: 7,
        content: {
          users_default: 0,
          users: { '@alice:example.org': 100 },
          retained: { value: true },
        },
      },
    };
  };

  const snapshot = await readRoomPowerLevelsWithNativeOwner(roomId, true, invoke);
  assert.deepEqual(snapshot?.content, {
    users_default: 0,
    users: { '@alice:example.org': 100 },
    retained: { value: true },
  });
  assert.deepEqual(calls, [
    { command: 'matrix_session_snapshot', args: undefined },
    { command: 'matrix_room_power_levels_snapshot', args: { roomId } },
  ]);
});

test('native creator read owner validates the fixed event and creator IDs', async () => {
  const calls: string[] = [];
  const invoke: NativeRoomCreatorsInvoke = async (command) => {
    calls.push(command);
    if (command === 'matrix_session_snapshot') return loggedIn;
    return {
      available: true,
      value: {
        status: 'ok',
        roomId,
        eventType: 'm.room.create',
        stateKey: '',
        sessionGeneration: 7,
        creators: ['@alice:example.org', '@bob:example.org'],
      },
    };
  };

  const snapshot = await readRoomCreatorsWithNativeOwner(roomId, true, invoke);
  assert.deepEqual(snapshot?.creators, ['@alice:example.org', '@bob:example.org']);
  assert.deepEqual(calls, ['matrix_session_snapshot', 'matrix_room_creators_snapshot']);
});

test('native power-level tag read owner preserves custom tag metadata', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativeRoomPowerLevelTagsInvoke = async (command, args) => {
    calls.push({ command, args });
    if (command === 'matrix_session_snapshot') return loggedIn;
    return {
      available: true,
      value: {
        status: 'ok',
        roomId,
        eventType: 'in.synara.room.power_level_tags',
        stateKey: '',
        sessionGeneration: 7,
        content: {
          '100': {
            name: 'Admin',
            color: '#0088ff',
            icon: { key: 'mxc://example.org/admin', info: { w: 32, h: 32 } },
          },
        },
      },
    };
  };

  const snapshot = await readRoomPowerLevelTagsWithNativeOwner(roomId, true, invoke);
  assert.deepEqual(snapshot?.content, {
    '100': {
      name: 'Admin',
      color: '#0088ff',
      icon: { key: 'mxc://example.org/admin', info: { w: 32, h: 32 } },
    },
  });
  assert.deepEqual(calls, [
    { command: 'matrix_session_snapshot', args: undefined },
    { command: 'matrix_room_power_level_tags_snapshot', args: { roomId } },
  ]);
});

test('native read owners fail closed on non-native, logged-out, malformed, and stale results', async () => {
  let calls = 0;
  const unavailableInvoke: NativeRoomPowerLevelsInvoke = async () => {
    calls += 1;
    return { available: false };
  };
  assert.equal(
    await readRoomPowerLevelsWithNativeOwner(roomId, false, unavailableInvoke),
    undefined
  );
  assert.equal(calls, 0);
  await assert.rejects(
    readRoomPowerLevelsWithNativeOwner(roomId, true, unavailableInvoke),
    /unavailable/
  );

  const loggedOut: NativeRoomPowerLevelsInvoke = async () => ({
    available: true,
    value: { status: 'logged_out' },
  });
  publishNativeRoomPowerLevelsProjection(roomId, 7, { users: { '@stale:example.org': 100 } });
  await assert.rejects(readRoomPowerLevelsWithNativeOwner(roomId, true, loggedOut), /unavailable/);
  assert.equal(getNativeRoomStateProjection(roomId), undefined);

  const malformed: NativeRoomCreatorsInvoke = async (command) =>
    command === 'matrix_session_snapshot'
      ? loggedIn
      : {
          available: true,
          value: {
            status: 'ok',
            roomId,
            eventType: 'm.room.create',
            stateKey: '',
            sessionGeneration: 8,
            creators: ['not-a-user'],
          },
        };
  await assert.rejects(readRoomCreatorsWithNativeOwner(roomId, true, malformed), /unavailable/);

  let tagCalls = 0;
  const tagUnavailable: NativeRoomPowerLevelTagsInvoke = async () => {
    tagCalls += 1;
    return { available: false };
  };
  assert.equal(
    await readRoomPowerLevelTagsWithNativeOwner(roomId, false, tagUnavailable),
    undefined
  );
  assert.equal(tagCalls, 0);
  await assert.rejects(
    readRoomPowerLevelTagsWithNativeOwner(roomId, true, tagUnavailable),
    /unavailable/
  );
  assert.deepEqual(NATIVE_UNAVAILABLE_POWER_LEVEL_TAGS, {});

  const malformedTags: NativeRoomPowerLevelTagsInvoke = async (command) =>
    command === 'matrix_session_snapshot'
      ? loggedIn
      : {
          available: true,
          value: {
            status: 'ok',
            roomId,
            eventType: 'in.synara.room.power_level_tags',
            stateKey: '',
            sessionGeneration: 7,
            content: { '100': { name: 42 } },
          },
        };
  await assert.rejects(
    readRoomPowerLevelTagsWithNativeOwner(roomId, true, malformedTags),
    /unavailable/
  );
});

test('native room read failures invalidate old projections after a logged-in generation change', async () => {
  clearNativeRoomStateProjections();
  publishNativeRoomPowerLevelsProjection(roomId, 7, {
    users: { '@old:example.org': 100 },
  });
  assert.equal(getNativeRoomStateProjection(roomId)?.sessionGeneration, 7);

  const unavailable: NativeRoomPowerLevelsInvoke = async (command) =>
    command === 'matrix_session_snapshot'
      ? { available: true, value: { status: 'logged_in', sessionGeneration: 8 } }
      : { available: false };
  await assert.rejects(
    readRoomPowerLevelsWithNativeOwner(roomId, true, unavailable),
    /unavailable/
  );
  assert.equal(getNativeRoomStateProjection(roomId), undefined);

  publishNativeRoomCreatorsProjection(roomId, 8, ['@new:example.org']);
  const malformed: NativeRoomCreatorsInvoke = async (command) =>
    command === 'matrix_session_snapshot'
      ? { available: true, value: { status: 'logged_in', sessionGeneration: 8 } }
      : { available: true, value: { status: 'ok', roomId, creators: ['not-a-user'] } };
  await assert.rejects(readRoomCreatorsWithNativeOwner(roomId, true, malformed), /unavailable/);
  assert.equal(getNativeRoomStateProjection(roomId), undefined);

  publishNativeRoomPowerLevelsProjection(roomId, 8, {
    users: { '@new:example.org': 100 },
  });
  const stale: NativeRoomPowerLevelsInvoke = async (command) =>
    command === 'matrix_session_snapshot'
      ? { available: true, value: { status: 'logged_in', sessionGeneration: 8 } }
      : {
          available: true,
          value: {
            status: 'ok',
            roomId,
            eventType: 'm.room.power_levels',
            stateKey: '',
            sessionGeneration: 7,
            content: { users: { '@old:example.org': 100 } },
          },
        };
  await assert.rejects(readRoomPowerLevelsWithNativeOwner(roomId, true, stale), /unavailable/);
  assert.equal(getNativeRoomStateProjection(roomId), undefined);

  clearNativeRoomStateProjections();
});

test('native direct-reader projections consume DTOs and fail closed without them', () => {
  clearNativeRoomStateProjections();
  assert.equal(getNativeHighestPowerUserId(undefined), undefined);
  assert.deepEqual(getNativeSpecialUsers(undefined), []);

  publishNativeRoomCreatorsProjection(roomId, 7, ['@creator:example.org']);
  assert.equal(
    getNativeHighestPowerUserId(getNativeRoomStateProjection(roomId)),
    '@creator:example.org'
  );

  publishNativeRoomPowerLevelsProjection(roomId, 7, {
    users_default: 0,
    users: {
      '@creator:example.org': 100,
      '@moderator:example.org': 50,
    },
  });
  assert.deepEqual(getNativeSpecialUsers(getNativeRoomStateProjection(roomId)), [
    '@creator:example.org',
    '@moderator:example.org',
  ]);

  publishNativeRoomPowerLevelsProjection('!new-room:example.org', 8, {
    users: { '@new:example.org': 100 },
  });
  assert.equal(getNativeRoomStateProjection(roomId), undefined);
  assert.deepEqual(getNativeSpecialUsers(getNativeRoomStateProjection(roomId)), []);
  clearNativeRoomStateProjections();
});

test('native direct readers do not reopen getStateEvent for create or power data', () => {
  const viaServers = readFileSync('src/app/plugins/via-servers.ts', 'utf8');
  const viaNativeBranch = viaServers.match(/if \(isNativeMatrixSession\(\)\) \{([\s\S]*?)\n\s*\}/);
  assert.ok(viaNativeBranch, 'expected an explicit native via-server branch');
  assert.doesNotMatch(viaNativeBranch[1], /getStateEvent/);

  const roomUtils = readFileSync('src/app/utils/room.ts', 'utf8');
  const creatorNativeBranch = roomUtils.match(
    /export const getAllVersionsRoomCreator[\s\S]*?if \(isNativeMatrixSession\(\)\) \{([\s\S]*?)\n\s*\}/
  );
  assert.ok(creatorNativeBranch, 'expected an explicit native creator branch');
  assert.doesNotMatch(creatorNativeBranch[1], /getStateEvent/);

  const parentNativeBranch = roomUtils.match(
    /const getSpecialUsers = \(rId: string\): string\[\] => \{[\s\S]*?if \(isNativeMatrixSession\(\)\) \{([\s\S]*?)\n\s*\}/
  );
  assert.ok(parentNativeBranch, 'expected an explicit native perfect-parent branch');
  assert.doesNotMatch(parentNativeBranch[1], /getStateEvent/);
});
