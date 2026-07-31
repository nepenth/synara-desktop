import { ReactNode, useCallback, useEffect, useMemo } from 'react';
import { createClient } from 'matrix-js-sdk';
import { AsyncStatus, useAsyncCallback } from '../hooks/useAsyncCallback';
import { useAutoDiscoveryInfo } from '../hooks/useAutoDiscoveryInfo';
import { promiseFulfilledResult } from '../utils/common';
import {
  AuthFlows,
  RegisterFlowStatus,
  type RegisterFlowsResponse,
} from '../hooks/useAuthFlows';

type AuthFlowsLoaderProps = {
  fallback?: () => ReactNode;
  error?: (err: unknown) => ReactNode;
  children: (authFlows: AuthFlows) => ReactNode;
};

/**
 * Loads login-flow discovery for auth layout.
 *
 * Registration flow probe is owned natively by V-AUTH.4b (`Register.tsx` /
 * `matrix_register_flows`) and is no longer loaded via matrix-js-sdk here.
 * A stub registerFlows value is retained for the shared AuthFlows context shape
 * used by login until login-flow discovery is re-homed.
 */
export function AuthFlowsLoader({ fallback, error, children }: AuthFlowsLoaderProps) {
  const autoDiscoveryInfo = useAutoDiscoveryInfo();
  const baseUrl = autoDiscoveryInfo['m.homeserver'].base_url;

  const mx = useMemo(() => createClient({ baseUrl }), [baseUrl]);

  const [state, load] = useAsyncCallback(
    useCallback(async () => {
      const result = await Promise.allSettled([mx.loginFlows()]);
      const loginFlows = promiseFulfilledResult(result[0]);

      if (!loginFlows) {
        throw new Error('Missing auth flow!');
      }
      if ('errcode' in loginFlows) {
        throw new Error('Failed to load auth flow!');
      }

      // Register discovery is native-owned (V-AUTH.4b). Stub keeps AuthFlows type stable for login.
      const registerFlows: RegisterFlowsResponse = {
        status: RegisterFlowStatus.InvalidRequest,
      };

      const authFlows: AuthFlows = {
        loginFlows,
        registerFlows,
      };

      return authFlows;
    }, [mx])
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
