import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { DesktopInvokeResult } from '../../../../utils/desktop';
import {
  setRoomAvatarWithNativeOwner,
  setRoomNameWithNativeOwner,
  setRoomTopicWithNativeOwner,
  type NativeInvoke,
} from '../nativeRoomProfileOwner';

const loggedInInvoke: NativeInvoke = async (command) => {
  if (command === 'matrix_session_snapshot') {
    return { available: true, value: { status: 'logged_in' } };
  }
  if (
    command === 'matrix_set_room_name' ||
    command === 'matrix_set_room_topic' ||
    command === 'matrix_set_room_avatar'
  ) {
    return { available: true, value: { status: 'ok' } };
  }
  return { available: false };
};

const failClosedInvoke: NativeInvoke = async (command) => {
  if (command === 'matrix_session_snapshot') {
    return { available: true, value: { status: 'logged_in' } };
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

// silence unused import for type-only DesktopInvokeResult in some tooling
const _typeCheck: DesktopInvokeResult<unknown> = { available: false };
void _typeCheck;
