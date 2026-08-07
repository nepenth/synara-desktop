import React, { useCallback, useEffect, useMemo } from 'react';
import { Box, Text, color } from 'folds';
import { Link, useSearchParams } from 'react-router-dom';
import { useAuthServer } from '../../../hooks/useAuthServer';
import { AsyncStatus, useAsyncCallback } from '../../../hooks/useAsyncCallback';
import { useAutoDiscoveryInfo } from '../../../hooks/useAutoDiscoveryInfo';
import { PasswordRegisterForm, SUPPORTED_REGISTER_STAGES } from './PasswordRegisterForm';
import { SupportedUIAFlowsLoader } from '../../../components/SupportedUIAFlowsLoader';
import { getLoginPath } from '../../pathUtils';
import { RegisterPathSearchParams } from '../../paths';
import {
  probeRegisterFlows,
  uiaAuthDataFromProbe,
  type NativeRegisterFlowsProbe,
} from './nativeRegister';

const useRegisterSearchParams = (searchParams: URLSearchParams): RegisterPathSearchParams =>
  useMemo(
    () => ({
      username: searchParams.get('username') ?? undefined,
      email: searchParams.get('email') ?? undefined,
      token: searchParams.get('token') ?? undefined,
    }),
    [searchParams]
  );

export function Register() {
  const server = useAuthServer();
  const serverDiscovery = useAutoDiscoveryInfo();
  const baseUrl = serverDiscovery['m.homeserver'].base_url;
  const [searchParams] = useSearchParams();
  const registerSearchParams = useRegisterSearchParams(searchParams);

  const [flowsState, loadFlows] = useAsyncCallback<NativeRegisterFlowsProbe, Error, []>(
    useCallback(async () => probeRegisterFlows(baseUrl), [baseUrl])
  );

  useEffect(() => {
    loadFlows();
  }, [loadFlows]);

  if (flowsState.status === AsyncStatus.Idle || flowsState.status === AsyncStatus.Loading) {
    return (
      <Box direction="Column" gap="500">
        <Text size="H2" priority="400">
          Register
        </Text>
        <Text size="T300">Loading registration options…</Text>
      </Box>
    );
  }

  if (flowsState.status === AsyncStatus.Error) {
    return (
      <Box direction="Column" gap="500">
        <Text size="H2" priority="400">
          Register
        </Text>
        <Text style={{ color: color.Critical.Main }} size="T300">
          {flowsState.error.message || 'Failed to load registration options.'}
        </Text>
        <Text align="Center">
          Already have an account? <Link to={getLoginPath(server)}>Login</Link>
        </Text>
      </Box>
    );
  }

  const registerFlows = flowsState.data;

  return (
    <Box direction="Column" gap="500">
      <Text size="H2" priority="400">
        Register
      </Text>
      {registerFlows.status === 'registration_disabled' && (
        <Text style={{ color: color.Critical.Main }} size="T300">
          Registration has been disabled on this homeserver.
        </Text>
      )}
      {registerFlows.status === 'rate_limited' && (
        <Text style={{ color: color.Critical.Main }} size="T300">
          You have been rate-limited! Please try after some time.
        </Text>
      )}
      {registerFlows.status === 'invalid_request' && (
        <Text style={{ color: color.Critical.Main }} size="T300">
          Invalid Request! Failed to get any registration options.
        </Text>
      )}
      {registerFlows.status === 'flow_required' && (
        <>
          <SupportedUIAFlowsLoader
            flows={registerFlows.flows ?? []}
            supportedStages={[...SUPPORTED_REGISTER_STAGES]}
          >
            {(supportedFlows) =>
              supportedFlows.length === 0 ? (
                <Text style={{ color: color.Critical.Main }} size="T300">
                  This application does not support registration on this homeserver.
                </Text>
              ) : (
                <PasswordRegisterForm
                  authData={uiaAuthDataFromProbe(registerFlows)}
                  uiaFlows={supportedFlows}
                  defaultUsername={registerSearchParams.username}
                  defaultEmail={registerSearchParams.email}
                  defaultRegisterToken={registerSearchParams.token}
                />
              )
            }
          </SupportedUIAFlowsLoader>
          <span data-spacing-node />
        </>
      )}
      <Text align="Center">
        Already have an account? <Link to={getLoginPath(server)}>Login</Link>
      </Text>
    </Box>
  );
}
