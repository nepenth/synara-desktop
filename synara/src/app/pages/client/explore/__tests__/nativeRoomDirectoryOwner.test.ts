import assert from 'node:assert/strict';
import test from 'node:test';

import {
  createNativeRoomDirectoryOwner,
  type NativeRoomDirectoryInvoke,
} from '../nativeRoomDirectoryOwner';

const loggedIn = {
  status: 'logged_in',
  user_id: '@alice:example.org',
  device_id: 'DEVICE',
  homeserver_url: 'https://example.org',
  sessionGeneration: 7,
};

const readyPage = {
  sessionGeneration: 7,
  requestId: 1,
  status: 'ready',
  page: {
    sessionGeneration: 7,
    requestId: 1,
    chunk: [
      {
        roomId: '!room:example.org',
        memberCount: 3,
        worldReadable: true,
        guestCanJoin: true,
        roomType: 'room',
      },
    ],
    nextBatch: 'next',
  },
};

test('directory owner fails closed before native search when unavailable or logged out', async () => {
  const unavailable = createNativeRoomDirectoryOwner(false, async () => ({ available: false }));
  await assert.rejects(() => unavailable.search({ serverName: 'example.org', limit: 24 }));

  const loggedOut: NativeRoomDirectoryInvoke = async (command) =>
    command === 'matrix_session_snapshot'
      ? { available: true, value: { status: 'logged_out' } }
      : { available: false };
  const owner = createNativeRoomDirectoryOwner(true, loggedOut);
  await assert.rejects(() => owner.search({ serverName: 'example.org', limit: 24 }));

  const secretSession = createNativeRoomDirectoryOwner(true, async (command) =>
    command === 'matrix_session_snapshot'
      ? { available: true, value: { ...loggedIn, accessToken: 'secret' } }
      : { available: false }
  );
  await assert.rejects(() => secretSession.search({ serverName: 'example.org', limit: 24 }));
});

test('directory owner uses exact native command arguments and parses the page', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativeRoomDirectoryInvoke = async (command, args) => {
    calls.push({ command, args });
    if (command === 'matrix_session_snapshot') return { available: true, value: loggedIn };
    if (command === 'matrix_room_directory_search') return { available: true, value: readyPage };
    throw new Error('unexpected command');
  };
  const owner = createNativeRoomDirectoryOwner(true, invoke);
  const page = await owner.search({
    serverName: 'example.org',
    term: 'rust',
    roomType: 'space',
    thirdPartyInstanceId: 'irc-example',
    limit: 96,
    since: 'previous',
  });

  assert.equal(page.chunk[0]?.roomId, '!room:example.org');
  assert.deepEqual(calls, [
    { command: 'matrix_session_snapshot', args: undefined },
    {
      command: 'matrix_room_directory_search',
      args: {
        sessionGeneration: 7,
        requestId: 1,
        serverName: 'example.org',
        term: 'rust',
        roomType: 'space',
        thirdPartyInstanceId: 'irc-example',
        limit: 96,
        since: 'previous',
      },
    },
  ]);
});

test('protocol selector uses the native protocol command and rejects malformed DTOs', async () => {
  const calls: string[] = [];
  const invoke: NativeRoomDirectoryInvoke = async (command) => {
    calls.push(command);
    if (command === 'matrix_session_snapshot') return { available: true, value: loggedIn };
    if (command === 'matrix_room_directory_protocols') {
      return {
        available: true,
        value: {
          sessionGeneration: 7,
          instances: [{ protocolId: 'irc', instanceId: 'irc-example', description: 'IRC Example' }],
        },
      };
    }
    throw new Error('unexpected command');
  };
  const owner = createNativeRoomDirectoryOwner(true, invoke);
  const protocols = await owner.getProtocols();
  assert.equal(protocols.instances[0]?.instanceId, 'irc-example');
  assert.deepEqual(calls, ['matrix_session_snapshot', 'matrix_room_directory_protocols']);

  const malformed = createNativeRoomDirectoryOwner(true, async (command) =>
    command === 'matrix_session_snapshot'
      ? { available: true, value: loggedIn }
      : { available: true, value: { sessionGeneration: 7, instances: [{ raw: true }] } }
  );
  await assert.rejects(() => malformed.getProtocols());
});

test('newer request cancels and suppresses the older native result', async () => {
  const calls: string[] = [];
  let resolveFirst: ((value: unknown) => void) | undefined;
  const firstResponse = new Promise<unknown>((resolve) => {
    resolveFirst = resolve;
  });
  const invoke: NativeRoomDirectoryInvoke = async (command, args) => {
    calls.push(command);
    if (command === 'matrix_session_snapshot') return { available: true, value: loggedIn };
    if (command === 'matrix_room_directory_cancel')
      return { available: true, value: { status: 'cancelled' } };
    if (command === 'matrix_room_directory_search' && args?.requestId === 1) {
      return { available: true, value: await firstResponse };
    }
    return {
      available: true,
      value: { ...readyPage, requestId: 2, page: { ...readyPage.page, requestId: 2 } },
    };
  };
  const owner = createNativeRoomDirectoryOwner(true, invoke);
  const first = owner.search({ serverName: 'example.org', limit: 24 });
  await new Promise((resolve) => setImmediate(resolve));
  const second = owner.search({ serverName: 'example.org', limit: 24, term: 'new' });
  const secondPage = await second;
  assert.equal(secondPage.requestId, 2);
  resolveFirst?.(readyPage);
  await assert.rejects(() => first);
  assert.ok(calls.includes('matrix_room_directory_cancel'));
});

test('invalid native status, generation, and room data never become a successful empty page', async () => {
  const owner = createNativeRoomDirectoryOwner(true, async (command) => {
    if (command === 'matrix_session_snapshot') return { available: true, value: loggedIn };
    return {
      available: true,
      value: { sessionGeneration: 8, requestId: 1, status: 'ready', page: { chunk: [] } },
    };
  });
  await assert.rejects(() => owner.search({ serverName: 'example.org', limit: 24 }));
});
