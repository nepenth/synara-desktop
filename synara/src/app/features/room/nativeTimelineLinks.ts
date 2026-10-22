import {
  parseMatrixToRoom,
  parseMatrixToRoomEvent,
  parseMatrixToUser,
  testMatrixTo,
} from '../../plugins/matrix-to';
import { nativeThumbnailContentUri } from '../../matrix/nativeThumbnail';

/** Where a matrix.to permalink in a message points. */
export type NativePermalinkTarget =
  | { kind: 'user'; userId: string }
  | { kind: 'room'; roomIdOrAlias: string; eventId?: string; viaServers?: string[] };

const decodeHref = (href: string): string => {
  try {
    return decodeURIComponent(href);
  } catch {
    return href;
  }
};

/** A matrix.to user, room or event permalink, or undefined for any other link. */
export const parseNativePermalink = (href: string): NativePermalinkTarget | undefined => {
  const decoded = decodeHref(href);
  if (!testMatrixTo(decoded)) return undefined;
  const userId = parseMatrixToUser(decoded);
  if (userId) return { kind: 'user', userId };
  const roomEvent = parseMatrixToRoomEvent(decoded);
  if (roomEvent) {
    return {
      kind: 'room',
      roomIdOrAlias: roomEvent.roomIdOrAlias,
      eventId: roomEvent.eventId,
      viaServers: roomEvent.viaServers,
    };
  }
  const room = parseMatrixToRoom(decoded);
  if (room) {
    return { kind: 'room', roomIdOrAlias: room.roomIdOrAlias, viaServers: room.viaServers };
  }
  return undefined;
};

/** True when formatted HTML links a mention of `userId` (a matrix.to pill). */
export const nativeFormattedBodyMentionsUser = (
  html: string | undefined,
  userId: string | undefined
): boolean => {
  if (!html || !userId) return false;
  return (
    html.includes(`matrix.to/#/${userId}`) ||
    html.includes(`matrix.to/#/${encodeURIComponent(userId)}`)
  );
};

const hostOf = (value: string): string | undefined => {
  try {
    const url = new URL(value);
    return url.protocol === 'http:' || url.protocol === 'https:'
      ? url.hostname.toLowerCase().replace(/^www\./, '')
      : undefined;
  } catch {
    return undefined;
  }
};

/** Links the desktop opener handles: web and mail (and matrix.to, which is https). */
export const isNativeOpenableHref = (href: string): boolean => /^(https?|mailto):/i.test(href);

const LOOKS_LIKE_URL = /^(https?:\/\/)?([a-z0-9-]+\.)+[a-z]{2,}(\/\S*)?$/i;

/**
 * The real destination host when a link's visible text is itself a web
 * address for a different host (`<a href="https://evil">https://bank</a>`).
 * Undefined when the text is ordinary words or names the same host.
 */
export const nativeLinkHostMismatch = (text: string, href: string): string | undefined => {
  const destination = hostOf(href);
  if (!destination) return undefined;
  const shown = text.trim();
  if (!LOOKS_LIKE_URL.test(shown)) return undefined;
  const shownHost = hostOf(/^https?:\/\//i.test(shown) ? shown : `https://${shown}`);
  if (!shownHost || shownHost === destination) return undefined;
  return destination;
};

/** A custom emoji source: an `mxc://` URI and nothing else. */
const MXC_URI = /^mxc:\/\/[A-Za-z0-9.:[\]-]+\/[A-Za-z0-9_-]+$/;

export const isNativeCustomEmojiSource = (src: string | undefined): src is string =>
  typeof src === 'string' && MXC_URI.test(src);

/** Custom emoji render about one line high; ask Core for a small thumbnail. */
export const NATIVE_CUSTOM_EMOJI_THUMBNAIL_PX = 48;

export const nativeCustomEmojiContentUri = (mxc: string): string =>
  nativeThumbnailContentUri(
    mxc,
    NATIVE_CUSTOM_EMOJI_THUMBNAIL_PX,
    NATIVE_CUSTOM_EMOJI_THUMBNAIL_PX,
    'scale'
  );
