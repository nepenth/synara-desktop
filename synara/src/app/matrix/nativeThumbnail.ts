/**
 * Native thumbnail request carried in the synara-media path:
 * `thumbnail/<width>x<height>/<crop|scale>/<mxc:// or timeline handle>`.
 * Rust snaps the size to a small bucket set and falls back to the original
 * when the homeserver cannot thumbnail (for example, encrypted media).
 *
 * No imports: the native client facade depends on this module.
 */
export const NATIVE_THUMBNAIL_PREFIX = 'thumbnail/';
export const NATIVE_TIMELINE_MEDIA_HANDLE_PREFIX = 'timeline-media-';
const NATIVE_THUMBNAIL_MAX_PX = 4096;

/** Avatar thumbnails in CSS pixels; device pixel ratio is applied on top. */
export const NATIVE_AVATAR_THUMBNAIL_PX = 96;

export type NativeThumbnailMethod = 'crop' | 'scale';

export const devicePixelScale = (): number => {
  const ratio = typeof window === 'undefined' ? 1 : window.devicePixelRatio;
  return Number.isFinite(ratio) && ratio > 1 ? Math.min(3, Math.ceil(ratio)) : 1;
};

const thumbnailPixels = (cssPixels: number, scale: number): number | undefined => {
  if (!Number.isFinite(cssPixels) || cssPixels <= 0) return undefined;
  return Math.min(NATIVE_THUMBNAIL_MAX_PX, Math.ceil(cssPixels * scale));
};

/**
 * Wrap a native media source in a thumbnail request. Only `mxc://` and opaque
 * timeline handles qualify; anything else (and an existing thumbnail) is
 * returned unchanged so callers can apply this unconditionally.
 */
export function nativeThumbnailContentUri(
  contentUri: string,
  width: number,
  height: number,
  method: NativeThumbnailMethod = 'crop',
  scale: number = devicePixelScale()
): string {
  const trimmed = contentUri.trim();
  if (trimmed.startsWith(NATIVE_THUMBNAIL_PREFIX)) return trimmed;
  if (!trimmed.startsWith('mxc://') && !trimmed.startsWith(NATIVE_TIMELINE_MEDIA_HANDLE_PREFIX)) {
    return contentUri;
  }
  const w = thumbnailPixels(width, scale);
  const h = thumbnailPixels(height, scale);
  if (!w || !h) return contentUri;
  return `${NATIVE_THUMBNAIL_PREFIX}${w}x${h}/${method}/${trimmed}`;
}

/** The original media behind an optional thumbnail wrapper. */
export function stripNativeThumbnail(contentUri: string): string {
  if (!contentUri.startsWith(NATIVE_THUMBNAIL_PREFIX)) return contentUri;
  return contentUri.slice(NATIVE_THUMBNAIL_PREFIX.length).split('/').slice(2).join('/');
}
