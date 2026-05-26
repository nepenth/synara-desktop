import { safeRemoteContentUrl } from './remoteContent';

export type GifProviderKind = 'tenor' | 'giphy' | 'custom';

export type GifPickerConfig = {
  enabled?: boolean;
  provider?: GifProviderKind;
  apiKey?: string;
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

export const GIF_SEARCH_LIMIT = 24;
export const GIF_UPLOAD_MAX_BYTES = 15 * 1024 * 1024;
const MAX_GIF_QUERY_LENGTH = 80;
const MIN_GIF_QUERY_LENGTH = 2;

const DEFAULT_ENDPOINTS: Record<GifProviderKind, string> = {
  tenor: 'https://tenor.googleapis.com/v2/search',
  giphy: 'https://api.giphy.com/v1/gifs/search',
  custom: '',
};

export const normalizeGifQuery = (query: string): string =>
  query.replace(/\s+/g, ' ').trim().slice(0, MAX_GIF_QUERY_LENGTH);

const getConfiguredEndpoint = (config: GifPickerConfig, provider: GifProviderKind): string =>
  config.endpoint?.trim() || DEFAULT_ENDPOINTS[provider];

export const gifPickerEnabled = (config?: GifPickerConfig): boolean => {
  if (!config?.enabled) return false;
  const provider = config.provider ?? 'custom';
  if (provider !== 'custom' && !config.apiKey) return false;
  const endpoint = getConfiguredEndpoint(config, provider);
  return !!safeRemoteContentUrl(endpoint);
};

export const gifSearchAvailable = (config: GifPickerConfig | undefined, userEnabled: boolean) =>
  userEnabled && gifPickerEnabled(config);

export const buildGifSearchUrl = (
  config: GifPickerConfig,
  query: string,
  limit = GIF_SEARCH_LIMIT
): string | undefined => {
  const provider = config.provider ?? 'custom';
  const endpoint = getConfiguredEndpoint(config, provider);
  const safeEndpoint = safeRemoteContentUrl(endpoint);
  const q = normalizeGifQuery(query);
  if (!safeEndpoint || q.length < MIN_GIF_QUERY_LENGTH) return undefined;

  const url = new URL(safeEndpoint);
  if (provider === 'tenor') {
    if (!config.apiKey) return undefined;
    url.searchParams.set('key', config.apiKey);
    url.searchParams.set('q', q);
    url.searchParams.set('limit', String(limit));
    url.searchParams.set('media_filter', 'gif,tinygif');
    url.searchParams.set('contentfilter', config.contentRating ?? 'medium');
    return url.toString();
  }
  if (provider === 'giphy') {
    if (!config.apiKey) return undefined;
    url.searchParams.set('api_key', config.apiKey);
    url.searchParams.set('q', q);
    url.searchParams.set('limit', String(limit));
    url.searchParams.set('rating', config.contentRating ?? 'pg-13');
    return url.toString();
  }

  url.searchParams.set('q', q);
  url.searchParams.set('limit', String(limit));
  return url.toString();
};

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

export const parseGifSearchResponse = (provider: GifProviderKind, data: unknown): GifResult[] => {
  const root = asObject(data);
  let items: unknown[] = [];
  const resultItems = root?.results;
  const dataItems = root?.data;
  if (Array.isArray(resultItems)) {
    items = resultItems;
  } else if (Array.isArray(dataItems)) {
    items = dataItems;
  }

  return items
    .map((item) => {
      const obj = asObject(item);
      if (!obj) return undefined;

      if (provider === 'tenor') {
        const formats = asObject(obj.media_formats);
        const gif = asObject(formats?.gif);
        const tinyGif = asObject(formats?.tinygif);
        return normalizeResult({
          provider,
          id: asString(obj.id),
          title: asString(obj.content_description) ?? asString(obj.title),
          url: asString(gif?.url) ?? asString(tinyGif?.url),
          previewUrl: asString(tinyGif?.url) ?? asString(gif?.url),
          width: Array.isArray(gif?.dims) ? asNumber(gif?.dims[0]) : undefined,
          height: Array.isArray(gif?.dims) ? asNumber(gif?.dims[1]) : undefined,
          sourceUrl: asString(obj.itemurl),
        });
      }

      if (provider === 'giphy') {
        const images = asObject(obj.images);
        const original = asObject(images?.original);
        const preview = asObject(images?.preview_gif) ?? asObject(images?.downsized);
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
    })
    .filter((result): result is GifResult => !!result);
};

export const searchGifProvider = async (
  config: GifPickerConfig,
  query: string,
  fetchFn: typeof fetch = fetch
): Promise<GifResult[]> => {
  if (!gifPickerEnabled(config)) return [];
  const provider = config.provider ?? 'custom';
  const url = buildGifSearchUrl(config, query);
  if (!url) return [];
  const response = await fetchFn(url, {
    method: 'GET',
    credentials: 'omit',
    referrerPolicy: 'no-referrer',
  });
  if (response.status === 429) throw new Error('GIF provider rate limit reached.');
  if (!response.ok) throw new Error('Failed to search GIFs.');
  return parseGifSearchResponse(provider, await response.json());
};

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
