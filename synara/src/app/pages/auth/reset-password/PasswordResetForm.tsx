import React, { FormEventHandler, useCallback, useEffect, useRef, useState } from 'react';
import {
  Box,
  Button,
  Dialog,
  Input,
  Overlay,
  OverlayBackdrop,
  OverlayCenter,
  Spinner,
  Text,
  color,
  config,
} from 'folds';
import { useNavigate } from 'react-router-dom';
import FocusTrap from 'focus-trap-react';
import { useAutoDiscoveryInfo } from '../../../hooks/useAutoDiscoveryInfo';
import { AsyncStatus, useAsyncCallback } from '../../../hooks/useAsyncCallback';
import { useAuthServer } from '../../../hooks/useAuthServer';
import { PasswordInput } from '../../../components/password-input';
import { ConfirmPasswordMatch } from '../../../components/ConfirmPasswordMatch';
import { FieldError } from '../FiledError';
import { UIAFlowOverlay } from '../../../components/UIAFlowOverlay';
import { getLoginPath, withSearchParam } from '../../pathUtils';
import { LoginPathSearchParams } from '../../paths';
import {
  completePasswordReset,
  generatePasswordResetClientSecret,
  requestPasswordResetEmailToken,
  type NativePasswordEmailTokenResult,
  type NativePasswordResetOutcome,
} from './nativePasswordReset';

type FormData = {
  email: string;
  password: string;
  clientSecret: string;
};

function ResetPasswordComplete({ email }: { email?: string }) {
  const server = useAuthServer();
  const navigate = useNavigate();

  const handleClick = () => {
    const path = getLoginPath(server);
    if (email) {
      navigate(withSearchParam<LoginPathSearchParams>(path, { email }));
      return;
    }
    navigate(path);
  };

  return (
    <Overlay open backdrop={<OverlayBackdrop />}>
      <OverlayCenter>
        <FocusTrap>
          <Dialog>
            <Box style={{ padding: config.space.S400 }} direction="Column" gap="400">
              <Text>
                Password has been reset successfully. Please login with your new password.
              </Text>
              <Button variant="Primary" onClick={handleClick}>
                <Text size="B400" as="span">
                  Login
                </Text>
              </Button>
            </Box>
          </Dialog>
        </FocusTrap>
      </OverlayCenter>
    </Overlay>
  );
}

function EmailVerifyDialog({
  email,
  errorCode,
  errorMessage,
  onContinue,
  onResend,
  onCancel,
  busy,
}: {
  email: string;
  errorCode?: string | null;
  errorMessage?: string | null;
  onContinue: () => void;
  onResend: () => void;
  onCancel: () => void;
  busy: boolean;
}) {
  return (
    <Dialog>
      <Box style={{ padding: config.space.S400 }} direction="Column" gap="400">
        <Box direction="Column" gap="100">
          <Text size="H4">Verification Request Sent</Text>
          <Text>
            {`Please check your email "${email}" and validate before continuing further.`}
          </Text>
          {(errorCode || errorMessage) && (
            <Text style={{ color: color.Critical.Main }}>
              {errorCode ? `${errorCode}: ` : ''}
              {errorMessage ?? 'Email has not been verified yet.'}
            </Text>
          )}
        </Box>
        <Button variant="Primary" onClick={onContinue} disabled={busy}>
          <Text as="span" size="B400">
            Continue
          </Text>
        </Button>
        <Button variant="Secondary" fill="Soft" onClick={onResend} disabled={busy}>
          <Text as="span" size="B400">
            Resend Email
          </Text>
        </Button>
        <Button variant="Critical" fill="None" outlined type="button" onClick={onCancel}>
          <Text as="span" size="B400">
            Cancel
          </Text>
        </Button>
      </Box>
    </Dialog>
  );
}

type PasswordResetFormProps = {
  defaultEmail?: string;
};

export function PasswordResetForm({ defaultEmail }: PasswordResetFormProps) {
  const server = useAuthServer();
  const serverDiscovery = useAutoDiscoveryInfo();
  const baseUrl = serverDiscovery['m.homeserver'].base_url;

  const [formData, setFormData] = useState<FormData>();
  const [verifyError, setVerifyError] = useState<{
    errorCode?: string | null;
    errorMessage?: string | null;
  }>();
  const sendAttemptRef = useRef(1);

  const [passwordEmailState, requestEmail] = useAsyncCallback<
    NativePasswordEmailTokenResult,
    Error,
    [string, string]
  >(
    useCallback(
      async (email, clientSecret) => {
        const sendAttempt = sendAttemptRef.current;
        sendAttemptRef.current += 1;
        return requestPasswordResetEmailToken(baseUrl, email, clientSecret, sendAttempt);
      },
      [baseUrl]
    )
  );

  const [resetPasswordState, handleResetPassword] = useAsyncCallback<
    NativePasswordResetOutcome,
    Error,
    [FormData, string]
  >(
    useCallback(
      async (data, sid) =>
        completePasswordReset(baseUrl, data.email, data.password, data.clientSecret, sid),
      [baseUrl]
    )
  );

  useEffect(() => {
    if (
      resetPasswordState.status === AsyncStatus.Success &&
      resetPasswordState.data.status === 'email_not_verified'
    ) {
      setVerifyError({
        errorCode: resetPasswordState.data.errorCode,
        errorMessage: resetPasswordState.data.errorMessage,
      });
    }
  }, [resetPasswordState]);

  const resetComplete =
    resetPasswordState.status === AsyncStatus.Success &&
    resetPasswordState.data.status === 'complete';

  const resetPasswordError =
    resetPasswordState.status === AsyncStatus.Error ? resetPasswordState.error : undefined;

  const emailToken =
    passwordEmailState.status === AsyncStatus.Success ? passwordEmailState.data : undefined;

  const handleSubmit: FormEventHandler<HTMLFormElement> = (evt) => {
    evt.preventDefault();
    const { emailInput, passwordInput, confirmPasswordInput } = evt.target as HTMLFormElement & {
      emailInput: HTMLInputElement;
      passwordInput: HTMLInputElement;
      confirmPasswordInput: HTMLInputElement;
    };

    const email = emailInput.value.trim();
    const password = passwordInput.value;
    const confirmPassword = confirmPasswordInput.value;
    if (!email) {
      emailInput.focus();
      return;
    }
    if (password !== confirmPassword) return;

    const clientSecret = generatePasswordResetClientSecret();
    setFormData({ email, password, clientSecret });
    setVerifyError(undefined);
    requestEmail(email, clientSecret);
  };

  const handleCancel = () => {
    window.location.reload();
  };

  const handleContinue = () => {
    if (!formData || !emailToken) return;
    setVerifyError(undefined);
    handleResetPassword(formData, emailToken.sid);
  };

  const handleResend = () => {
    if (!formData) return;
    setVerifyError(undefined);
    requestEmail(formData.email, formData.clientSecret);
  };

  const showEmailOverlay =
    Boolean(formData) &&
    Boolean(emailToken) &&
    !resetComplete &&
    passwordEmailState.status !== AsyncStatus.Loading;

  const busy =
    passwordEmailState.status === AsyncStatus.Loading ||
    resetPasswordState.status === AsyncStatus.Loading;

  return (
    <Box as="form" onSubmit={handleSubmit} direction="Inherit" gap="400">
      <Text size="T300" priority="400">
        Homeserver <strong>{server}</strong> will send you an email to let you reset your password.
      </Text>
      <Box direction="Column" gap="100">
        <Text as="label" size="L400" priority="300">
          Email
        </Text>
        <Input
          defaultValue={defaultEmail}
          type="email"
          name="emailInput"
          variant="Background"
          size="500"
          required
          outlined
        />
        {passwordEmailState.status === AsyncStatus.Error && (
          <FieldError message={passwordEmailState.error.message || 'Failed to send reset email.'} />
        )}
      </Box>
      <ConfirmPasswordMatch initialValue>
        {(match, doMatch, passRef, confPassRef) => (
          <>
            <Box direction="Column" gap="100">
              <Text as="label" size="L400" priority="300">
                New Password
              </Text>
              <PasswordInput
                ref={passRef}
                onChange={doMatch}
                name="passwordInput"
                variant="Background"
                size="500"
                outlined
                required
              />
            </Box>
            <Box direction="Column" gap="100">
              <Text as="label" size="L400" priority="300">
                Confirm Password
              </Text>
              <PasswordInput
                ref={confPassRef}
                onChange={doMatch}
                name="confirmPasswordInput"
                variant="Background"
                size="500"
                style={{ color: match ? undefined : color.Critical.Main }}
                outlined
                required
              />
            </Box>
          </>
        )}
      </ConfirmPasswordMatch>
      {resetPasswordError && (
        <FieldError message={resetPasswordError.message || 'Failed to reset password.'} />
      )}
      <span data-spacing-node />
      <Button type="submit" variant="Primary" size="500">
        <Text as="span" size="B500">
          Reset Password
        </Text>
      </Button>

      {resetComplete && <ResetPasswordComplete email={formData?.email} />}

      {showEmailOverlay && formData && emailToken && (
        <UIAFlowOverlay currentStep={1} stepCount={1} onCancel={handleCancel}>
          <EmailVerifyDialog
            email={formData.email}
            errorCode={verifyError?.errorCode}
            errorMessage={verifyError?.errorMessage}
            onContinue={handleContinue}
            onResend={handleResend}
            onCancel={handleCancel}
            busy={busy}
          />
        </UIAFlowOverlay>
      )}

      <Overlay open={busy} backdrop={<OverlayBackdrop />}>
        <OverlayCenter>
          <Spinner variant="Secondary" size="600" />
        </OverlayCenter>
      </Overlay>
    </Box>
  );
}
