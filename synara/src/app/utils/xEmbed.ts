import type { DesktopInvokeResult } from './desktop';
import { safeRemoteContentUrl } from './remoteContent';

/**
 * X post embeds. The desktop shell fetches a post's public JSON from X's
 * embed syndication endpoint (`desktop_x_post`); this module finds post
 * links, derives the endpoint token, and parses only what the card draws.
 * Media must come from X's own CDNs, which the CSP allows for this card.
 */

const X_HOSTS = new Set([
  'x.com',
  'www.x.com',
  'mobile.x.com',
  'twitter.com',
  'www.twitter.com',
  'mobile.twitter.com',
]);
const STATUS_PATH = /^\/(?:[A-Za-z0-9_]{1,15}|i(?:\/web)?)\/status(?:es)?\/(\d{1,20})(?:\/|$)/;
const VIDEO_HOST = 'video.twimg.com';
const IMAGE_HOST = 'pbs.twimg.com';
const MAX_TEXT_CHARS = 1000;
const MAX_MEDIA = 4;

export type XPostMedia =
  | { kind: 'photo'; url: string; width?: number; height?: number }
  | { kind: 'video' | 'gif'; src: string; poster?: string; width?: number; height?: number };

export type XPost = {
  id: string;
  url: string;
  authorName: string;
  authorHandle: string;
  avatarUrl?: string;
  text: string;
  createdAt?: string;
  media: XPostMedia[];
};

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

/** The post id of an x.com or twitter.com status link. */
export const xStatusId = (value: string): string | undefined => {
  let url: URL;
  try {
    url = new URL(value);
  } catch {
    return undefined;
  }
  if (url.protocol !== 'https:' && url.protocol !== 'http:') return undefined;
  if (url.username || url.password || !X_HOSTS.has(url.hostname.toLowerCase())) return undefined;
  return STATUS_PATH.exec(url.pathname)?.[1];
};

/** The token X's embed endpoint expects, as X's own embed script derives it. */
export const xSyndicationToken = (id: string): string =>
  ((Number(id) / 1e15) * Math.PI).toString(36).replace(/(0+|\.)/g, '');

const asObject = (value: unknown): Record<string, unknown> | undefined =>
  value && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : undefined;

const asString = (value: unknown): string | undefined =>
  typeof value === 'string' && value.length > 0 ? value : undefined;

const asNumber = (value: unknown): number | undefined =>
  typeof value === 'number' && Number.isFinite(value) && value > 0 ? value : undefined;

const onHost = (value: unknown, host: string): string | undefined => {
  const url = asString(value);
  const safe = url && safeRemoteContentUrl(url);
  if (!safe) return undefined;
  return new URL(safe).hostname === host ? safe : undefined;
};

/** The highest-bitrate MP4 variant; HLS playlists are skipped. */
const bestMp4 = (videoInfo: Record<string, unknown> | undefined): string | undefined => {
  const variants = Array.isArray(videoInfo?.variants) ? videoInfo.variants : [];
  let best: { url: string; bitrate: number } | undefined;
  for (const raw of variants) {
    const variant = asObject(raw);
    if (variant?.content_type !== 'video/mp4') continue;
    const url = onHost(variant.url, VIDEO_HOST);
    if (!url) continue;
    const bitrate = typeof variant.bitrate === 'number' ? variant.bitrate : 0;
    if (!best || bitrate > best.bitrate) best = { url, bitrate };
  }
  return best?.url;
};

const parseMedia = (raw: unknown): XPostMedia | undefined => {
  const media = asObject(raw);
  if (!media) return undefined;
  const info = asObject(media.original_info);
  const width = asNumber(info?.width);
  const height = asNumber(info?.height);
  const image = onHost(media.media_url_https, IMAGE_HOST);
  if (media.type === 'photo') {
    return image ? { kind: 'photo', url: image, width, height } : undefined;
  }
  if (media.type === 'video' || media.type === 'animated_gif') {
    const src = bestMp4(asObject(media.video_info));
    if (!src) return undefined;
    return {
      kind: media.type === 'video' ? 'video' : 'gif',
      src,
      poster: image,
      width,
      height,
    };
  }
  return undefined;
};

/** Code points of `text` in `[start, end)`, as X's `display_text_range` counts them. */
const sliceCodePoints = (text: string, start: number, end: number): string =>
  Array.from(text).slice(start, end).join('');

const displayText = (post: Record<string, unknown>): string => {
  const raw = asString(post.text) ?? '';
  const range = Array.isArray(post.display_text_range) ? post.display_text_range : undefined;
  let text =
    range && typeof range[0] === 'number' && typeof range[1] === 'number'
      ? sliceCodePoints(raw, range[0], range[1])
      : raw;
  // t.co short links read as the links they stand for.
  const urls = asObject(post.entities)?.urls;
  if (Array.isArray(urls)) {
    for (const entry of urls) {
      const link = asObject(entry);
      const short = asString(link?.url);
      const expanded = asString(link?.expanded_url);
      if (short && expanded) text = text.split(short).join(expanded);
    }
  }
  text = text.replace(/&amp;/g, '&').replace(/&lt;/g, '<').replace(/&gt;/g, '>').trim();
  return text.length > MAX_TEXT_CHARS ? `${text.slice(0, MAX_TEXT_CHARS)}…` : text;
};

/** The card's view of a syndication response, or `undefined` for a deleted or unavailable post. */
export const parseXPost = (data: unknown, url: string): XPost | undefined => {
  const post = asObject(data);
  if (!post || post.__typename !== 'Tweet') return undefined;
  const id = asString(post.id_str);
  const user = asObject(post.user);
  const authorHandle = asString(user?.screen_name);
  if (!id || !authorHandle) return undefined;
  const media = (Array.isArray(post.mediaDetails) ? post.mediaDetails : [])
    .map(parseMedia)
    .filter((item): item is XPostMedia => !!item)
    .slice(0, MAX_MEDIA);
  return {
    id,
    url,
    authorName: asString(user?.name) ?? authorHandle,
    authorHandle,
    avatarUrl: onHost(user?.profile_image_url_https, IMAGE_HOST),
    text: displayText(post),
    createdAt: asString(post.created_at),
    media,
  };
};

/** Posts by id for this session, so scrolling past a link never refetches it. */
const postCache = new Map<string, Promise<XPost | undefined>>();

export const fetchXPost = (url: string, invoke: NativeInvoke): Promise<XPost | undefined> => {
  const id = xStatusId(url);
  if (!id) return Promise.resolve(undefined);
  const cached = postCache.get(id);
  if (cached) return cached;
  const task = (async () => {
    try {
      const result = await invoke('desktop_x_post', { id, token: xSyndicationToken(id) });
      if (!result.available || typeof result.value !== 'string') return undefined;
      return parseXPost(JSON.parse(result.value), url);
    } catch {
      return undefined;
    }
  })();
  postCache.set(id, task);
  // A failed fetch may succeed later; only a parsed post stays cached.
  void task.then((post) => {
    if (!post) postCache.delete(id);
  });
  return task;
};
