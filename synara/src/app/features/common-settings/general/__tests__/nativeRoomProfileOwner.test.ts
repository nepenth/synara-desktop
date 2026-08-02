import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { DesktopInvokeResult } from '../../../../utils/desktop';
import {
  getRoomDirectoryVisibilityWithNativeOwner,
  setRoomDirectoryVisibilityWithNativeOwner,
  setRoomAvatarWithNativeOwner,
  setRoomNameWithNativeOwner,
  setRoomTopicWithNativeOwner,
  type NativeInvoke,
} from '../nativeRoomProfileOwner';

const loggedInInvoke: NativeInvoke = async (command) => {
  if (command === 'matrix_session_snapshot') {
    return {
      available: true,
      value: {
        status: 'logged_in',
        user_id: '@alice:example.org',
        device_id: 'DEVICE',
        homeserver_url: 'https://matrix.example.org',
        sessionGeneration: 7,
      },
    };
  }
  if (
    command === 'matrix_set_room_name' ||
    command === 'matrix_set_room_topic' ||
    command === 'matrix_set_room_avatar'
  ) {
    return { available: true, value: { status: 'ok' } };
  }
  if (command === 'matrix_get_room_directory_visibility') {
    return {
      available: true,
      value: {
        status: 'ok',
        roomId: '!r:example.org',
        sessionGeneration: 7,
        visibility: 'public',
      },
    };
  }
  if (command === 'matrix_set_room_directory_visibility') {
    return {
      available: true,
      value: {
        status: 'ok',
        roomId: '!r:example.org',
        sessionGeneration: 7,
        requestedVisibility: 'private',
      },
    };
  }
  return { available: false };
};

const failClosedInvoke: NativeInvoke = async (command) => {
  if (command === 'matrix_session_snapshot') {
    return {
      available: true,
      value: {
        status: 'logged_in',
        user_id: '@alice:example.org',
        device_id: 'DEVICE',
        homeserver_url: 'https://matrix.example.org',
        sessionGeneration: 7,
      },
    };
  }
  return { available: false };
};

test('room name write legacy when not desktop', async () => {
  assert.equal(
    await setRoomNameWithNativeOwner('!r:example.org', 'Name', false, loggedInInvoke),
    'legacy'
  );
});

test('room name write native ok', async () => {
  assert.equal(
    await setRoomNameWithNativeOwner('!r:example.org', 'Name', true, loggedInInvoke),
    'native'
  );
});

test('room name write fail-closed when command missing', async () => {
  await assert.rejects(
    () => setRoomNameWithNativeOwner('!r:example.org', 'Name', true, failClosedInvoke),
    /unavailable/i
  );
});

test('room topic write native ok', async () => {
  assert.equal(
    await setRoomTopicWithNativeOwner('!r:example.org', 'Topic', true, loggedInInvoke),
    'native'
  );
});

test('room topic write fail-closed when command missing', async () => {
  await assert.rejects(
    () => setRoomTopicWithNativeOwner('!r:example.org', 'Topic', true, failClosedInvoke),
    /unavailable/i
  );
});

test('room avatar write native ok', async () => {
  assert.equal(
    await setRoomAvatarWithNativeOwner(
      '!r:example.org',
      'mxc://example.org/abc',
      true,
      loggedInInvoke
    ),
    'native'
  );
});

test('room avatar clear empty mxc native ok', async () => {
  assert.equal(
    await setRoomAvatarWithNativeOwner('!r:example.org', '', true, loggedInInvoke),
    'native'
  );
});

test('room avatar write fail-closed when command missing', async () => {
  await assert.rejects(
    () =>
      setRoomAvatarWithNativeOwner(
        '!r:example.org',
        'mxc://example.org/abc',
        true,
        failClosedInvoke
      ),
    /unavailable/i
  );
});

test('directory visibility read validates the room and generation-stamped DTO', async () => {
  const result = await getRoomDirectoryVisibilityWithNativeOwner(
    '!r:example.org',
    true,
    loggedInInvoke
  );
  assert.deepEqual(result, {
    status: 'ok',
    roomId: '!r:example.org',
    sessionGeneration: 7,
    visibility: 'public',
  });
});

test('directory visibility write validates the acknowledgement and exact request', async () => {
  const result = await setRoomDirectoryVisibilityWithNativeOwner(
    '!r:example.org',
    'private',
    true,
    loggedInInvoke
  );
  assert.deepEqual(result, {
    status: 'ok',
    roomId: '!r:example.org',
    sessionGeneration: 7,
    requestedVisibility: 'private',
  });
});

test('directory visibility owner rejects unavailable, invalid, stale, and malformed native state', async () => {
  await assert.rejects(
    () => getRoomDirectoryVisibilityWithNativeOwner('!r:example.org', false, loggedInInvoke),
    /unavailable/i
  );
  await assert.rejects(
    () => getRoomDirectoryVisibilityWithNativeOwner('#alias:example.org', true, loggedInInvoke),
    /unavailable/i
  );
  await assert.rejects(
    () =>
      getRoomDirectoryVisibilityWithNativeOwner('!r:example.org', true, async (command) =>
        command === 'matrix_session_snapshot'
          ? {
              available: true,
              value: {
                status: 'logged_in',
                user_id: '@alice:example.org',
                device_id: 'DEVICE',
                homeserver_url: 'https://matrix.example.org',
                sessionGeneration: 8,
              },
            }
          : { available: true, value: { status: 'ok' } }
      ),
    /unavailable/i
  );
  await assert.rejects(
    () =>
      getRoomDirectoryVisibilityWithNativeOwner('!r:example.org', true, async (command) =>
        command === 'matrix_session_snapshot'
          ? {
              available: true,
              value: {
                status: 'logged_in',
                user_id: '@alice:example.org',
                device_id: 'DEVICE',
                homeserver_url: 'https://matrix.example.org',
                sessionGeneration: 7,
              },
            }
          : { available: true, value: { status: 'ok', roomId: '!other:example.org' } }
      ),
    /unavailable/i
  );
});

// silence unused import for type-only DesktopInvokeResult in some tooling
const _typeCheck: DesktopInvokeResult<unknown> = { available: false };
void _typeCheck;
