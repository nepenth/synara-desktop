import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { DesktopInvokeResult } from '../../../../utils/desktop';
import {
  getOwnProfileWithNativeOwner,
  setOwnAvatarWithNativeOwner,
  setOwnDisplayNameWithNativeOwner,
  uploadMediaWithNativeOwner,
  type NativeInvoke,
} from '../nativeProfileOwner';

const loggedIn: DesktopInvokeResult<unknown> = {
  available: true,
  value: { status: 'logged_in' },
};

test('display name returns legacy when desktop unavailable', async () => {
  const invoke: NativeInvoke = async () => {
    throw new Error('should not invoke');
  };
  assert.equal(await setOwnDisplayNameWithNativeOwner('Alice', false, invoke), 'legacy');
});

test('display name fail-closed when native session live but command unavailable', async () => {
  const invoke: NativeInvoke = async (command) => {
    if (command === 'matrix_session_snapshot') return loggedIn;
    return { available: false };
  };
  await assert.rejects(
    () => setOwnDisplayNameWithNativeOwner('Alice', true, invoke),
    /unavailable/i
  );
});

test('display name succeeds on native ok status', async () => {
  const invoke: NativeInvoke = async (command) => {
    if (command === 'matrix_session_snapshot') return loggedIn;
    if (command === 'matrix_set_own_display_name') {
      return { available: true, value: { status: 'ok' } };
    }
    throw new Error(`unexpected ${command}`);
  };
  assert.equal(await setOwnDisplayNameWithNativeOwner('Alice', true, invoke), 'native');
});

test('avatar set fails closed without fallthrough', async () => {
  const invoke: NativeInvoke = async (command) => {
    if (command === 'matrix_session_snapshot') return loggedIn;
    return { available: false };
  };
  await assert.rejects(
    () => setOwnAvatarWithNativeOwner('mxc://ex/x', true, invoke),
    /unavailable/i
  );
});

test('own profile read uses native owner and keeps mxc only', async () => {
  const invoke: NativeInvoke = async (command) => {
    if (command === 'matrix_session_snapshot') return loggedIn;
    if (command === 'matrix_get_own_profile') {
      return {
        available: true,
        value: {
          userId: '@alice:example.org',
          displayName: 'Alice',
          avatarUrl: 'mxc://example.org/abc',
        },
      };
    }
    throw new Error(`unexpected ${command}`);
  };
  assert.deepEqual(await getOwnProfileWithNativeOwner(true, invoke), {
    userId: '@alice:example.org',
    displayName: 'Alice',
    avatarUrl: 'mxc://example.org/abc',
  });
});

test('own profile read drops non-mxc avatars', async () => {
  const invoke: NativeInvoke = async (command) => {
    if (command === 'matrix_session_snapshot') return loggedIn;
    if (command === 'matrix_get_own_profile') {
      return {
        available: true,
        value: {
          userId: '@alice:example.org',
          displayName: 'Alice',
          avatarUrl: 'https://example.org/secret.png',
        },
      };
    }
    throw new Error(`unexpected ${command}`);
  };
  assert.deepEqual(await getOwnProfileWithNativeOwner(true, invoke), {
    userId: '@alice:example.org',
    displayName: 'Alice',
    avatarUrl: undefined,
  });
});

test('own profile read fails closed when native command is unavailable', async () => {
  const invoke: NativeInvoke = async (command) => {
    if (command === 'matrix_session_snapshot') return loggedIn;
    return { available: false };
  };
  await assert.rejects(() => getOwnProfileWithNativeOwner(true, invoke), /unavailable/i);
});

test('upload media returns mxc on native path', async () => {
  const invoke: NativeInvoke = async (command) => {
    if (command === 'matrix_session_snapshot') return loggedIn;
    if (command === 'matrix_upload_media') {
      return { available: true, value: { mxc: 'mxc://ex/abc' } };
    }
    throw new Error(`unexpected ${command}`);
  };
  const result = await uploadMediaWithNativeOwner('image/png', [1, 2, 3], true, invoke);
  assert.deepEqual(result, { mxc: 'mxc://ex/abc' });
});
