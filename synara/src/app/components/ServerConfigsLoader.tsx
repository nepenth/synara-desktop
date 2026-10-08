import { ReactNode, useCallback, useMemo } from 'react';
import type { Capabilities } from '../hooks/useCapabilities';

/** Structural subset of the js-sdk ValidatedAuthMetadata used by Synara. */
export type ValidatedAuthMetadata = {
  issuer?: string;
  account_management_uri?: string;
  homeserver_url?: string;
};
import { AsyncStatus, useAsyncCallbackValue } from '../hooks/useAsyncCallback';
import { MediaConfig } from '../hooks/useMediaConfig';

import { getNativeMediaConfig } from '../native/nativeCommands';
export type ServerConfigs = {
  capabilities?: Capabilities;
  mediaConfig?: MediaConfig;
  authMetadata?: ValidatedAuthMetadata;
};

type ServerConfigsLoaderProps = {
  children: (configs: ServerConfigs) => ReactNode;
};
export function ServerConfigsLoader({ children }: ServerConfigsLoaderProps) {
  const fallbackConfigs = useMemo(() => ({}), []);

  const [configsState] = useAsyncCallbackValue<ServerConfigs, unknown>(
    // Native reports the media upload limit. Server capabilities and OIDC auth
    // metadata have no native read, so their providers receive empty values.
    useCallback(async () => {
      const [mediaConfig] = await Promise.allSettled([getNativeMediaConfig()]);
      return {
        capabilities: {},
        mediaConfig: mediaConfig.status === 'fulfilled' ? mediaConfig.value : undefined,
        authMetadata: undefined,
      };
    }, [])
  );

  const configs: ServerConfigs =
    configsState.status === AsyncStatus.Success ? configsState.data : fallbackConfigs;

  return children(configs);
}
