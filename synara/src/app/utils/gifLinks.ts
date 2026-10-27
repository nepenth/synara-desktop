import { safeRemoteContentUrl } from './remoteContent';

/**
 * Media hosts whose GIF links Synara plays inline. Klipy and GIPHY require
 * their media to be loaded straight from these URLs, so a GIF sent from the
 * picker travels as its link (see `gifSendsAsLink`).
 */
const INLINE_GIF_HOSTS = [/^static\d*\.klipy\.com$/, /^media\d*\.giphy\.com$/, /^i\.giphy\.com$/];

const IMAGE_EXTENSIONS = /\.(gif|webp)$/i;

export type InlineGifLink = { url: string; host: string };

export const inlineGifLink = (value: string): InlineGifLink | undefined => {
  const safe = safeRemoteContentUrl(value);
  if (!safe) return undefined;
  let parsed: URL;
  try {
    parsed = new URL(safe);
  } catch {
    return undefined;
  }
  if (!INLINE_GIF_HOSTS.some((host) => host.test(parsed.hostname))) return undefined;
  if (!IMAGE_EXTENSIONS.test(parsed.pathname)) return undefined;
  // The provider URL is kept exactly as sent: its parameters carry delivery data.
  return { url: safe, host: parsed.hostname };
};

/** A message whose whole body is one inline GIF link, as the picker sends it. */
export const messageInlineGif = (body: string): InlineGifLink | undefined => {
  const trimmed = body.trim();
  if (!trimmed || /\s/.test(trimmed)) return undefined;
  return inlineGifLink(trimmed);
};
