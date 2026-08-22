import React, { FormEventHandler, useState } from 'react';
import {
  Badge,
  Box,
  Button,
  color,
  config,
  Dialog,
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
import FocusTrap from 'focus-trap-react';
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
        <Text size="L400">Unverified</Text>
      </Badge>
    );
  }

  if (unverifiedCount > 0) {
    return (
      <Badge variant="Warning" fill="Solid" size="500">
        <Text size="L400">{unverifiedCount} Unverified</Text>
      </Badge>
    );
  }

  return (
    <Badge variant="Success" fill="Solid" size="500">
      <Text size="L400">Verified</Text>
    </Badge>
  );
}

export function VerifyCurrentDeviceTile({ onVerified }: { onVerified?: () => void }) {
  return (
    <InfoCard
      variant="Critical"
      title="Unverified"
      description="Use another verified device to compare emoji or decimal security codes."
      after={<NativeStartVerification onExit={() => onVerified?.()} />}
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
      title="Unverified"
      description="Verify device identity and grant access to encrypted messages."
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
  if (!visible) return null;
  if (loading && !status) return <Spinner size="200" variant="Secondary" />;
  if (error || status?.readiness === 'unavailable') {
    return (
      <Text size="T200">Native cross-signing is unavailable. Restart Synara and try again.</Text>
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
          Enable
        </Text>
      </Button>
      {open && (
        <Overlay open backdrop={<OverlayBackdrop />}>
          <OverlayCenter>
            <FocusTrap
              focusTrapOptions={{
                initialFocus: false,
                clickOutsideDeactivates: false,
                escapeDeactivates: false,
              }}
            >
              <NativeCrossSigningSetup onCancel={() => setOpen(false)} />
            </FocusTrap>
          </OverlayCenter>
        </Overlay>
      )}
    </>
  );
}

function NativeCrossSigningSetup({ onCancel }: { onCancel: () => void }) {
  const [authenticationRequired, setAuthenticationRequired] = useState(false);
  const [crossSigningComplete, setCrossSigningComplete] = useState(false);
  const [working, setWorking] = useState(false);
  const [setupError, setSetupError] = useState<string>();

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
      setSetupError(error instanceof Error ? error.message : 'Cross-signing setup failed.');
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
      setSetupError(error instanceof Error ? error.message : 'Cross-signing setup failed.');
    } finally {
      if (passwordInput) passwordInput.value = '';
      setWorking(false);
    }
  };

  return (
    <Dialog>
      <Header
        style={{
          padding: `0 ${config.space.S200} 0 ${config.space.S400}`,
          borderBottomWidth: config.borderWidth.B300,
        }}
        variant="Surface"
        size="500"
      >
        <Box grow="Yes">
          <Text size="H4">Setup Device Verification</Text>
        </Box>
        <IconButton size="300" radii="300" onClick={onCancel} disabled={working}>
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
                <Text size="L400">Account Password</Text>
                <PasswordInput name="password" size="400" readOnly={working} autoFocus />
              </Box>
            )}
            <Button
              type={authenticationRequired ? 'submit' : 'button'}
              onClick={authenticationRequired ? undefined : startSetup}
              disabled={working}
              before={working && <Spinner size="200" variant="Primary" fill="Solid" />}
            >
              <Text size="B400">{authenticationRequired ? 'Authenticate' : 'Continue'}</Text>
            </Button>
            {setupError && (
              <Text size="T200" style={{ color: color.Critical.Main }}>
                {setupError}
              </Text>
            )}
          </>
        )}
      </Box>
    </Dialog>
  );
}
