import assert from 'node:assert/strict';
import test from 'node:test';

import {
  downloadFileWithNativeOwner,
  getMediaConfigWithNativeOwner,
  isValidCallWidgetMediaContentUri,
  type NativeCallWidgetMediaInvoke,
} from '../nativeCallWidgetMediaOwner';

const loggedInSession = { available: true as const, value: { status: 'logged_in' } };

test('native media config requires a logged-in session and uses the exact command', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativeCallWidgetMediaInvoke = async (command, args) => {
    calls.push({ command, args });
    if (command === 'matrix_session_snapshot') return loggedInSession;
    return { available: true, value: { 'm.upload.size': 16 * 1024 * 1024 } };
  };

  assert.deepEqual(await getMediaConfigWithNativeOwner(true, invoke), {
    'm.upload.size': 16 * 1024 * 1024,
  });
  assert.deepEqual(calls, [
    { command: 'matrix_session_snapshot', args: undefined },
    { command: 'matrix_call_media_config', args: undefined },
  ]);
});

test('native media download uses the exact camelCase request and returns Uint8Array', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const contentUri = 'mxc://example.org/call-media';
  const invoke: NativeCallWidgetMediaInvoke = async (command, args) => {
    calls.push({ command, args });
    if (command === 'matrix_session_snapshot') return loggedInSession;
    return { available: true, value: { bytes: [0, 1, 255] } };
  };

  const result = await downloadFileWithNativeOwner(contentUri, true, invoke);
  assert.ok(result.file instanceof Uint8Array);
  assert.deepEqual([...result.file], [0, 1, 255]);
  assert.deepEqual(calls, [
    { command: 'matrix_session_snapshot', args: undefined },
    { command: 'matrix_media_download', args: { contentUri } },
  ]);
});

test('media content URI validation rejects non-MXC, credential-bearing, and oversized values', () => {
  assert.equal(isValidCallWidgetMediaContentUri('mxc://example.org/call-media'), true);
  for (const value of [
    '',
    'https://example.org/call-media',
    'data:text/plain,secret',
    'javascript:alert(1)',
    'mxc://example.org/call-media?access_token=secret',
    'mxc://example.org/',
    `mxc://example.org/${'a'.repeat(2_050)}`,
  ]) {
    assert.equal(isValidCallWidgetMediaContentUri(value), false, value);
  }
});

test('native media failures are terminal and visibly unavailable', async () => {
  await assert.rejects(
    getMediaConfigWithNativeOwner(false, async () => {
      throw new Error('invoke must not run');
    }),
    /Native Matrix call widget media is unavailable/
  );

  await assert.rejects(
    getMediaConfigWithNativeOwner(true, async () => ({
      available: true,
      value: { status: 'logged_out' },
    })),
    /Native Matrix call widget session is unavailable/
  );

  await assert.rejects(
    getMediaConfigWithNativeOwner(true, async (command) => {
      if (command === 'matrix_session_snapshot') return loggedInSession;
      return { available: false };
    }),
    /Native Matrix call widget media config is unavailable/
  );

  await assert.rejects(
    getMediaConfigWithNativeOwner(true, async (command) => {
      if (command === 'matrix_session_snapshot') return loggedInSession;
      throw new Error('raw SDK error must not escape');
    }),
    /Native Matrix call widget media config is unavailable/
  );

  for (const value of [
    {},
    { 'm.upload.size': Number.NaN },
    { 'm.upload.size': Number.POSITIVE_INFINITY },
    { 'm.upload.size': 1.5 },
    { 'm.upload.size': Number.MAX_SAFE_INTEGER + 1 },
    { 'm.upload.size': 12, extra: true },
  ]) {
    await assert.rejects(
      getMediaConfigWithNativeOwner(true, async (command) =>
        command === 'matrix_session_snapshot' ? loggedInSession : { available: true, value }
      ),
      /Native Matrix call widget media config is unavailable/
    );
  }

  for (const value of [
    {},
    { bytes: [1, 2, 300] },
    { bytes: [1, Number.NaN] },
    { bytes: [], extra: true },
  ]) {
    await assert.rejects(
      downloadFileWithNativeOwner('mxc://example.org/call-media', true, async (command) =>
        command === 'matrix_session_snapshot' ? loggedInSession : { available: true, value }
      ),
      /Native Matrix call widget media download is unavailable/
    );
  }

  await assert.rejects(
    downloadFileWithNativeOwner('https://example.org/call-media', true, async () => {
      throw new Error('invalid URI must not invoke native');
    }),
    /Native Matrix call widget media download is unavailable/
  );
});

test('native media failures make zero calls to a legacy JS media path', async () => {
  const calls: string[] = [];
  const invoke: NativeCallWidgetMediaInvoke = async (command) => {
    calls.push(command);
    if (command === 'matrix_session_snapshot') return loggedInSession;
    return { available: false };
  };

  await assert.rejects(getMediaConfigWithNativeOwner(true, invoke));
  await assert.rejects(downloadFileWithNativeOwner('mxc://example.org/call-media', true, invoke));
  assert.deepEqual(calls, [
    'matrix_session_snapshot',
    'matrix_call_media_config',
    'matrix_session_snapshot',
    'matrix_media_download',
  ]);
});
