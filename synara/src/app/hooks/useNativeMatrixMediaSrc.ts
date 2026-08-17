import { useEffect, useState } from 'react';
import { createMatrixMediaObjectUrl, isNativeMediaContentUri } from '../matrix/media';
import { useMaybeMatrixClient } from './useMatrixClient';

/**
 * Resolve leftover `mxc://` / handle URIs through native download.
 * Blob, data, and http(s) URLs pass through. Missing client or a failed
 * native download yields undefined so the avatar fallback can render.
 */
export function useNativeMatrixMediaSrc(
  contentUri: string | undefined,
  mimeType = 'image/jpeg'
): string | undefined {
  const mx = useMaybeMatrixClient();
  const [objectUrl, setObjectUrl] = useState<string | undefined>(() =>
    contentUri && !isNativeMediaContentUri(contentUri) ? contentUri : undefined
  );

  useEffect(() => {
    if (!contentUri) {
      setObjectUrl(undefined);
      return undefined;
    }
    if (!isNativeMediaContentUri(contentUri)) {
      setObjectUrl(contentUri);
      return undefined;
    }
    if (!mx) {
      setObjectUrl(undefined);
      return undefined;
    }

    let cancelled = false;
    let created: string | undefined;
    setObjectUrl(undefined);
    void createMatrixMediaObjectUrl(mx, contentUri, { mimeType })
      .then((url) => {
        if (cancelled) {
          URL.revokeObjectURL(url);
          return;
        }
        created = url;
        setObjectUrl(url);
      })
      .catch(() => {
        if (!cancelled) setObjectUrl(undefined);
      });

    return () => {
      cancelled = true;
      if (created) URL.revokeObjectURL(created);
    };
  }, [mx, contentUri, mimeType]);

  return objectUrl;
}
