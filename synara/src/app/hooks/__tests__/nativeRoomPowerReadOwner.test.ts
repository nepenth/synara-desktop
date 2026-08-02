import assert from 'node:assert/strict';
import test from 'node:test';
import {
  readRoomPowerLevelsWithNativeOwner,
  type NativeRoomPowerLevelsInvoke,
} from '../nativeRoomPowerLevelsOwner';
import {
  readRoomCreatorsWithNativeOwner,
  type NativeRoomCreatorsInvoke,
} from '../nativeRoomCreatorsOwner';

const roomId = '!room:example.org';
const loggedIn = {
  available: true as const,
  value: { status: 'logged_in', sessionGeneration: 7 },
};

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
  await assert.rejects(readRoomPowerLevelsWithNativeOwner(roomId, true, loggedOut), /unavailable/);

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
});
