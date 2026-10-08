import { ReactNode, useCallback, useEffect } from 'react';
import { AsyncStatus, useAsyncCallback } from '../hooks/useAsyncCallback';
import { MediaConfig } from '../hooks/useMediaConfig';

import { getNativeMediaConfig } from '../native/nativeCommands';
type MediaConfigLoaderProps = {
  children: (mediaConfig: MediaConfig | undefined) => ReactNode;
};
export function MediaConfigLoader({ children }: MediaConfigLoaderProps) {
  const [state, load] = useAsyncCallback(useCallback(() => getNativeMediaConfig(), []));

  useEffect(() => {
    load();
  }, [load]);

  return children(state.status === AsyncStatus.Success ? state.data : undefined);
}
