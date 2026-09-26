import { isNativeMediaContentUri, nativeMediaDisplayUrl } from '../matrix/media';

/**
 * Native avatars and packs load through the synara-media protocol. The URL is
 * derived from the content URI, so the first paint does not wait on a byte
 * download and the decrypted file never enters the JavaScript heap.
 */
export function useNativeMatrixMediaSrc(contentUri: string | undefined): string | undefined {
  if (!contentUri || !isNativeMediaContentUri(contentUri)) return contentUri;
  return nativeMediaDisplayUrl(contentUri);
}
