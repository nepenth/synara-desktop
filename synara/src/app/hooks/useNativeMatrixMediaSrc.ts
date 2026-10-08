import {
  isNativeMediaContentUri,
  nativeMediaDisplayUrl,
  nativeThumbnailContentUri,
  type NativeThumbnailMethod,
} from '../matrix/media';

export type NativeMediaThumbnail = {
  /** Square size in CSS pixels, or width when `height` is set. */
  size: number;
  height?: number;
  method?: NativeThumbnailMethod;
};

/**
 * Native avatars and packs load through the synara-media protocol. The URL is
 * derived from the content URI, so the first paint does not wait on a byte
 * download and the decrypted file never enters the JavaScript heap. With a
 * `thumbnail`, a raw `mxc://` source asks the homeserver for a sized
 * thumbnail instead of the original upload.
 */
export function useNativeMatrixMediaSrc(
  contentUri: string | undefined,
  thumbnail?: NativeMediaThumbnail
): string | undefined {
  if (!contentUri || !isNativeMediaContentUri(contentUri)) return contentUri;
  const source = thumbnail
    ? nativeThumbnailContentUri(
        contentUri,
        thumbnail.size,
        thumbnail.height ?? thumbnail.size,
        thumbnail.method ?? 'crop'
      )
    : contentUri;
  return nativeMediaDisplayUrl(source);
}
