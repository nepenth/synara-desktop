import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import {
  createMatrixMediaObjectUrl,
  isNativeMediaContentUri,
  nativeMediaDisplayUrl,
  previewMatrixMediaText,
  resolveMatrixMediaUrl,
  resolveMatrixThumbnailUrl,
  saveMatrixMediaFile,
} from '../media';

type MockMatrixClient = {
  mxcUrlToHttp: (...args: unknown[]) => string | null;
};

test('isNativeMediaContentUri matches leftover mxc and timeline handles', () => {
  assert.equal(isNativeMediaContentUri('mxc://example/avatar'), true);
  assert.equal(isNativeMediaContentUri('synara-media://localhost/timeline-media-ab'), true);
  assert.equal(isNativeMediaContentUri('timeline-media-ab'), true);
  assert.equal(isNativeMediaContentUri('https://example.org/avatar'), false);
  assert.equal(isNativeMediaContentUri('blob:https://example.org/1'), false);
  assert.equal(isNativeMediaContentUri(undefined), false);
});

test('resolveMatrixMediaUrl delegates authenticated MXC conversion to matrix-js-sdk', () => {
  const calls: unknown[][] = [];
  const mx: MockMatrixClient = {
    mxcUrlToHttp: (...args: unknown[]) => {
      calls.push(args);
      return 'https://matrix.example.org/_matrix/media/v3/download/example/media';
    },
  };

  assert.equal(
    resolveMatrixMediaUrl(mx as never, 'mxc://example/media', {
      useAuthentication: true,
      width: 96,
      height: 64,
      resizeMethod: 'crop',
      allowDirectLinks: false,
      allowRedirects: true,
    }),
    'https://matrix.example.org/_matrix/media/v3/download/example/media'
  );
  assert.deepEqual(calls, [['mxc://example/media', 96, 64, 'crop', false, true, true]]);
});

test('resolveMatrixMediaUrl rejects unresolved Matrix media URLs', () => {
  const mx: MockMatrixClient = {
    mxcUrlToHttp: () => null,
  };

  assert.throws(
    () => resolveMatrixMediaUrl(mx as never, 'mxc://example/missing'),
    /Invalid Matrix media URL/
  );
});

test('resolveMatrixThumbnailUrl requests cropped authenticated thumbnails', () => {
  const calls: unknown[][] = [];
  const mx: MockMatrixClient = {
    mxcUrlToHttp: (...args: unknown[]) => {
      calls.push(args);
      return 'https://matrix.example.org/_matrix/media/v3/thumbnail/example/avatar';
    },
  };

  assert.equal(
    resolveMatrixThumbnailUrl(mx as never, 'mxc://example/avatar', 100, {
      useAuthentication: true,
    }),
    'https://matrix.example.org/_matrix/media/v3/thumbnail/example/avatar'
  );
  assert.deepEqual(calls, [['mxc://example/avatar', 100, 100, 'crop', undefined, undefined, true]]);
});

test('display URLs stay on synara-media and do not fetch file bytes', async () => {
  const originalFetch = globalThis.fetch;
  const originalWindow = globalThis.window;
  const requests: string[] = [];
  globalThis.fetch = (async (url: RequestInfo | URL) => {
    requests.push(String(url));
    return { blob: async () => new Blob(['js']) } as Response;
  }) as typeof fetch;
  const handle = `timeline-media-${'ab'.repeat(32)}`;
  (globalThis as { window: unknown }).window = {
    __SYNARA_DESKTOP__: { platform: 'tauri' },
    __TAURI_INTERNALS__: {
      convertFileSrc: (filePath: string, protocol = 'asset') =>
        `${protocol}://localhost/${encodeURIComponent(filePath)}`,
      invoke: async () => {
        throw new Error('display must not invoke a byte download');
      },
    },
  };

  try {
    const mx = { mxcUrlToHttp: () => 'https://example.invalid/should-not-run' };
    const mxcUrl = await createMatrixMediaObjectUrl(mx as never, 'mxc://example/media', {
      mimeType: 'image/png',
    });
    assert.equal(mxcUrl, 'synara-media://localhost/mxc%3A%2F%2Fexample%2Fmedia');
    assert.equal(
      nativeMediaDisplayUrl(`synara-media://localhost/${handle}`),
      `synara-media://localhost/${encodeURIComponent(handle)}`
    );
    assert.equal(requests.length, 0);
  } finally {
    globalThis.fetch = originalFetch;
    (globalThis as { window: unknown }).window = originalWindow;
  }
});

test('text preview and save return text or a filename, not file bytes', async () => {
  const originalWindow = globalThis.window;
  const handle = `timeline-media-${'ab'.repeat(32)}`;
  const calls: string[] = [];
  (globalThis as { window: unknown }).window = {
    __SYNARA_DESKTOP__: { platform: 'tauri' },
    __TAURI_INTERNALS__: {
      invoke: async (command: string, args?: Record<string, unknown>) => {
        calls.push(command);
        assert.equal(args?.contentUri, handle);
        if (command === 'matrix_media_text_preview') return { text: '# hi' };
        if (command === 'matrix_media_save') {
          assert.equal(args?.filename, 'notes.md');
          return { filename: 'notes.md' };
        }
        throw new Error(`unexpected ${command}`);
      },
    },
  };

  try {
    assert.deepEqual(await previewMatrixMediaText(handle), { kind: 'ready', text: '# hi' });
    assert.equal(await saveMatrixMediaFile(handle, 'notes.md'), 'notes.md');
    assert.deepEqual(calls, ['matrix_media_text_preview', 'matrix_media_save']);
  } finally {
    (globalThis as { window: unknown }).window = originalWindow;
  }
});

test('text preview maps the native size ceiling without returning bytes', async () => {
  const originalWindow = globalThis.window;
  (globalThis as { window: unknown }).window = {
    __TAURI_INTERNALS__: {
      invoke: async () => {
        throw { diagnosticId: 'v-send.r-media-preview-too-large' };
      },
    },
  };
  try {
    assert.deepEqual(await previewMatrixMediaText('timeline-media-ab'), { kind: 'tooLarge' });
  } finally {
    (globalThis as { window: unknown }).window = originalWindow;
  }
});

test('encrypted leftover mxc fails closed before a display URL is built', async () => {
  const mx = { mxcUrlToHttp: () => 'https://example.invalid/should-not-run' };
  await assert.rejects(
    createMatrixMediaObjectUrl(mx as never, 'mxc://example/enc', {
      mimeType: 'image/png',
      encryptedInfo: {
        v: 'v2',
        key: { alg: 'A256CTR', ext: true, k: 'x', key_ops: ['encrypt', 'decrypt'], kty: 'oct' },
        iv: 'iv',
        hashes: { sha256: 'hash' },
      },
    }),
    /Leftover encrypted media requires a native handle/
  );
});

test('desktop media boundary has no JS encrypt/decrypt leftover', () => {
  const media = readFileSync(join(process.cwd(), 'src/app/matrix/media.ts'), 'utf8');
  const roomInput = readFileSync(
    join(process.cwd(), 'src/app/features/room/RoomInput.tsx'),
    'utf8'
  );
  const sw = readFileSync(join(process.cwd(), 'src/sw.ts'), 'utf8');
  assert.doesNotMatch(media, /browser-encrypt-attachment|decryptFile|downloadEncryptedMedia/);
  assert.doesNotMatch(roomInput, /encryptFile|browser-encrypt-attachment/);
  assert.match(roomInput, /Native Matrix attachment send is unavailable/);
  assert.doesNotMatch(sw, /_matrix\/client\/v1\/media|accessToken|Bearer/);
});

test('desktop leftover avatars resolve through native media src', () => {
  const roomAvatar = readFileSync(
    join(process.cwd(), 'src/app/components/room-avatar/RoomAvatar.tsx'),
    'utf8'
  );
  const userAvatar = readFileSync(
    join(process.cwd(), 'src/app/components/user-avatar/UserAvatar.tsx'),
    'utf8'
  );
  const hook = readFileSync(
    join(process.cwd(), 'src/app/hooks/useNativeMatrixMediaSrc.ts'),
    'utf8'
  );
  assert.match(roomAvatar, /useNativeMatrixMediaSrc/);
  assert.match(userAvatar, /useNativeMatrixMediaSrc/);
  assert.match(hook, /nativeMediaDisplayUrl/);
  assert.doesNotMatch(hook, /matrix_media_download|downloadMatrixMedia/);
  assert.doesNotMatch(hook, /browser-encrypt-attachment|decryptFile/);
});
