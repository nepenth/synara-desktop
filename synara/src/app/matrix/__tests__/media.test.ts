import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import {
  downloadMatrixMedia,
  isNativeMediaContentUri,
  resolveMatrixMediaUrl,
  resolveMatrixThumbnailUrl,
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

test('downloadMatrixMedia resolves leftover mxc through native download without JS fetch', async () => {
  const originalFetch = globalThis.fetch;
  const originalWindow = globalThis.window;
  const requests: string[] = [];
  globalThis.fetch = (async (url: RequestInfo | URL) => {
    requests.push(String(url));
    return { blob: async () => new Blob(['js']) } as Response;
  }) as typeof fetch;

  (globalThis as { window: unknown }).window = {
    __TAURI_INTERNALS__: {
      invoke: async (command: string, args?: Record<string, unknown>) => {
        assert.equal(command, 'matrix_media_download');
        assert.equal(args?.contentUri, 'mxc://example/media');
        return { bytes: [9, 8, 7] };
      },
    },
  };

  const mx = { mxcUrlToHttp: () => 'https://example.invalid/should-not-run' };

  try {
    const blob = await downloadMatrixMedia(mx as never, 'mxc://example/media', {
      mimeType: 'text/plain',
    });
    assert.equal(requests.length, 0);
    assert.equal(await blob.arrayBuffer().then((buf) => new Uint8Array(buf).join(',')), '9,8,7');
  } finally {
    globalThis.fetch = originalFetch;
    (globalThis as { window: unknown }).window = originalWindow;
  }
});

test('downloadMatrixMedia resolves timeline handles through native download without JS fetch', async () => {
  const originalFetch = globalThis.fetch;
  const originalWindow = globalThis.window;
  const requests: string[] = [];
  globalThis.fetch = (async (url: RequestInfo | URL) => {
    requests.push(String(url));
    return { blob: async () => new Blob(['js']) } as Response;
  }) as typeof fetch;

  const handle = `timeline-media-${'ab'.repeat(32)}`;
  (globalThis as { window: unknown }).window = {
    __TAURI_INTERNALS__: {
      invoke: async (command: string, args?: Record<string, unknown>) => {
        assert.equal(command, 'matrix_media_download');
        assert.equal(args?.contentUri, handle);
        return { bytes: [1, 2, 3] };
      },
    },
  };

  const mx = { mxcUrlToHttp: () => 'https://example.invalid/should-not-run' };

  try {
    const blob = await downloadMatrixMedia(mx as never, `synara-media://localhost/${handle}`, {
      mimeType: 'image/png',
    });
    assert.equal(requests.length, 0);
    assert.equal(await blob.arrayBuffer().then((buf) => new Uint8Array(buf).join(',')), '1,2,3');
    assert.equal(blob.type, 'image/png');
  } finally {
    globalThis.fetch = originalFetch;
    (globalThis as { window: unknown }).window = originalWindow;
  }
});

test('downloadMatrixMedia fail-closes leftover encrypted mxc without a native handle', async () => {
  const mx = { mxcUrlToHttp: () => 'https://example.invalid/should-not-run' };
  await assert.rejects(
    downloadMatrixMedia(mx as never, 'mxc://example/enc', {
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
  assert.match(hook, /createMatrixMediaObjectUrl/);
  assert.doesNotMatch(hook, /browser-encrypt-attachment|decryptFile/);
});
