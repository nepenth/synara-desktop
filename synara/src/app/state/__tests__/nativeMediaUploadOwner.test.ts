import assert from 'node:assert/strict';
import test from 'node:test';
import { uploadMediaWithNativeOwner, type NativeInvoke } from '../nativeMediaUploadOwner';

const loggedInInvoke: NativeInvoke = async (command) => {
  if (command === 'matrix_session_snapshot') {
    return { available: true, value: { status: 'logged_in' } };
  }
  if (command === 'matrix_upload_media') {
    return { available: true, value: { mxc: 'mxc://example.org/pack1' } };
  }
  return { available: false };
};

const failClosedInvoke: NativeInvoke = async (command) => {
  if (command === 'matrix_session_snapshot') {
    return { available: true, value: { status: 'logged_in' } };
  }
  return { available: false };
};

test('pack media upload legacy when not desktop', async () => {
  assert.equal(
    await uploadMediaWithNativeOwner('image/png', [1, 2, 3], false, loggedInInvoke),
    'legacy'
  );
});

test('pack media upload native ok', async () => {
  const result = await uploadMediaWithNativeOwner('image/png', [1, 2, 3], true, loggedInInvoke);
  assert.deepEqual(result, { mxc: 'mxc://example.org/pack1' });
});

test('pack media upload fail-closed when command missing', async () => {
  await assert.rejects(
    () => uploadMediaWithNativeOwner('image/png', [1, 2, 3], true, failClosedInvoke),
    /unavailable/i
  );
});
