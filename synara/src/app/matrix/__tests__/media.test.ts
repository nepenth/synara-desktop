import assert from 'node:assert/strict';
import test from 'node:test';
import { downloadMatrixMedia, resolveMatrixMediaUrl, resolveMatrixThumbnailUrl } from '../media';

type MockMatrixClient = {
  mxcUrlToHttp: (...args: unknown[]) => string | null;
};

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

test('downloadMatrixMedia fetches resolved Matrix media through the desktop media boundary', async () => {
  const originalFetch = globalThis.fetch;
  const requests: Array<{ url: string; method?: string }> = [];
  const blob = new Blob(['hello'], { type: 'text/plain' });
  globalThis.fetch = (async (url: RequestInfo | URL, init?: RequestInit) => {
    requests.push({ url: String(url), method: init?.method });
    return { blob: async () => blob } as Response;
  }) as typeof fetch;

  const mx: MockMatrixClient = {
    mxcUrlToHttp: () => 'https://matrix.example.org/_matrix/media/v3/download/example/media',
  };

  try {
    assert.equal(
      await downloadMatrixMedia(mx as never, 'mxc://example/media', {
        mimeType: 'text/plain',
        useAuthentication: true,
      }),
      blob
    );
    assert.deepEqual(requests, [
      {
        url: 'https://matrix.example.org/_matrix/media/v3/download/example/media',
        method: 'GET',
      },
    ]);
  } finally {
    globalThis.fetch = originalFetch;
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
      encryptedInfo: {
        v: 'v2',
        key: { alg: 'A256CTR', ext: true, k: 'x', key_ops: ['encrypt', 'decrypt'], kty: 'oct' },
        iv: 'iv',
        hashes: { sha256: 'hash' },
      },
    });
    assert.equal(requests.length, 0);
    assert.equal(await blob.arrayBuffer().then((buf) => new Uint8Array(buf).join(',')), '1,2,3');
    assert.equal(blob.type, 'image/png');
  } finally {
    globalThis.fetch = originalFetch;
    (globalThis as { window: unknown }).window = originalWindow;
  }
});
