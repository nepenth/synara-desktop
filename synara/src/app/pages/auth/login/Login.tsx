import React, { useMemo } from 'react';
import { Box, Text, color } from 'folds';
import * as css from '../styles.css';
import { usePickedAuthServer } from '../authServerResolution';
import { Link, useSearchParams } from 'react-router-dom';
import { useAuthFlows } from '../../../hooks/useAuthFlows';
import { useAuthServer } from '../../../hooks/useAuthServer';
import { useParsedLoginFlows } from '../../../hooks/useParsedLoginFlows';
import { PasswordLoginForm } from './PasswordLoginForm';
import { getRegisterPath } from '../../pathUtils';
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
  const pickedServer = usePickedAuthServer(server);
  const { loginFlows } = useAuthFlows();
  const [searchParams] = useSearchParams();
  const loginSearchParams = useLoginSearchParams(searchParams);
  const parsedFlows = useParsedLoginFlows(loginFlows.flows);

  return (
    <Box direction="Column" gap="500">
      <div className={css.AuthHeading}>
        <Text as="h2" size="H3" priority="400">
          Welcome back
        </Text>
        <Text size="T300" priority="300">
          Sign in to your account on {pickedServer}.
        </Text>
      </div>
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
        New to {pickedServer}? <Link to={getRegisterPath(server)}>Create an account</Link>
      </Text>
    </Box>
  );
}
