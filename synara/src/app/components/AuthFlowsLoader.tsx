import { ReactNode, useCallback, useEffect } from 'react';
import { AsyncStatus, useAsyncCallback } from '../hooks/useAsyncCallback';
import { useAutoDiscoveryInfo } from '../hooks/useAutoDiscoveryInfo';
import { AuthFlows, RegisterFlowStatus, type RegisterFlowsResponse } from '../hooks/useAuthFlows';
import { discoverLoginFlows } from '../pages/auth/login/nativeLoginFlows';

type AuthFlowsLoaderProps = {
  fallback?: () => ReactNode;
  error?: (err: unknown) => ReactNode;
  children: (authFlows: AuthFlows) => ReactNode;
};

/**
 * Loads login-flow discovery for auth layout via native IPC (V-AUTH.3).
 *
 * Fail-closed: no matrix-js-sdk live client; transport / availability errors
 * surface through the existing error UI.
 *
 * Registration flow probe is owned natively by V-AUTH.4b (`Register.tsx` /
 * `matrix_register_flows`). A stub registerFlows value is retained for the
 * shared AuthFlows context shape used by login.
 */
export function AuthFlowsLoader({ fallback, error, children }: AuthFlowsLoaderProps) {
  const autoDiscoveryInfo = useAutoDiscoveryInfo();
  const baseUrl = autoDiscoveryInfo['m.homeserver'].base_url;

  const [state, load] = useAsyncCallback(
    useCallback(async () => {
      const loginFlows = await discoverLoginFlows(baseUrl);

      // Register discovery is native-owned (V-AUTH.4b). Stub keeps AuthFlows type stable for login.
      const registerFlows: RegisterFlowsResponse = {
        status: RegisterFlowStatus.InvalidRequest,
      };

      const authFlows: AuthFlows = {
        loginFlows,
        registerFlows,
      };

      return authFlows;
    }, [baseUrl])
  );

  useEffect(() => {
    load();
  }, [load]);

  if (state.status === AsyncStatus.Idle || state.status === AsyncStatus.Loading) {
    return fallback?.();
  }

  if (state.status === AsyncStatus.Error) {
    return error?.(state.error);
  }

  return children(state.data);
}
