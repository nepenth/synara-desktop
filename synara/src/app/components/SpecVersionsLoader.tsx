import { ReactNode, useCallback, useEffect, useState } from 'react';
import { AsyncStatus, useAsyncCallback } from '../hooks/useAsyncCallback';
import { SpecVersions, specVersions } from '../cs-api';

type SpecVersionsLoaderProps = {
  baseUrl: string;
  blocking?: boolean;
  fallback?: () => ReactNode;
  error?: (err: unknown, retry: () => void, ignore: () => void) => ReactNode;
  children: (versions: SpecVersions) => ReactNode;
};
export function SpecVersionsLoader({
  baseUrl,
  blocking = true,
  fallback,
  error,
  children,
}: SpecVersionsLoaderProps) {
  const [state, load] = useAsyncCallback(
    useCallback(() => specVersions(fetch, baseUrl), [baseUrl])
  );
  const [ignoreError, setIgnoreError] = useState(false);

  const ignoreCallback = useCallback(() => setIgnoreError(true), []);
  const retry = useCallback(() => {
    void load().catch(() => {
      // The loader projects errors into UI or optional metadata below.
    });
  }, [load]);

  useEffect(() => {
    retry();
  }, [retry]);

  if (blocking && (state.status === AsyncStatus.Idle || state.status === AsyncStatus.Loading)) {
    return fallback?.();
  }

  if (blocking && !ignoreError && state.status === AsyncStatus.Error) {
    return error?.(state.error, retry, ignoreCallback);
  }

  return children(
    state.status === AsyncStatus.Success
      ? state.data
      : {
          versions: [],
        }
  );
}
