import React, { FormEventHandler, useEffect, useId, useRef, useState } from 'react';
import {
  Badge,
  Box,
  Button,
  config,
  Header,
  Icon,
  Icons,
  Spinner,
  Text,
  Overlay,
  OverlayBackdrop,
  OverlayCenter,
  IconButton,
} from 'folds';
import { Dialog } from '../../../components/modal-dialog';
import FocusTrap from '../../../components/FocusTrap';
import { VerificationStatus } from './nativeDevices';
import { InfoCard } from '../../../components/info-card';
import { NativeStartVerification } from '../../verification/NativeDeviceVerification';
import {
  authenticateNativeCrossSigningSetup,
  NativeCrossSigningStatus,
  startNativeCrossSigningSetup,
} from '../../cross-signing/nativeCrossSigning';
import { PasswordInput } from '../../../components/password-input';
import { NativeSecretStorageGate } from '../../../components/SecretStorage';
import { currentDeviceVerificationAvailabilityMessage } from './deviceVerificationStatus';
import { composerErrorMessage } from '../../room/composerSendError';
import { InlineMessage } from '../../../components/inline-message';
import { stopPropagation } from '../../../utils/keyboard';

type VerificationStatusBadgeProps = {
  verificationStatus: VerificationStatus;
  otherUnverifiedCount?: number;
};
export function VerificationStatusBadge({
  verificationStatus,
  otherUnverifiedCount,
}: VerificationStatusBadgeProps) {
  if (verificationStatus === 'unknown') {
    return <Spinner size="400" variant="Secondary" />;
  }
  const unverifiedCount = otherUnverifiedCount ?? 0;
  if (verificationStatus === 'unverified') {
    return (
      <Badge variant="Critical" fill="Solid" size="500">
        <Text size="L400">Not verified</Text>
      </Badge>
    );
  }

  if (unverifiedCount > 0) {
    return (
      <Badge variant="Warning" fill="Solid" size="500">
        <Text size="L400">{unverifiedCount} not verified</Text>
      </Badge>
    );
  }

  return (
    <Badge variant="Success" fill="Solid" size="500">
      <Text size="L400">Verified</Text>
    </Badge>
  );
}

export function VerifyCurrentDeviceTile({
  hasDevicesToVerifyAgainst,
  canStart,
  refreshing,
  onRetry,
  onVerified,
}: {
  hasDevicesToVerifyAgainst: boolean | null;
  canStart: boolean;
  refreshing?: boolean;
  onRetry?: () => void;
  onVerified?: () => void;
}) {
  // Both "could not check" (null) and "no eligible session yet" (false) are
  // recoverable by re-reading the snapshot, so both offer the same action.
  const retry = hasDevicesToVerifyAgainst !== true && onRetry;
  return (
    <InfoCard
      variant="Warning"
      title="This session is not verified"
      description={currentDeviceVerificationAvailabilityMessage(hasDevicesToVerifyAgainst)}
      after={
        canStart ? (
          <NativeStartVerification onExit={() => onVerified?.()} />
        ) : retry ? (
          <Button
            size="300"
            radii="300"
            disabled={refreshing}
            before={refreshing ? <Spinner size="100" variant="Secondary" fill="Soft" /> : undefined}
            onClick={onRetry}
          >
            <Text as="span" size="B300">
              {hasDevicesToVerifyAgainst === null ? 'Retry check' : 'Refresh'}
            </Text>
          </Button>
        ) : undefined
      }
    />
  );
}

export function VerifyOtherDeviceTile({
  deviceId,
  onVerified,
}: {
  deviceId: string;
  onVerified?: () => void;
}) {
  return (
    <InfoCard
      variant="Warning"
      title="Not verified"
      description="Verify this session so it is trusted and can read encrypted messages."
      after={<NativeStartVerification deviceId={deviceId} onExit={() => onVerified?.()} />}
    />
  );
}

type EnableVerificationProps = {
  visible: boolean;
  nativeStatus?: NativeCrossSigningStatus;
  loading?: boolean;
  error?: string;
};
export function EnableVerification({
  visible,
  nativeStatus,
  loading,
  error,
}: EnableVerificationProps) {
  return (
    <NativeEnableVerification
      visible={visible}
      status={nativeStatus}
      loading={loading}
      error={error}
    />
  );
}

function NativeEnableVerification({
  visible,
  status,
  loading,
  error,
}: {
  visible: boolean;
  status?: NativeCrossSigningStatus;
  loading?: boolean;
  error?: string;
}) {
  const [open, setOpen] = useState(false);
  // Escape and clicking outside close the setup dialog, except while a step runs.
  const workingRef = useRef(false);
  if (!visible) return null;
  if (loading && !status) return <Spinner size="200" variant="Secondary" />;
  if (error || status?.readiness === 'unavailable') {
    return (
      <InlineMessage tone="critical">
        Synara could not check cross-signing. Restart Synara and try again.
      </InlineMessage>
    );
  }
  if (status?.bootstrap === 'not_needed') {
    return (
      <Text size="T200">
        Your cross-signing identity is configured, but this device needs verification or recovery.
      </Text>
    );
  }

  return (
    <>
      <Button size="300" radii="300" onClick={() => setOpen(true)}>
        <Text as="span" size="B300">
          Set up
        </Text>
      </Button>
      {open && (
        <Overlay open backdrop={<OverlayBackdrop />}>
          <OverlayCenter>
            <FocusTrap
              focusTrapOptions={{
                initialFocus: false,
                onDeactivate: () => setOpen(false),
                clickOutsideDeactivates: () => !workingRef.current,
                escapeDeactivates: (evt) => (workingRef.current ? false : stopPropagation(evt)),
              }}
            >
              <NativeCrossSigningSetup
                onCancel={() => setOpen(false)}
                onWorkingChange={(working) => {
                  workingRef.current = working;
                }}
              />
            </FocusTrap>
          </OverlayCenter>
        </Overlay>
      )}
    </>
  );
}

function NativeCrossSigningSetup({
  onCancel,
  onWorkingChange,
}: {
  onCancel: () => void;
  onWorkingChange?: (working: boolean) => void;
}) {
  const [authenticationRequired, setAuthenticationRequired] = useState(false);
  const [crossSigningComplete, setCrossSigningComplete] = useState(false);
  const [working, setWorking] = useState(false);
  const [setupError, setSetupError] = useState<string>();
  const passwordId = useId();
  useEffect(() => {
    onWorkingChange?.(working);
  }, [working, onWorkingChange]);

  const startSetup = async () => {
    setWorking(true);
    setSetupError(undefined);
    try {
      const result = await startNativeCrossSigningSetup();
      if (result.outcome === 'authentication_required') {
        setAuthenticationRequired(true);
      } else {
        setCrossSigningComplete(true);
      }
    } catch (error) {
      setSetupError(
        composerErrorMessage(error, 'Synara could not set up cross-signing. Try again.')
      );
    } finally {
      setWorking(false);
    }
  };

  const authenticate: FormEventHandler<HTMLFormElement> = async (event) => {
    event.preventDefault();
    if (working) return;
    const target = event.currentTarget;
    const passwordInput = target.elements.namedItem('password') as HTMLInputElement | null;
    const password = passwordInput?.value ?? '';
    setWorking(true);
    setSetupError(undefined);
    try {
      await authenticateNativeCrossSigningSetup(password);
      setCrossSigningComplete(true);
    } catch (error) {
      setSetupError(
        composerErrorMessage(error, 'Synara could not set up cross-signing. Try again.')
      );
    } finally {
      if (passwordInput) passwordInput.value = '';
      setWorking(false);
    }
  };

  return (
    <Dialog variant="Surface">
      <Header
        style={{
          padding: `0 ${config.space.S200} 0 ${config.space.S400}`,
          borderBottomWidth: config.borderWidth.B300,
        }}
        variant="Surface"
        size="500"
      >
        <Box grow="Yes">
          <Text as="h2" size="H4">
            Set up device verification
          </Text>
        </Box>
        <IconButton size="300" radii="300" onClick={onCancel} disabled={working} aria-label="Close">
          <Icon src={Icons.Cross} />
        </IconButton>
      </Header>
      <Box
        as={authenticationRequired && !crossSigningComplete ? 'form' : undefined}
        onSubmit={authenticationRequired && !crossSigningComplete ? authenticate : undefined}
        style={{ padding: config.space.S400 }}
        direction="Column"
        gap="400"
      >
        {crossSigningComplete ? (
          <NativeSecretStorageGate>
            {() => (
              <>
                <Text size="T300">
                  Device verification and account recovery are configured for this session.
                </Text>
                <Button type="button" onClick={onCancel}>
                  <Text size="B400">Done</Text>
                </Button>
              </>
            )}
          </NativeSecretStorageGate>
        ) : (
          <>
            <Text size="T300">
              Synara will create and securely store your cross-signing identity on this device.
            </Text>
            {authenticationRequired && (
              <Box direction="Column" gap="100">
                <Text as="label" size="L400" priority="300" htmlFor={passwordId}>
                  Account password
                </Text>
                <PasswordInput
                  id={passwordId}
                  name="password"
                  size="400"
                  readOnly={working}
                  autoFocus
                />
              </Box>
            )}
            <Button
              type={authenticationRequired ? 'submit' : 'button'}
              onClick={authenticationRequired ? undefined : startSetup}
              disabled={working}
              before={working && <Spinner size="200" variant="Primary" fill="Solid" />}
            >
              <Text size="B400">Continue</Text>
            </Button>
            {setupError && <InlineMessage tone="critical">{setupError}</InlineMessage>}
          </>
        )}
      </Box>
    </Dialog>
  );
}
