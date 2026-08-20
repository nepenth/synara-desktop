import React, { FormEventHandler, useCallback, useEffect, useRef, useState } from 'react';
import { Box, Button, config, Menu, Spinner, Text } from 'folds';
import { SequenceCard } from '../../../components/sequence-card';
import { SequenceCardStyle } from '../styles.css';
import { DeviceDeleteBtn, DeviceTile } from './DeviceTile';
import { VerifyOtherDeviceTile } from './Verification';
import { useAuthMetadata } from '../../../hooks/useAuthMetadata';
import { withSearchParam } from '../../../pages/pathUtils';
import { useAccountManagementActions } from '../../../hooks/useAccountManagement';
import { SettingTile } from '../../../components/setting-tile';
import { openExternalUrl } from '../../../utils/appLinks';
import { PasswordInput } from '../../../components/password-input';
import {
  authenticateNativeDeviceDeletePassword,
  cancelNativeDeviceDelete,
  NativeDevice,
  NativeDeviceDeleteChallenge,
  NativeDeviceDeleteResult,
  startNativeDeviceDelete,
} from './nativeDevices';
import { RefreshDeviceList } from '../../../hooks/useDeviceList';

type OtherDevicesProps = {
  devices: NativeDevice[];
  refreshDeviceList: RefreshDeviceList;
  showVerification?: boolean;
};

export function OtherDevices({ devices, refreshDeviceList, showVerification }: OtherDevicesProps) {
  const authMetadata = useAuthMetadata();
  const accountManagementActions = useAccountManagementActions();
  const [deleted, setDeleted] = useState<Set<string>>(new Set());
  const [challenge, setChallenge] = useState<NativeDeviceDeleteChallenge>();
  const [working, setWorking] = useState(false);
  const [deleteFailed, setDeleteFailed] = useState(false);
  const pendingOperationRef = useRef<
    { operationId: number; sessionGeneration: number } | undefined
  >(undefined);
  const authentication = challenge?.authentication;

  const handleDashboardOIDC = useCallback(() => {
    const authUrl = authMetadata?.account_management_uri ?? authMetadata?.issuer;
    if (!authUrl) return;
    void openExternalUrl(
      withSearchParam(authUrl, { action: accountManagementActions.sessionsList })
    );
  }, [authMetadata, accountManagementActions]);

  const handleDeleteOIDC = useCallback(
    (deviceId: string) => {
      const authUrl = authMetadata?.account_management_uri ?? authMetadata?.issuer;
      if (!authUrl) return;
      void openExternalUrl(
        withSearchParam(authUrl, {
          action: accountManagementActions.sessionEnd,
          device_id: deviceId,
        })
      );
    },
    [authMetadata, accountManagementActions]
  );

  const handleToggleDelete = useCallback((deviceId: string) => {
    setDeleted((deviceIds) => {
      const next = new Set(deviceIds);
      if (next.has(deviceId)) next.delete(deviceId);
      else next.add(deviceId);
      return next;
    });
  }, []);

  const applyDeleteResult = useCallback(
    async (result: NativeDeviceDeleteResult) => {
      if (result.outcome === 'authentication_required') {
        pendingOperationRef.current = {
          operationId: result.challenge.operationId,
          sessionGeneration: result.challenge.sessionGeneration,
        };
        setChallenge(result.challenge);
        setDeleteFailed(false);
        return;
      }
      pendingOperationRef.current = undefined;
      setChallenge(undefined);
      setDeleted(new Set());
      setDeleteFailed(false);
      await refreshDeviceList(result.snapshot);
    },
    [refreshDeviceList]
  );

  const startDelete = useCallback(async () => {
    if (working || deleted.size === 0) return;
    setWorking(true);
    setDeleteFailed(false);
    try {
      await applyDeleteResult(await startNativeDeviceDelete(Array.from(deleted)));
    } catch {
      setDeleteFailed(true);
    } finally {
      setWorking(false);
    }
  }, [applyDeleteResult, deleted, working]);

  const submitPassword: FormEventHandler<HTMLFormElement> = async (event) => {
    event.preventDefault();
    if (!challenge || working) return;
    const passwordInput = event.currentTarget.elements.namedItem(
      'password'
    ) as HTMLInputElement | null;
    const password = passwordInput?.value ?? '';
    setWorking(true);
    setDeleteFailed(false);
    try {
      await applyDeleteResult(
        await authenticateNativeDeviceDeletePassword(
          challenge.operationId,
          challenge.sessionGeneration,
          password
        )
      );
    } catch {
      setDeleteFailed(true);
    } finally {
      if (passwordInput) passwordInput.value = '';
      setWorking(false);
    }
  };

  const cancelDelete = useCallback(async () => {
    const pendingOperation = challenge
      ? {
          operationId: challenge.operationId,
          sessionGeneration: challenge.sessionGeneration,
        }
      : undefined;
    pendingOperationRef.current = undefined;
    setChallenge(undefined);
    setDeleted(new Set());
    setDeleteFailed(false);
    if (pendingOperation) {
      await cancelNativeDeviceDelete(
        pendingOperation.operationId,
        pendingOperation.sessionGeneration
      ).catch(() => undefined);
    }
  }, [challenge]);

  useEffect(
    () => () => {
      const pendingOperation = pendingOperationRef.current;
      pendingOperationRef.current = undefined;
      if (pendingOperation) {
        void cancelNativeDeviceDelete(
          pendingOperation.operationId,
          pendingOperation.sessionGeneration
        ).catch(() => undefined);
      }
    },
    []
  );

  return devices.length > 0 ? (
    <>
      <Box direction="Column" gap="100">
        <Text size="L400">Others</Text>
        {authMetadata && (
          <SequenceCard
            className={SequenceCardStyle}
            variant="SurfaceVariant"
            direction="Column"
            gap="400"
          >
            <SettingTile
              title="Device Dashboard"
              description="Manage your devices on OIDC dashboard."
              after={
                <Button
                  size="300"
                  variant="Secondary"
                  fill="Soft"
                  radii="300"
                  outlined
                  onClick={handleDashboardOIDC}
                >
                  <Text size="B300">Open</Text>
                </Button>
              }
            />
          </SequenceCard>
        )}
        {devices.map((device) => (
          <SequenceCard
            key={device.deviceId}
            className={SequenceCardStyle}
            variant={deleted.has(device.deviceId) ? 'Critical' : 'SurfaceVariant'}
            direction="Column"
            gap="400"
          >
            <DeviceTile
              device={device}
              deleted={deleted.has(device.deviceId)}
              refreshDeviceList={refreshDeviceList}
              disabled={working || challenge !== undefined}
              options={
                authMetadata ? (
                  <DeviceDeleteBtn
                    deviceId={device.deviceId}
                    deleted={false}
                    onDeleteToggle={handleDeleteOIDC}
                  />
                ) : (
                  <DeviceDeleteBtn
                    deviceId={device.deviceId}
                    deleted={deleted.has(device.deviceId)}
                    onDeleteToggle={handleToggleDelete}
                    disabled={working || challenge !== undefined}
                  />
                )
              }
            />
            {showVerification && device.trust === 'unverified' && (
              <VerifyOtherDeviceTile
                deviceId={device.deviceId}
                onVerified={() => void refreshDeviceList()}
              />
            )}
          </SequenceCard>
        ))}
      </Box>
      {deleted.size > 0 && !authMetadata && (
        <Menu
          style={{
            position: 'sticky',
            padding: config.space.S200,
            paddingLeft: config.space.S400,
            bottom: config.space.S400,
            left: config.space.S400,
            right: 0,
            zIndex: 1,
          }}
          variant="Critical"
        >
          <Box
            as={authentication === 'password' ? 'form' : undefined}
            onSubmit={authentication === 'password' ? submitPassword : undefined}
            alignItems="Center"
            gap="400"
          >
            <Box grow="Yes" direction="Column" gap="200">
              <Text size="T200">
                <b>Logout from selected devices. ({deleted.size} selected)</b>
              </Text>
              {deleteFailed && <Text size="T200">Failed to logout devices. Please try again.</Text>}
              {challenge?.authenticationFailed && (
                <Text size="T200">
                  Authentication was not completed. Check your password and try again.
                </Text>
              )}
              {authentication === 'password' && (
                <PasswordInput
                  name="password"
                  size="400"
                  outlined
                  required
                  autoFocus
                  readOnly={working}
                />
              )}
            </Box>
            <Box shrink="No" gap="200">
              <Button
                type="button"
                size="300"
                variant="Critical"
                fill="None"
                radii="300"
                disabled={working}
                onClick={() => void cancelDelete()}
              >
                <Text size="B300">Cancel</Text>
              </Button>
              {authentication === 'password' ? (
                <Button
                  type="submit"
                  size="300"
                  variant="Critical"
                  radii="300"
                  disabled={working}
                  before={working && <Spinner variant="Critical" fill="Solid" size="100" />}
                >
                  <Text size="B300">Authenticate</Text>
                </Button>
              ) : (
                challenge === undefined && (
                  <Button
                    type="button"
                    size="300"
                    variant="Critical"
                    radii="300"
                    disabled={working}
                    before={working && <Spinner variant="Critical" fill="Solid" size="100" />}
                    onClick={() => void startDelete()}
                  >
                    <Text size="B300">Logout</Text>
                  </Button>
                )
              )}
            </Box>
          </Box>
        </Menu>
      )}
    </>
  ) : null;
}
