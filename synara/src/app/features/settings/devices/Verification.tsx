import React, { FormEventHandler, MouseEventHandler, useCallback, useState } from 'react';
import {
  Badge,
  Box,
  Button,
  Chip,
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
  RectCords,
  PopOut,
  Menu,
  MenuItem,
} from 'folds';
import FocusTrap from 'focus-trap-react';
import { CryptoApi, VerificationRequest } from 'matrix-js-sdk/lib/crypto-api';
import { VerificationStatus } from '../../../hooks/useDeviceVerificationStatus';
import { InfoCard } from '../../../components/info-card';
import { ManualVerificationTile } from '../../../components/ManualVerification';
import { SecretStorageKeyContent } from '../../../../types/matrix/accountData';
import { AsyncState, AsyncStatus, useAsync } from '../../../hooks/useAsyncCallback';
import { useMatrixClient } from '../../../hooks/useMatrixClient';
import { DeviceVerification } from '../../../components/DeviceVerification';
import {
  DeviceVerificationReset,
  DeviceVerificationSetup,
} from '../../../components/DeviceVerificationSetup';
import { stopPropagation } from '../../../utils/keyboard';
import { useAuthMetadata } from '../../../hooks/useAuthMetadata';
import { withSearchParam } from '../../../pages/pathUtils';
import { useAccountManagementActions } from '../../../hooks/useAccountManagement';
import { openExternalUrl } from '../../../utils/appLinks';
import { NativeStartVerification } from '../../verification/NativeDeviceVerification';
import { isNativeMatrixSession } from '../../verification/nativeVerification';
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
  if (
    verificationStatus === VerificationStatus.Unknown ||
    typeof otherUnverifiedCount !== 'number'
  ) {
    return <Spinner size="400" variant="Secondary" />;
  }
  if (verificationStatus === VerificationStatus.Unverified) {
    return (
      <Badge variant="Critical" fill="Solid" size="500">
        <Text size="L400">Unverified</Text>
      </Badge>
    );
  }

  if (otherUnverifiedCount > 0) {
    return (
      <Badge variant="Warning" fill="Solid" size="500">
        <Text size="L400">{otherUnverifiedCount} Unverified</Text>
      </Badge>
    );
  }

  return (
    <Badge variant="Success" fill="Solid" size="500">
      <Text size="L400">Verified</Text>
    </Badge>
  );
}

function LearnStartVerificationFromOtherDevice({
  manualVerificationAvailable,
}: {
  manualVerificationAvailable: boolean;
}) {
  return (
    <Box direction="Column">
      <Text size="T200">Steps to verify from another device.</Text>
      <Text as="div" size="T200">
        <ul style={{ margin: `${config.space.S100} 0` }}>
          <li>Press Verify from Another Device.</li>
          <li>Open your other verified device.</li>
          <li>Accept the verification request.</li>
          <li>Compare the emoji shown on both devices.</li>
        </ul>
      </Text>
      {manualVerificationAvailable && (
        <Text size="T200">
          If you do not have any verified device press the <i>&quot;Verify Manually&quot;</i>{' '}
          button.
        </Text>
      )}
    </Box>
  );
}

type VerifyCurrentDeviceTileProps = {
  crypto?: CryptoApi;
  secretStorageKeyId?: string;
  secretStorageKeyContent?: SecretStorageKeyContent;
};
export function VerifyCurrentDeviceTile({
  crypto,
  secretStorageKeyId,
  secretStorageKeyContent,
}: VerifyCurrentDeviceTileProps) {
  if (isNativeMatrixSession()) {
    return (
      <InfoCard
        variant="Critical"
        title="Unverified"
        description="Use another verified device to compare emoji or decimal security codes."
        after={<NativeStartVerification onExit={() => undefined} />}
      />
    );
  }
  if (!crypto) {
    return (
      <InfoCard
        variant="Critical"
        title="Verification unavailable"
        description="Device verification is unavailable for this session."
      />
    );
  }
  return (
    <LegacyVerifyCurrentDeviceTile
      crypto={crypto}
      secretStorageKeyId={secretStorageKeyId}
      secretStorageKeyContent={secretStorageKeyContent}
    />
  );
}

function LegacyVerifyCurrentDeviceTile({
  crypto,
  secretStorageKeyId,
  secretStorageKeyContent,
}: {
  crypto: CryptoApi;
  secretStorageKeyId?: string;
  secretStorageKeyContent?: SecretStorageKeyContent;
}) {
  const [learnMore, setLearnMore] = useState(false);

  const [manualVerification, setManualVerification] = useState(false);
  const handleCancelVerification = () => setManualVerification(false);
  const manualVerificationAvailable = Boolean(secretStorageKeyId && secretStorageKeyContent);

  const [requestState, setRequestState] = useState<AsyncState<VerificationRequest, Error>>({
    status: AsyncStatus.Idle,
  });

  const requestVerification = useAsync<VerificationRequest, Error, []>(
    useCallback(() => crypto.requestOwnUserVerification(), [crypto]),
    setRequestState
  );

  const handleExit = useCallback(() => {
    setRequestState({
      status: AsyncStatus.Idle,
    });
  }, []);

  const requesting = requestState.status === AsyncStatus.Loading;
  const verificationOpen = requestState.status === AsyncStatus.Success;

  return (
    <>
      <InfoCard
        variant="Critical"
        title="Unverified"
        description={
          <>
            Use another verified device to compare emoji
            {manualVerificationAvailable ? ' or verify manually with your recovery key' : ''}.{' '}
            <Text as="a" size="T200" onClick={() => setLearnMore(!learnMore)}>
              <b>{learnMore ? 'View Less' : 'Learn More'}</b>
            </Text>
          </>
        }
        after={
          !manualVerification &&
          !verificationOpen && (
            <Box gap="200" wrap="Wrap" justifyContent="End">
              <Button
                size="300"
                variant="Critical"
                radii="300"
                onClick={requestVerification}
                before={requesting && <Spinner size="100" variant="Critical" fill="Solid" />}
                disabled={requesting}
              >
                <Text as="span" size="B300">
                  Verify from Another Device
                </Text>
              </Button>
              {manualVerificationAvailable && (
                <Button
                  size="300"
                  variant="Critical"
                  fill="Soft"
                  radii="300"
                  outlined
                  onClick={() => setManualVerification(true)}
                >
                  <Text as="span" size="B300">
                    Verify Manually
                  </Text>
                </Button>
              )}
            </Box>
          )
        }
      >
        {learnMore && (
          <LearnStartVerificationFromOtherDevice
            manualVerificationAvailable={manualVerificationAvailable}
          />
        )}
        {requestState.status === AsyncStatus.Error && (
          <Text size="T200">{requestState.error.message}</Text>
        )}
        {requestState.status === AsyncStatus.Success && (
          <DeviceVerification request={requestState.data} onExit={handleExit} />
        )}
      </InfoCard>
      {manualVerification && secretStorageKeyId && secretStorageKeyContent && (
        <ManualVerificationTile
          secretStorageKeyId={secretStorageKeyId}
          secretStorageKeyContent={secretStorageKeyContent}
          options={
            <Chip
              type="button"
              variant="Secondary"
              fill="Soft"
              radii="Pill"
              onClick={handleCancelVerification}
            >
              <Icon size="100" src={Icons.Cross} />
            </Chip>
          }
        />
      )}
    </>
  );
}

type VerifyOtherDeviceTileProps = {
  crypto?: CryptoApi;
  deviceId: string;
};
export function VerifyOtherDeviceTile({ crypto, deviceId }: VerifyOtherDeviceTileProps) {
  if (isNativeMatrixSession()) {
    return (
      <InfoCard
        variant="Warning"
        title="Unverified"
        description="Verify device identity and grant access to encrypted messages."
        after={<NativeStartVerification deviceId={deviceId} onExit={() => undefined} />}
      />
    );
  }
  if (!crypto) {
    return (
      <InfoCard
        variant="Warning"
        title="Verification unavailable"
        description="Device verification is unavailable for this session."
      />
    );
  }
  return <LegacyVerifyOtherDeviceTile crypto={crypto} deviceId={deviceId} />;
}

function LegacyVerifyOtherDeviceTile({
  crypto,
  deviceId,
}: {
  crypto: CryptoApi;
  deviceId: string;
}) {
  const mx = useMatrixClient();
  const [requestState, setRequestState] = useState<AsyncState<VerificationRequest, Error>>({
    status: AsyncStatus.Idle,
  });

  const requestVerification = useAsync<VerificationRequest, Error, []>(
    useCallback(() => {
      const requestPromise = crypto.requestDeviceVerification(mx.getSafeUserId(), deviceId);
      return requestPromise;
    }, [mx, crypto, deviceId]),
    setRequestState
  );

  const handleExit = useCallback(() => {
    setRequestState({
      status: AsyncStatus.Idle,
    });
  }, []);

  const requesting = requestState.status === AsyncStatus.Loading;
  return (
    <InfoCard
      variant="Warning"
      title="Unverified"
      description="Verify device identity and grant access to encrypted messages."
      after={
        <Button
          size="300"
          variant="Warning"
          radii="300"
          onClick={requestVerification}
          before={requesting && <Spinner size="100" variant="Warning" fill="Solid" />}
          disabled={requesting}
        >
          <Text as="span" size="B300">
            Verify
          </Text>
        </Button>
      }
    >
      {requestState.status === AsyncStatus.Error && (
        <Text size="T200">{requestState.error.message}</Text>
      )}
      {requestState.status === AsyncStatus.Success && (
        <DeviceVerification request={requestState.data} onExit={handleExit} />
      )}
    </InfoCard>
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
  if (isNativeMatrixSession()) {
    return (
      <NativeEnableVerification
        visible={visible}
        status={nativeStatus}
        loading={loading}
        error={error}
      />
    );
  }
  return <LegacyEnableVerification visible={visible} />;
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
  if (loading) return <Spinner size="200" variant="Secondary" />;
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

function LegacyEnableVerification({ visible }: EnableVerificationProps) {
  const [open, setOpen] = useState(false);

  const handleCancel = useCallback(() => setOpen(false), []);

  return (
    <>
      {visible && (
        <Button size="300" radii="300" onClick={() => setOpen(true)}>
          <Text as="span" size="B300">
            Enable
          </Text>
        </Button>
      )}
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
              <DeviceVerificationSetup onCancel={handleCancel} />
            </FocusTrap>
          </OverlayCenter>
        </Overlay>
      )}
    </>
  );
}

export function DeviceVerificationOptions() {
  const [menuCords, setMenuCords] = useState<RectCords>();
  const authMetadata = useAuthMetadata();
  const accountManagementActions = useAccountManagementActions();

  const [reset, setReset] = useState(false);

  const handleCancelReset = useCallback(() => {
    setReset(false);
  }, []);

  const handleMenu: MouseEventHandler<HTMLButtonElement> = (event) => {
    setMenuCords(event.currentTarget.getBoundingClientRect());
  };

  const handleReset = () => {
    setMenuCords(undefined);

    if (authMetadata) {
      const authUrl = authMetadata.account_management_uri ?? authMetadata.issuer;
      void openExternalUrl(
        withSearchParam(authUrl, {
          action: accountManagementActions.crossSigningReset,
        })
      );
      return;
    }

    setReset(true);
  };

  return (
    <>
      <IconButton
        aria-pressed={!!menuCords}
        variant="SurfaceVariant"
        size="300"
        radii="300"
        onClick={handleMenu}
      >
        <Icon size="100" src={Icons.VerticalDots} />
      </IconButton>
      <PopOut
        anchor={menuCords}
        offset={5}
        position="Bottom"
        align="Center"
        content={
          <FocusTrap
            focusTrapOptions={{
              initialFocus: false,
              onDeactivate: () => setMenuCords(undefined),
              clickOutsideDeactivates: true,
              isKeyForward: (evt: KeyboardEvent) =>
                evt.key === 'ArrowDown' || evt.key === 'ArrowRight',
              isKeyBackward: (evt: KeyboardEvent) =>
                evt.key === 'ArrowUp' || evt.key === 'ArrowLeft',
              escapeDeactivates: stopPropagation,
            }}
          >
            <Menu>
              <Box direction="Column" gap="100" style={{ padding: config.space.S100 }}>
                <MenuItem
                  variant="Critical"
                  onClick={handleReset}
                  size="300"
                  radii="300"
                  fill="None"
                >
                  <Text as="span" size="T300" truncate>
                    Reset
                  </Text>
                </MenuItem>
              </Box>
            </Menu>
          </FocusTrap>
        }
      />
      {reset && (
        <Overlay open backdrop={<OverlayBackdrop />}>
          <OverlayCenter>
            <FocusTrap
              focusTrapOptions={{
                initialFocus: false,
                clickOutsideDeactivates: false,
                escapeDeactivates: false,
              }}
            >
              <DeviceVerificationReset onCancel={handleCancelReset} />
            </FocusTrap>
          </OverlayCenter>
        </Overlay>
      )}
    </>
  );
}
