import { Box, Text } from 'folds';
import React, { useMemo } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { getLoginPath } from '../../pathUtils';
import { useAuthServer } from '../../../hooks/useAuthServer';
import { PasswordResetForm } from './PasswordResetForm';
import { ResetPasswordPathSearchParams } from '../../paths';
import * as css from '../styles.css';

const useResetPasswordSearchParams = (
  searchParams: URLSearchParams
): ResetPasswordPathSearchParams =>
  useMemo(
    () => ({
      email: searchParams.get('email') ?? undefined,
    }),
    [searchParams]
  );

export function ResetPassword() {
  const server = useAuthServer();
  const [searchParams] = useSearchParams();
  const resetPasswordSearchParams = useResetPasswordSearchParams(searchParams);

  return (
    <Box direction="Column" gap="500">
      <div className={css.AuthHeading}>
        <Text as="h2" size="H3" priority="400">
          Reset your password
        </Text>
        <Text size="T300" priority="300">
          We&apos;ll email you a link to choose a new password on {server}.
        </Text>
      </div>
      <PasswordResetForm defaultEmail={resetPasswordSearchParams.email} />
      <span data-spacing-node />

      <Text align="Center">
        Remember your password? <Link to={getLoginPath(server)}>Sign in</Link>
      </Text>
    </Box>
  );
}
