import { useCallback, useLayoutEffect, useSyncExternalStore } from 'react';
import { downloadMatrixMedia, isNativeMediaContentUri } from '../matrix/media';
import { getClientMediaObjectUrlCache } from '../matrix/mediaObjectUrlCache';
import { useMaybeMatrixClient } from './useMatrixClient';

/**
 * Resolve native URIs using a client-scoped URL lease. Warm avatars have a src
 * on their first render when returning to a room; concurrent avatars share IPC.
 * Other URL types pass through and are never owned/revoked by this hook.
 */
export function useNativeMatrixMediaSrc(
  contentUri: string | undefined,
  mimeType = 'image/jpeg'
): string | undefined {
  const mx = useMaybeMatrixClient();
  const native = isNativeMediaContentUri(contentUri);
  const cache = mx && native ? getClientMediaObjectUrlCache(mx) : undefined;
  const key = JSON.stringify([contentUri, mimeType]);
  const subscribe = useCallback(
    (listener: () => void) => cache?.subscribe(key, listener) ?? (() => undefined),
    [cache, key]
  );
  const getSnapshot = useCallback(
    () => (native ? cache?.peek(key) : contentUri),
    [cache, key, native, contentUri]
  );
  // React rechecks the snapshot before commit, including concurrent renders
  // whose cached URL expired while rendering. No lease is taken during render.
  const src = useSyncExternalStore(subscribe, getSnapshot, getSnapshot);

  // A failed entry may be replaced by a later avatar's successful retry. Its
  // new src snapshot must acquire a lease for this still-mounted avatar too.
  useLayoutEffect(() => {
    if (!mx || !cache || !contentUri) return undefined;
    const lease = cache.acquire(
      key,
      () => downloadMatrixMedia(mx, contentUri, { mimeType }),
      // MXC sources are immutable. Opaque native handles can be rebound or
      // revoked with their timeline and must resolve again after unmount.
      contentUri.trim().startsWith('mxc://')
    );
    // Successful downloads and eviction notify the external-store snapshot.
    // Failed downloads keep its undefined fallback and remain retryable.
    void lease.promise.catch(() => undefined);
    return lease.release;
  }, [mx, cache, key, contentUri, mimeType, src]);

  return src;
}
