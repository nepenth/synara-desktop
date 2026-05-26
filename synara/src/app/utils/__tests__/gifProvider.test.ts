import assert from 'node:assert/strict';
import test from 'node:test';
import {
  buildGifSearchUrl,
  fetchGifForUpload,
  gifPickerEnabled,
  gifSearchAvailable,
  parseGifSearchResponse,
  searchGifProvider,
  type GifPickerConfig,
} from '../gifProvider';

test('gifPickerEnabled requires a safe endpoint and hosted provider API keys', () => {
  assert.equal(
    gifPickerEnabled({
      enabled: true,
      provider: 'custom',
      endpoint: 'https://gifs.example.org/search',
    }),
    true
  );
  assert.equal(
    gifPickerEnabled({
      enabled: true,
      provider: 'custom',
      endpoint: 'https://127.0.0.1/search',
    }),
    false
  );
  assert.equal(
    gifPickerEnabled({
      enabled: true,
      provider: 'giphy',
      endpoint: 'https://api.giphy.com/v1/gifs/search',
    }),
    false
  );
  assert.equal(
    gifPickerEnabled({
      enabled: true,
      provider: 'tenor',
      apiKey: '<SET_IN_CONFIG>',
      endpoint: '',
    }),
    true
  );
});

test('gifSearchAvailable requires both client config and user opt-in', () => {
  const config: GifPickerConfig = {
    enabled: true,
    provider: 'custom',
    endpoint: 'https://gif.example.org/search',
  };

  assert.equal(gifSearchAvailable(config, true), true);
  assert.equal(gifSearchAvailable(config, false), false);
  assert.equal(gifSearchAvailable({ ...config, enabled: false }, true), false);
});

test('buildGifSearchUrl normalizes search terms and provider params', () => {
  const config: GifPickerConfig = {
    enabled: true,
    provider: 'giphy',
    endpoint: 'https://api.giphy.com/v1/gifs/search',
    apiKey: '<SET_IN_CONFIG>',
  };
  const url = new URL(buildGifSearchUrl(config, '  hello   world  ', 10)!);

  assert.equal(url.searchParams.get('q'), 'hello world');
  assert.equal(url.searchParams.get('limit'), '10');
  assert.equal(url.searchParams.get('api_key'), 'secret');
});

test('parseGifSearchResponse filters unsafe result URLs', () => {
  const results = parseGifSearchResponse('custom', {
    results: [
      {
        id: 'safe',
        title: 'safe',
        url: 'https://cdn.example.org/safe.gif',
        previewUrl: 'https://cdn.example.org/safe-preview.gif',
        sourceUrl: 'https://gifs.example.org/safe',
      },
      {
        id: 'unsafe',
        title: 'unsafe',
        url: 'http://127.0.0.1/private.gif',
        sourceUrl: 'https://127.0.0.1/private',
      },
    ],
  });

  assert.equal(results.length, 1);
  assert.equal(results[0].id, 'safe');
  assert.equal(results[0].sourceUrl, 'https://gifs.example.org/safe');
});

test('parseGifSearchResponse omits unsafe source URLs from metadata', () => {
  const results = parseGifSearchResponse('custom', {
    results: [
      {
        id: 'safe-gif-unsafe-source',
        title: 'safe gif unsafe source',
        url: 'https://cdn.example.org/safe.gif',
        sourceUrl: 'https://127.0.0.1/private',
      },
    ],
  });

  assert.equal(results.length, 1);
  assert.equal(results[0].sourceUrl, undefined);
});

test('searchGifProvider fetches without credentials or referrer', async () => {
  const config: GifPickerConfig = {
    enabled: true,
    provider: 'custom',
    endpoint: 'https://gifs.example.org/search',
  };
  let requestInit: RequestInit | undefined;
  const results = await searchGifProvider(config, 'hi', async (_input, init) => {
    requestInit = init;
    return new Response(
      JSON.stringify({
        results: [{ id: '1', title: 'hi', url: 'https://cdn.example.org/hi.gif' }],
      })
    );
  });

  assert.equal(requestInit?.credentials, 'omit');
  assert.equal(requestInit?.referrerPolicy, 'no-referrer');
  assert.equal(results.length, 1);
});

test('fetchGifForUpload downloads GIFs without credentials or referrer', async () => {
  let requestInput: RequestInfo | URL | undefined;
  let requestInit: RequestInit | undefined;
  const gif = {
    id: '1',
    title: 'happy dance',
    url: 'https://cdn.example.org/happy.gif',
  };

  const download = await fetchGifForUpload(gif, async (input, init) => {
    requestInput = input;
    requestInit = init;
    return new Response(new Blob(['gif'], { type: 'image/gif' }), {
      headers: {
        'content-type': 'image/gif',
        'content-length': '3',
      },
    });
  });

  assert.equal(requestInput, 'https://cdn.example.org/happy.gif');
  assert.equal(requestInit?.credentials, 'omit');
  assert.equal(requestInit?.referrerPolicy, 'no-referrer');
  assert.equal(download.fileName, 'happy_dance.gif');
  assert.equal(download.blob.size, 3);
});

test('fetchGifForUpload rejects non-GIF and oversized downloads', async () => {
  const gif = {
    id: '1',
    title: 'not gif',
    url: 'https://cdn.example.org/file.png',
  };

  await assert.rejects(
    () =>
      fetchGifForUpload(gif, async () => new Response(new Blob(['png'], { type: 'image/png' }))),
    /not a GIF/
  );

  await assert.rejects(
    () =>
      fetchGifForUpload(
        { ...gif, url: 'https://cdn.example.org/large.gif' },
        async () =>
          new Response(new Blob(['gif'], { type: 'image/gif' }), {
            headers: {
              'content-type': 'image/gif',
              'content-length': '10',
            },
          }),
        2
      ),
    /too large/
  );
});
