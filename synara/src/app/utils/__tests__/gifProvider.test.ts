import assert from 'node:assert/strict';
import test from 'node:test';
import {
  buildGifSearchUrl,
  buildGifTrendingUrl,
  fetchGifForUpload,
  gifAttribution,
  gifPickerEnabled,
  gifSearchAvailable,
  gifSearchPlaceholder,
  gifSendsAsLink,
  parseGifPage,
  parseGifSearchResponse,
  searchGifProvider,
  trendingGifProviderPage,
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
      provider: 'klipy',
      apiKey: 'secret',
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
    apiKey: 'secret',
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

const klipyConfig: GifPickerConfig = {
  enabled: true,
  provider: 'klipy',
  apiKey: 'klipy-key',
};

const klipyItem = (id: number, slug: string) => ({
  id,
  slug,
  title: `GIF ${id}`,
  type: 'gif',
  file: {
    md: {
      gif: { url: `https://static.klipy.com/ii/${slug}/md.gif`, width: 320, height: 240 },
    },
    sm: {
      webp: { url: `https://static.klipy.com/ii/${slug}/sm.webp`, width: 160, height: 120 },
    },
  },
});

test('Klipy search and trending URLs carry the key in the path and page params', () => {
  const search = new URL(buildGifSearchUrl(klipyConfig, ' high   five ', 24, 2)!);
  assert.equal(search.origin, 'https://api.klipy.com');
  assert.equal(search.pathname, '/api/v1/klipy-key/gifs/search');
  assert.equal(search.searchParams.get('q'), 'high five');
  assert.equal(search.searchParams.get('page'), '2');
  assert.equal(search.searchParams.get('per_page'), '24');
  assert.equal(search.searchParams.get('content_filter'), 'medium');

  const trending = new URL(buildGifTrendingUrl(klipyConfig)!);
  assert.equal(trending.pathname, '/api/v1/klipy-key/gifs/trending');
  assert.equal(trending.searchParams.get('q'), null);
  assert.equal(trending.searchParams.get('page'), '1');

  assert.equal(buildGifTrendingUrl({ ...klipyConfig, provider: 'custom' }), undefined);
  assert.equal(gifPickerEnabled({ ...klipyConfig, apiKey: '' }), false);
});

test('Klipy responses map md GIFs to send URLs and small webps to previews', () => {
  const page = parseGifPage(
    'klipy',
    {
      result: true,
      data: {
        data: [klipyItem(1, 'wave'), { id: 2, slug: 'broken', file: {} }],
        current_page: 1,
        per_page: 24,
        has_next: true,
      },
    },
    1
  );
  assert.equal(page.results.length, 1);
  assert.deepEqual(page.results[0], {
    id: '1',
    title: 'GIF 1',
    url: 'https://static.klipy.com/ii/wave/md.gif',
    previewUrl: 'https://static.klipy.com/ii/wave/sm.webp',
    width: 320,
    height: 240,
    sourceUrl: 'https://klipy.com/gifs/wave',
    provider: 'klipy',
  });
  assert.equal(page.nextPage, 2);
  assert.equal(parseGifPage('klipy', { data: { data: [], has_next: false } }).nextPage, undefined);
});

test('trending loads page by page without a query', async () => {
  const requested: string[] = [];
  const page = await trendingGifProviderPage(klipyConfig, 3, async (input) => {
    requested.push(String(input));
    return new Response(
      JSON.stringify({ data: { data: [klipyItem(9, 'nine')], has_next: false } })
    );
  });
  assert.equal(new URL(requested[0]).searchParams.get('page'), '3');
  assert.equal(page.results[0].url, 'https://static.klipy.com/ii/nine/md.gif');
  assert.equal(page.nextPage, undefined);
});

test('Klipy and GIPHY GIFs are sent as links; only custom GIFs are uploaded', () => {
  assert.equal(gifSendsAsLink('klipy'), true);
  assert.equal(gifSendsAsLink('giphy'), true);
  assert.equal(gifSendsAsLink('custom'), false);
  assert.equal(gifSendsAsLink(undefined), false);
  assert.equal(gifSearchPlaceholder(klipyConfig), 'Search KLIPY');
  assert.equal(gifAttribution(klipyConfig), 'Powered by KLIPY');
  assert.equal(gifAttribution({ ...klipyConfig, provider: 'custom' }), undefined);
});

test('an unknown provider such as the retired Tenor API is not enabled', () => {
  assert.equal(gifPickerEnabled({ enabled: true, provider: 'tenor' as never, apiKey: 'k' }), false);
});
