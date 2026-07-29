import React, { useMemo } from 'react';
import { Box, Text, color } from 'folds';
import { Link, useSearchParams } from 'react-router-dom';
import { useAuthFlows } from '../../../hooks/useAuthFlows';
import { useAuthServer } from '../../../hooks/useAuthServer';
import { useParsedLoginFlows } from '../../../hooks/useParsedLoginFlows';
import { PasswordLoginForm } from './PasswordLoginForm';
import { getLoginPath, getRegisterPath } from '../../pathUtils';
import { LoginPathSearchParams } from '../../paths';

const useLoginSearchParams = (searchParams: URLSearchParams): LoginPathSearchParams =>
  useMemo(
    () => ({
      username: searchParams.get('username') ?? undefined,
      email: searchParams.get('email') ?? undefined,
    }),
    [searchParams]
  );

export function Login() {
  const server = useAuthServer();
  const { loginFlows } = useAuthFlows();
  const [searchParams] = useSearchParams();
  const loginSearchParams = useLoginSearchParams(searchParams);
  const parsedFlows = useParsedLoginFlows(loginFlows.flows);

  return (
    <Box direction="Column" gap="500">
      <Text size="H2" priority="400">
        Login
      </Text>
      {parsedFlows.password && (
        <>
          <PasswordLoginForm
            defaultUsername={loginSearchParams.username}
            defaultEmail={loginSearchParams.email}
          />
          <span data-spacing-node />
        </>
      )}
      {!parsedFlows.password && (
        <>
          <Text style={{ color: color.Critical.Main }}>
            {`This client does not support login on "${server}" homeserver. Password based login method not found.`}
          </Text>
          <span data-spacing-node />
        </>
      )}
      <Text align="Center">
        Do not have an account? <Link to={getRegisterPath(server)}>Register</Link>
      </Text>
    </Box>
  );
}
