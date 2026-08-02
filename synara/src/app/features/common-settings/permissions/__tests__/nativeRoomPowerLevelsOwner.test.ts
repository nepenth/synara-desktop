import assert from 'node:assert/strict';
import test from 'node:test';
import {
  setRoomPowerLevelTagsWithNativeOwner,
  setRoomPowerLevelsWithNativeOwner,
  type NativeRoomPowerLevelsInvoke,
} from '../nativeRoomPowerLevelsOwner';

const completeRoomPolicy = {
  ban: 50,
  historical: 0,
  users_default: 0,
  users: {
    '@alice:example.org': 100,
    '@bob:example.org': 25,
  },
  events_default: 0,
  events: {
    'm.room.name': 50,
    'm.room.power_levels': 100,
  },
  notifications: { room: 50 },
  retained_unknown_policy: { keep: ['this', 'value'] },
};

const completeTags = {
  '100': {
    name: 'Admin',
    color: '#0088ff',
    icon: {
      key: 'mxc://example.org/admin',
      info: {
        w: 32,
        h: 32,
        mimetype: 'image/png',
        size: 512,
        'xyz.amorgan.blurhash': 'LEHV6nWB2yk8pyo0adR*.7kCMdnj',
      },
    },
  },
};

const loggedIn = { available: true as const, value: { status: 'logged_in', sessionGeneration: 7 } };

test('room power levels owner sends one complete policy and validates readback', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativeRoomPowerLevelsInvoke = async (command, args) => {
    calls.push({ command, args });
    if (command === 'matrix_session_snapshot') return loggedIn;
    return {
      available: true,
      value: {
        status: 'ok',
        roomId: '!room:example.org',
        eventType: 'm.room.power_levels',
        stateKey: '',
        sessionGeneration: 7,
        content: args?.content,
      },
    };
  };

  const result = await setRoomPowerLevelsWithNativeOwner(
    '!room:example.org',
    completeRoomPolicy,
    true,
    invoke
  );

  assert.deepEqual(result.content, completeRoomPolicy);
  assert.deepEqual(calls, [
    { command: 'matrix_session_snapshot', args: undefined },
    {
      command: 'matrix_room_set_power_levels',
      args: { roomId: '!room:example.org', content: completeRoomPolicy },
    },
  ]);
});

test('tag owner preserves icon metadata and supports create, edit, delete, and empty maps', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativeRoomPowerLevelsInvoke = async (command, args) => {
    calls.push({ command, args });
    if (command === 'matrix_session_snapshot') return loggedIn;
    return {
      available: true,
      value: {
        status: 'ok',
        roomId: '!room:example.org',
        eventType: 'in.synara.room.power_level_tags',
        stateKey: '',
        sessionGeneration: 7,
        content: args?.content,
      },
    };
  };

  await setRoomPowerLevelTagsWithNativeOwner('!room:example.org', completeTags, true, invoke);
  await setRoomPowerLevelTagsWithNativeOwner(
    '!room:example.org',
    { '100': { name: 'Owner', color: '#ff0000' } },
    true,
    invoke
  );
  await setRoomPowerLevelTagsWithNativeOwner('!room:example.org', {}, true, invoke);

  assert.deepEqual(
    calls.filter(({ command }) => command !== 'matrix_session_snapshot').map(({ args }) => args),
    [
      { roomId: '!room:example.org', content: completeTags },
      { roomId: '!room:example.org', content: { '100': { name: 'Owner', color: '#ff0000' } } },
      { roomId: '!room:example.org', content: {} },
    ]
  );
});

test('power-level owner is fail-closed before any native write on unavailable desktop or invalid content', async () => {
  const calls: string[] = [];
  const invoke: NativeRoomPowerLevelsInvoke = async (command) => {
    calls.push(command);
    return loggedIn;
  };

  await assert.rejects(
    setRoomPowerLevelsWithNativeOwner('!room:example.org', completeRoomPolicy, false, invoke),
    /unavailable/i
  );
  await assert.rejects(
    setRoomPowerLevelsWithNativeOwner(
      '!room:example.org',
      { ...completeRoomPolicy, users: { '@alice:example.org': 1.5 } },
      true,
      invoke
    ),
    /unavailable/i
  );
  assert.deepEqual(calls, []);
});

test('power-level owner rejects logged-out, missing, malformed, mismatched, and stale native results', async () => {
  const loggedOut: NativeRoomPowerLevelsInvoke = async () => ({
    available: true,
    value: { status: 'logged_out' },
  });
  await assert.rejects(
    setRoomPowerLevelsWithNativeOwner('!room:example.org', completeRoomPolicy, true, loggedOut),
    /unavailable/i
  );

  const missingCommand: NativeRoomPowerLevelsInvoke = async (command) =>
    command === 'matrix_session_snapshot' ? loggedIn : { available: false };
  await assert.rejects(
    setRoomPowerLevelsWithNativeOwner(
      '!room:example.org',
      completeRoomPolicy,
      true,
      missingCommand
    ),
    /unavailable/i
  );

  const malformed: NativeRoomPowerLevelsInvoke = async (command) =>
    command === 'matrix_session_snapshot'
      ? loggedIn
      : { available: true, value: { status: 'ok', content: completeRoomPolicy } };
  await assert.rejects(
    setRoomPowerLevelsWithNativeOwner('!room:example.org', completeRoomPolicy, true, malformed),
    /unavailable/i
  );

  const mismatched: NativeRoomPowerLevelsInvoke = async (command) =>
    command === 'matrix_session_snapshot'
      ? loggedIn
      : {
          available: true,
          value: {
            status: 'ok',
            roomId: '!other:example.org',
            eventType: 'm.room.power_levels',
            stateKey: '',
            sessionGeneration: 7,
            content: completeRoomPolicy,
          },
        };
  await assert.rejects(
    setRoomPowerLevelsWithNativeOwner('!room:example.org', completeRoomPolicy, true, mismatched),
    /unavailable/i
  );

  const stale: NativeRoomPowerLevelsInvoke = async (command) =>
    command === 'matrix_session_snapshot'
      ? loggedIn
      : {
          available: true,
          value: {
            status: 'ok',
            roomId: '!room:example.org',
            eventType: 'm.room.power_levels',
            stateKey: '',
            sessionGeneration: 8,
            content: completeRoomPolicy,
          },
        };
  await assert.rejects(
    setRoomPowerLevelsWithNativeOwner('!room:example.org', completeRoomPolicy, true, stale),
    /unavailable/i
  );
});

test('native invoke rejection is safe and never exposes a legacy writer result', async () => {
  let writes = 0;
  const invoke: NativeRoomPowerLevelsInvoke = async (command) => {
    if (command === 'matrix_session_snapshot') throw new Error('raw sdk failure');
    writes += 1;
    throw new Error('legacy writer must not be called');
  };

  await assert.rejects(
    setRoomPowerLevelTagsWithNativeOwner('!room:example.org', completeTags, true, invoke),
    (error: unknown) =>
      error instanceof Error &&
      error.message === 'Native Matrix room power-level writes are unavailable.'
  );
  assert.equal(writes, 0);
});
