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
import { nativeThumbnailContentUri } from '../nativeThumbnail';

test('isNativeMediaContentUri matches leftover mxc and timeline handles', () => {
  assert.equal(isNativeMediaContentUri('mxc://example/avatar'), true);
  assert.equal(isNativeMediaContentUri('synara-media://localhost/timeline-media-ab'), true);
  assert.equal(isNativeMediaContentUri('timeline-media-ab'), true);
  assert.equal(isNativeMediaContentUri('https://example.org/avatar'), false);
  assert.equal(isNativeMediaContentUri('blob:https://example.org/1'), false);
  assert.equal(isNativeMediaContentUri(undefined), false);
});

test('resolveMatrixMediaUrl keeps an unsized mxc URI for the native media protocol', () => {
  assert.equal(resolveMatrixMediaUrl('mxc://example/media'), 'mxc://example/media');
});

test('resolveMatrixMediaUrl turns a requested size into a native thumbnail', () => {
  assert.equal(
    resolveMatrixMediaUrl('mxc://example/media', {
      useAuthentication: true,
      width: 96,
      height: 64,
      resizeMethod: 'crop',
    }),
    nativeThumbnailContentUri('mxc://example/media', 96, 64, 'crop')
  );
});

test('resolveMatrixMediaUrl rejects URLs that are not Matrix media', () => {
  assert.throws(
    () => resolveMatrixMediaUrl('https://example.org/missing'),
    /Invalid Matrix media URL/
  );
});

test('resolveMatrixThumbnailUrl requests cropped square thumbnails', () => {
  assert.equal(
    resolveMatrixThumbnailUrl('mxc://example/avatar', 100, { useAuthentication: true }),
    nativeThumbnailContentUri('mxc://example/avatar', 100, 100, 'crop')
  );
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
    const mxcUrl = await createMatrixMediaObjectUrl('mxc://example/media', {
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
  await assert.rejects(
    createMatrixMediaObjectUrl('mxc://example/enc', {
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
  assert.doesNotMatch(media, /browser-encrypt-attachment|decryptFile|downloadEncryptedMedia/);
  assert.doesNotMatch(roomInput, /encryptFile|browser-encrypt-attachment/);
  assert.match(roomInput, /Native Matrix attachment send is unavailable/);
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
