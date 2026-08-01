import assert from 'node:assert/strict';
import test from 'node:test';
import { uploadCallWidgetFileWithNativeOwner } from '../nativeCallMediaUploadOwner';
import type { NativeInvoke } from '../../../state/nativeMediaUploadOwner';

const loggedInInvoke: NativeInvoke = async (command) => {
  if (command === 'matrix_session_snapshot') {
    return { available: true, value: { status: 'logged_in' } };
  }
  if (command === 'matrix_upload_media') {
    return { available: true, value: { mxc: 'mxc://example.org/call-media' } };
  }
  return { available: false };
};

test('native call media upload uses matrix_upload_media', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativeInvoke = async (command, args) => {
    calls.push({ command, args });
    return loggedInInvoke(command, args);
  };
  const result = await uploadCallWidgetFileWithNativeOwner(
    new Blob([new Uint8Array([1, 2, 3])], { type: 'image/png' }),
    true,
    invoke,
    async () => {
      throw new Error('legacy upload must not be called');
    }
  );

  assert.deepEqual(result, { contentUri: 'mxc://example.org/call-media' });
  assert.deepEqual(calls, [
    { command: 'matrix_session_snapshot', args: undefined },
    {
      command: 'matrix_upload_media',
      args: { mimeType: 'image/png', bytes: [1, 2, 3] },
    },
  ]);
});

test('web call media upload retains the legacy owner', async () => {
  let legacyUploads = 0;
  const result = await uploadCallWidgetFileWithNativeOwner(
    new Blob([new Uint8Array([1, 2, 3])], { type: 'image/png' }),
    false,
    async () => {
      throw new Error('native session check must not invoke desktop');
    },
    async () => {
      legacyUploads += 1;
      return { content_uri: 'mxc://example.org/legacy-call-media' };
    }
  );

  assert.equal(legacyUploads, 1);
  assert.deepEqual(result, { contentUri: 'mxc://example.org/legacy-call-media' });
});

test('native call media upload fails closed without legacy fallback', async () => {
  let legacyUploads = 0;
  const failClosedInvoke: NativeInvoke = async (command) =>
    command === 'matrix_session_snapshot'
      ? { available: true, value: { status: 'logged_in' } }
      : { available: false };

  await assert.rejects(
    uploadCallWidgetFileWithNativeOwner(
      new Blob([new Uint8Array([1, 2, 3])], { type: 'image/png' }),
      true,
      failClosedInvoke,
      async () => {
        legacyUploads += 1;
        return { content_uri: 'mxc://example.org/legacy-call-media' };
      }
    ),
    /Native Matrix media upload is unavailable/
  );
  assert.equal(legacyUploads, 0);
});

test('native call media upload rejects unsupported widget bodies', async () => {
  await assert.rejects(
    uploadCallWidgetFileWithNativeOwner('not-a-binary-upload', true, loggedInInvoke, async () => {
      throw new Error('legacy upload must not be called');
    }),
    /Native Matrix call media upload is unavailable/
  );
});
