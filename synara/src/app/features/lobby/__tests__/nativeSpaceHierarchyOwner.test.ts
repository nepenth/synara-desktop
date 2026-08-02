import assert from 'node:assert/strict';
import test from 'node:test';
import { toRoomSummaryView } from '../../../components/RoomSummaryLoader';
import {
  isNativeRoomId,
  readSpaceHierarchyRoomWithNativeOwner,
  readSpaceHierarchyWithNativeOwner,
  type NativeInvoke,
} from '../nativeSpaceHierarchyOwner';

const ROOM_ID = '!room:example.org';
const OTHER_ROOM_ID = '!other:example.org';

const loggedInSession = {
  status: 'logged_in',
  user_id: '@alice:example.org',
  device_id: 'DEVICE',
  homeserver_url: 'https://matrix.example.org',
  sessionGeneration: 9,
};

const validRoom = {
  roomId: ROOM_ID,
  name: 'Example room',
  canonicalAlias: '#example:example.org',
  topic: 'A bounded topic',
  avatarUrl: 'mxc://example.org/avatar',
  roomType: 'm.space',
  numJoinedMembers: 2,
  joinRule: 'public',
  worldReadable: true,
  guestCanJoin: false,
};

const validSnapshot = {
  sessionGeneration: 9,
  rooms: [validRoom],
};

const makeInvoke = (
  sessionValue: unknown = loggedInSession,
  hierarchyValue: unknown = validSnapshot,
  options: {
    sessionAvailable?: boolean;
    hierarchyAvailable?: boolean;
    throwOnHierarchy?: boolean;
  } = {}
) => {
  const calls: Array<[string, Record<string, unknown> | undefined]> = [];
  const invoke: NativeInvoke = async (command, args) => {
    calls.push([command, args]);
    if (command === 'matrix_session_snapshot') {
      return options.sessionAvailable === false
        ? { available: false }
        : { available: true, value: sessionValue };
    }
    if (command === 'matrix_space_hierarchy_snapshot') {
      if (options.throwOnHierarchy) throw new Error('command failed');
      return options.hierarchyAvailable === false
        ? { available: false }
        : { available: true, value: hierarchyValue };
    }
    return { available: false };
  };
  return { calls, invoke };
};

test('native hierarchy read rejects non-desktop ownership', async () => {
  const { calls, invoke } = makeInvoke();
  await assert.rejects(
    () => readSpaceHierarchyWithNativeOwner(ROOM_ID, false, invoke),
    /unavailable/
  );
  assert.deepEqual(calls, []);
});

test('native hierarchy read rejects invalid room IDs and aliases before IPC', async () => {
  for (const roomId of ['', '!', 'not-a-room', '#alias:example.org', '#alias']) {
    const { calls, invoke } = makeInvoke();
    await assert.rejects(
      () => readSpaceHierarchyWithNativeOwner(roomId, true, invoke),
      /valid room ID/
    );
    assert.deepEqual(calls, [], roomId);
  }
  assert.equal(isNativeRoomId(ROOM_ID), true);
  assert.equal(isNativeRoomId('#alias:example.org'), false);
});

test('native hierarchy read requires the managed logged-in session', async () => {
  const loggedOut = makeInvoke({ status: 'logged_out' });
  await assert.rejects(
    () => readSpaceHierarchyWithNativeOwner(ROOM_ID, true, loggedOut.invoke),
    /logged-in/
  );

  for (const sessionValue of [
    { status: 'logged_in', sessionGeneration: 9 },
    { ...loggedInSession, access_token: 'secret' },
    { ...loggedInSession, sessionGeneration: 0 },
    { ...loggedInSession, device_id: '' },
  ]) {
    const { invoke } = makeInvoke(sessionValue);
    await assert.rejects(() => readSpaceHierarchyWithNativeOwner(ROOM_ID, true, invoke));
  }
});

test('native hierarchy read invokes only the existing command with exact arguments', async () => {
  const { calls, invoke } = makeInvoke();
  const room = await readSpaceHierarchyRoomWithNativeOwner(ROOM_ID, true, invoke);
  assert.equal(room.roomId, ROOM_ID);
  assert.deepEqual(calls, [
    ['matrix_session_snapshot', undefined],
    ['matrix_space_hierarchy_snapshot', { roomId: ROOM_ID }],
  ]);
});

test(
  'native hierarchy read selects the requested room and maps the native view fields',
  async () => {
    const { invoke } = makeInvoke();
    const room = await readSpaceHierarchyRoomWithNativeOwner(ROOM_ID, true, invoke);
    assert.deepEqual(toRoomSummaryView(room), {
      room_id: ROOM_ID,
      name: 'Example room',
      canonical_alias: '#example:example.org',
      topic: 'A bounded topic',
      avatar_url: 'mxc://example.org/avatar',
      room_type: 'm.space',
      num_joined_members: 2,
      join_rule: 'public',
      world_readable: true,
      guest_can_join: false,
    });
  }
);

test('native hierarchy read rejects stale generations and mismatched rooms', async () => {
  const stale = makeInvoke(loggedInSession, { ...validSnapshot, sessionGeneration: 8 });
  await assert.rejects(() => readSpaceHierarchyWithNativeOwner(ROOM_ID, true, stale.invoke));

  const omitted = makeInvoke(loggedInSession, { sessionGeneration: 9, rooms: [] });
  await assert.rejects(
    () => readSpaceHierarchyRoomWithNativeOwner(ROOM_ID, true, omitted.invoke)
  );

  const different = makeInvoke(loggedInSession, {
    sessionGeneration: 9,
    rooms: [{ ...validRoom, roomId: OTHER_ROOM_ID }],
  });
  await assert.rejects(
    () => readSpaceHierarchyRoomWithNativeOwner(ROOM_ID, true, different.invoke)
  );
});

test('native hierarchy read rejects malformed or unsupported native DTOs', async () => {
  const invalidRooms = [
    { ...validRoom, unknown: true },
    { ...validRoom, roomId: '!invalid' },
    { ...validRoom, canonicalAlias: '#invalid' },
    { ...validRoom, avatarUrl: 'https://example.org/avatar' },
    { ...validRoom, name: 'x'.repeat(4_097) },
    { ...validRoom, numJoinedMembers: Number.MAX_SAFE_INTEGER },
    { ...validRoom, joinRule: 'unknown' },
    { ...validRoom, roomType: 'm.forum' },
    { ...validRoom, worldReadable: 'true' },
  ];

  for (const room of invalidRooms) {
    const { invoke } = makeInvoke(loggedInSession, { sessionGeneration: 9, rooms: [room] });
    await assert.rejects(() => readSpaceHierarchyWithNativeOwner(ROOM_ID, true, invoke));
  }

  const invalidSnapshot = makeInvoke(loggedInSession, {
    sessionGeneration: 9,
    rooms: [validRoom],
    accessToken: 'secret',
  });
  await assert.rejects(
    () => readSpaceHierarchyWithNativeOwner(ROOM_ID, true, invalidSnapshot.invoke)
  );
});

test('native hierarchy read accepts the Rust optional-null shape', async () => {
  const { invoke } = makeInvoke(loggedInSession, {
    sessionGeneration: 9,
    rooms: [
      {
        ...validRoom,
        name: null,
        canonicalAlias: null,
        topic: null,
        avatarUrl: null,
        roomType: null,
      },
    ],
  });
  const room = await readSpaceHierarchyRoomWithNativeOwner(ROOM_ID, true, invoke);
  assert.deepEqual(room, {
    roomId: ROOM_ID,
    name: undefined,
    canonicalAlias: undefined,
    topic: undefined,
    avatarUrl: undefined,
    roomType: undefined,
    numJoinedMembers: 2,
    joinRule: 'public',
    worldReadable: true,
    guestCanJoin: false,
  });
});

test(
  'native hierarchy read turns unavailable and command errors into terminal failures',
  async () => {
    const unavailableSession = makeInvoke(loggedInSession, validSnapshot, {
      sessionAvailable: false,
    });
    await assert.rejects(
      () => readSpaceHierarchyWithNativeOwner(ROOM_ID, true, unavailableSession.invoke)
    );

    const unavailableHierarchy = makeInvoke(loggedInSession, validSnapshot, {
      hierarchyAvailable: false,
    });
    await assert.rejects(
      () => readSpaceHierarchyWithNativeOwner(ROOM_ID, true, unavailableHierarchy.invoke)
    );

    const commandError = makeInvoke(loggedInSession, validSnapshot, { throwOnHierarchy: true });
    await assert.rejects(
      () => readSpaceHierarchyWithNativeOwner(ROOM_ID, true, commandError.invoke)
    );
  }
);

test('native hierarchy read never returns an empty successful selected-room summary', async () => {
  const { invoke } = makeInvoke(loggedInSession, undefined);
  await assert.rejects(() => readSpaceHierarchyRoomWithNativeOwner(ROOM_ID, true, invoke));
});
