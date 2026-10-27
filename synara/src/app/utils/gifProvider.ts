import { safeRemoteContentUrl } from './remoteContent';

export type GifProviderKind = 'klipy' | 'giphy' | 'custom';

export type GifPickerConfig = {
  enabled?: boolean;
  provider?: GifProviderKind;
  apiKey?: string;
  /**
   * Klipy: API base (default `https://api.klipy.com/api/v1`). GIPHY and
   * custom: the search URL.
   */
  endpoint?: string;
  contentRating?: string;
};

export type GifResult = {
  id: string;
  title: string;
  url: string;
  previewUrl?: string;
  width?: number;
  height?: number;
  sourceUrl?: string;
  provider?: GifProviderKind;
};

/** One page of results; `nextPage` is set while the provider has more. */
export type GifPage = {
  results: GifResult[];
  nextPage?: number;
};

export const GIF_SEARCH_LIMIT = 24;
export const GIF_UPLOAD_MAX_BYTES = 15 * 1024 * 1024;
const MAX_GIF_QUERY_LENGTH = 80;
const MIN_GIF_QUERY_LENGTH = 2;

const DEFAULT_ENDPOINTS: Record<GifProviderKind, string> = {
  klipy: 'https://api.klipy.com/api/v1',
  giphy: 'https://api.giphy.com/v1/gifs/search',
  custom: '',
};

export const normalizeGifQuery = (query: string): string =>
  query.replace(/\s+/g, ' ').trim().slice(0, MAX_GIF_QUERY_LENGTH);

const getConfiguredEndpoint = (config: GifPickerConfig, provider: GifProviderKind): string =>
  config.endpoint?.trim() || DEFAULT_ENDPOINTS[provider];

const isKnownProvider = (provider: unknown): provider is GifProviderKind =>
  provider === 'klipy' || provider === 'giphy' || provider === 'custom';

export const gifPickerEnabled = (config?: GifPickerConfig): boolean => {
  if (!config?.enabled) return false;
  const provider = config.provider ?? 'custom';
  if (!isKnownProvider(provider)) return false;
  if (provider !== 'custom' && !config.apiKey) return false;
  const endpoint = getConfiguredEndpoint(config, provider);
  return !!safeRemoteContentUrl(endpoint);
};

export const gifSearchAvailable = (config: GifPickerConfig | undefined, userEnabled: boolean) =>
  userEnabled && gifPickerEnabled(config);

/** Providers with a trending feed, shown before the user types. */
export const gifTrendingSupported = (config?: GifPickerConfig): boolean =>
  config?.provider === 'klipy' || config?.provider === 'giphy';

/**
 * Klipy and GIPHY forbid re-hosting their media, so their GIFs are sent as
 * the provider's own media URL and played inline by the reader. A custom
 * provider's GIF is uploaded to Matrix media instead.
 */
export const gifSendsAsLink = (provider?: GifProviderKind): boolean =>
  provider === 'klipy' || provider === 'giphy';

/** Search placeholder; Klipy requires its own wording. */
export const gifSearchPlaceholder = (config?: GifPickerConfig): string | undefined =>
  config?.provider === 'klipy' ? 'Search KLIPY' : undefined;

export const gifAttribution = (config?: GifPickerConfig): string | undefined => {
  if (config?.provider === 'klipy') return 'Powered by KLIPY';
  if (config?.provider === 'giphy') return 'Powered by GIPHY';
  return undefined;
};

type GifFeed = { kind: 'search'; query: string } | { kind: 'trending' };

const buildGifFeedUrl = (
  config: GifPickerConfig,
  feed: GifFeed,
  limit: number,
  page: number
): string | undefined => {
  const provider = config.provider ?? 'custom';
  if (!isKnownProvider(provider)) return undefined;
  const safeEndpoint = safeRemoteContentUrl(getConfiguredEndpoint(config, provider));
  if (!safeEndpoint) return undefined;
  let q: string | undefined;
  if (feed.kind === 'search') {
    q = normalizeGifQuery(feed.query);
    if (q.length < MIN_GIF_QUERY_LENGTH) return undefined;
  }
  const safePage = Number.isSafeInteger(page) && page > 0 ? page : 1;

  if (provider === 'klipy') {
    if (!config.apiKey) return undefined;
    const base = safeEndpoint.replace(/\/+$/, '');
    const url = new URL(`${base}/${encodeURIComponent(config.apiKey)}/gifs/${feed.kind}`);
    if (q) url.searchParams.set('q', q);
    url.searchParams.set('page', String(safePage));
    url.searchParams.set('per_page', String(limit));
    url.searchParams.set('content_filter', config.contentRating ?? 'medium');
    url.searchParams.set('format_filter', 'gif,webp');
    return url.toString();
  }
  if (provider === 'giphy') {
    if (!config.apiKey) return undefined;
    const url = new URL(safeEndpoint);
    if (feed.kind === 'trending') {
      url.pathname = url.pathname.replace(/\/search\/?$/, '/trending');
    }
    url.searchParams.set('api_key', config.apiKey);
    if (q) url.searchParams.set('q', q);
    url.searchParams.set('limit', String(limit));
    url.searchParams.set('offset', String((safePage - 1) * limit));
    url.searchParams.set('rating', config.contentRating ?? 'pg-13');
    return url.toString();
  }

  if (!q) return undefined;
  const url = new URL(safeEndpoint);
  url.searchParams.set('q', q);
  url.searchParams.set('limit', String(limit));
  return url.toString();
};

export const buildGifSearchUrl = (
  config: GifPickerConfig,
  query: string,
  limit = GIF_SEARCH_LIMIT,
  page = 1
): string | undefined => buildGifFeedUrl(config, { kind: 'search', query }, limit, page);

export const buildGifTrendingUrl = (
  config: GifPickerConfig,
  limit = GIF_SEARCH_LIMIT,
  page = 1
): string | undefined =>
  gifTrendingSupported(config)
    ? buildGifFeedUrl(config, { kind: 'trending' }, limit, page)
    : undefined;

const asObject = (value: unknown): Record<string, unknown> | undefined =>
  value && typeof value === 'object' ? (value as Record<string, unknown>) : undefined;

const asString = (value: unknown): string | undefined =>
  typeof value === 'string' && value.length > 0 ? value : undefined;

const asNumber = (value: unknown): number | undefined =>
  typeof value === 'number' && Number.isFinite(value) ? value : undefined;

const normalizeResult = (result: Partial<GifResult>): GifResult | undefined => {
  const url = result.url && safeRemoteContentUrl(result.url);
  if (!url) return undefined;
  const previewUrl = result.previewUrl && safeRemoteContentUrl(result.previewUrl);
  const sourceUrl = result.sourceUrl && safeRemoteContentUrl(result.sourceUrl);
  return {
    id: result.id || url,
    title: result.title || 'GIF',
    url,
    previewUrl,
    width: result.width,
    height: result.height,
    sourceUrl,
    provider: result.provider,
  };
};

const klipyFormat = (
  file: Record<string, unknown> | undefined,
  size: string,
  format: string
): Record<string, unknown> | undefined => asObject(asObject(file?.[size])?.[format]);

const parseKlipyItem = (obj: Record<string, unknown>): GifResult | undefined => {
  const file = asObject(obj.file);
  const gif =
    klipyFormat(file, 'md', 'gif') ??
    klipyFormat(file, 'hd', 'gif') ??
    klipyFormat(file, 'sm', 'gif');
  const preview =
    klipyFormat(file, 'sm', 'webp') ??
    klipyFormat(file, 'sm', 'gif') ??
    klipyFormat(file, 'xs', 'gif') ??
    gif;
  const slug = asString(obj.slug);
  const id = asString(obj.id) ?? (asNumber(obj.id) !== undefined ? String(obj.id) : slug);
  return normalizeResult({
    provider: 'klipy',
    id,
    title: asString(obj.title),
    url: asString(gif?.url),
    previewUrl: asString(preview?.url),
    width: asNumber(gif?.width),
    height: asNumber(gif?.height),
    sourceUrl: slug ? `https://klipy.com/gifs/${encodeURIComponent(slug)}` : undefined,
  });
};

const parseItem = (provider: GifProviderKind, obj: Record<string, unknown>) => {
  if (provider === 'klipy') return parseKlipyItem(obj);

  if (provider === 'giphy') {
    const images = asObject(obj.images);
    const original = asObject(images?.original);
    const preview =
      asObject(images?.fixed_width) ?? asObject(images?.preview_gif) ?? asObject(images?.downsized);
    return normalizeResult({
      provider,
      id: asString(obj.id),
      title: asString(obj.title),
      url: asString(original?.url),
      previewUrl: asString(preview?.url) ?? asString(original?.url),
      width: Number.parseInt(asString(original?.width) ?? '', 10) || undefined,
      height: Number.parseInt(asString(original?.height) ?? '', 10) || undefined,
      sourceUrl: asString(obj.url),
    });
  }

  return normalizeResult({
    provider,
    id: asString(obj.id),
    title: asString(obj.title),
    url: asString(obj.url),
    previewUrl: asString(obj.previewUrl) ?? asString(obj.preview_url),
    width: asNumber(obj.width),
    height: asNumber(obj.height),
    sourceUrl: asString(obj.sourceUrl) ?? asString(obj.source_url),
  });
};

export const parseGifPage = (
  provider: GifProviderKind,
  data: unknown,
  page = 1,
  limit = GIF_SEARCH_LIMIT
): GifPage => {
  const root = asObject(data);
  let items: unknown[] = [];
  let hasNext = false;

  if (provider === 'klipy') {
    // { result, data: { data: [...], current_page, per_page, has_next } }
    const body = asObject(root?.data);
    if (Array.isArray(body?.data)) items = body.data;
    hasNext = body?.has_next === true;
  } else {
    const resultItems = root?.results;
    const dataItems = root?.data;
    if (Array.isArray(resultItems)) {
      items = resultItems;
    } else if (Array.isArray(dataItems)) {
      items = dataItems;
    }
    if (provider === 'giphy') {
      const pagination = asObject(root?.pagination);
      const total = asNumber(pagination?.total_count);
      const offset = asNumber(pagination?.offset) ?? (page - 1) * limit;
      hasNext = total !== undefined && offset + items.length < total && items.length > 0;
    }
  }

  const results = items
    .map((item) => {
      const obj = asObject(item);
      return obj ? parseItem(provider, obj) : undefined;
    })
    .filter((result): result is GifResult => !!result);
  return hasNext ? { results, nextPage: page + 1 } : { results };
};

export const parseGifSearchResponse = (provider: GifProviderKind, data: unknown): GifResult[] =>
  parseGifPage(provider, data).results;

const fetchGifPage = async (
  config: GifPickerConfig,
  url: string | undefined,
  page: number,
  fetchFn: typeof fetch
): Promise<GifPage> => {
  if (!url) return { results: [] };
  const response = await fetchFn(url, {
    method: 'GET',
    credentials: 'omit',
    referrerPolicy: 'no-referrer',
  });
  if (response.status === 429) throw new Error('GIF provider rate limit reached.');
  if (!response.ok) throw new Error('Failed to search GIFs.');
  return parseGifPage(config.provider ?? 'custom', await response.json(), page);
};

export const searchGifProviderPage = async (
  config: GifPickerConfig,
  query: string,
  page = 1,
  fetchFn: typeof fetch = fetch
): Promise<GifPage> => {
  if (!gifPickerEnabled(config)) return { results: [] };
  return fetchGifPage(
    config,
    buildGifSearchUrl(config, query, GIF_SEARCH_LIMIT, page),
    page,
    fetchFn
  );
};

export const trendingGifProviderPage = async (
  config: GifPickerConfig,
  page = 1,
  fetchFn: typeof fetch = fetch
): Promise<GifPage> => {
  if (!gifPickerEnabled(config)) return { results: [] };
  return fetchGifPage(config, buildGifTrendingUrl(config, GIF_SEARCH_LIMIT, page), page, fetchFn);
};

export const searchGifProvider = async (
  config: GifPickerConfig,
  query: string,
  fetchFn: typeof fetch = fetch
): Promise<GifResult[]> => (await searchGifProviderPage(config, query, 1, fetchFn)).results;

export type GifDownload = {
  blob: Blob;
  fileName: string;
};

const sanitizeGifFileName = (value: string): string => {
  const base = value
    .replace(/[^\w.-]+/g, '_')
    .replace(/^_+|_+$/g, '')
    .slice(0, 80);
  const name = base || 'gif';
  return name.toLowerCase().endsWith('.gif') ? name : `${name}.gif`;
};

export const getGifFileName = (gif: Pick<GifResult, 'title' | 'id'>): string =>
  sanitizeGifFileName(gif.title || gif.id || 'gif');

export const fetchGifForUpload = async (
  gif: GifResult,
  fetchFn: typeof fetch = fetch,
  maxBytes = GIF_UPLOAD_MAX_BYTES
): Promise<GifDownload> => {
  const url = safeRemoteContentUrl(gif.url);
  if (!url) throw new Error('Unsafe GIF URL.');

  const response = await fetchFn(url, {
    method: 'GET',
    credentials: 'omit',
    referrerPolicy: 'no-referrer',
  });
  if (!response.ok) throw new Error('Failed to download GIF.');

  const contentType = response.headers.get('content-type') ?? '';
  if (!contentType.toLowerCase().startsWith('image/gif')) {
    throw new Error('Selected item is not a GIF.');
  }

  const contentLength = Number.parseInt(response.headers.get('content-length') ?? '', 10);
  if (Number.isFinite(contentLength) && contentLength > maxBytes) {
    throw new Error('Selected GIF is too large.');
  }

  const blob = await response.blob();
  if (blob.size > maxBytes) throw new Error('Selected GIF is too large.');

  return { blob, fileName: getGifFileName(gif) };
};
