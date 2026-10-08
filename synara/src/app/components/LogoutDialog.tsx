import React, { forwardRef, useCallback } from 'react';
import { Dialog, Header, config, Box, Text, Button, Spinner, color } from 'folds';
import { AsyncStatus, useAsyncCallback } from '../hooks/useAsyncCallback';
import {
  attemptLogout,
  LOGOUT_RETRY_COPY,
  type LogoutAttemptOutcome,
} from '../../client/initMatrix';
import { useCrossSigningActive } from '../hooks/useCrossSigning';
import { InfoCard } from './info-card';
import { useDeviceList } from '../hooks/useDeviceList';
import {
  isLastSignedInDevice,
  LAST_DEVICE_LOGOUT_TITLE,
  LAST_DEVICE_LOGOUT_WARNING,
} from './logoutLastDevice';

import { getNativeRooms, nativeSession } from '../native/nativeSession';
type LogoutDialogProps = {
  handleClose: () => void;
};
export const LogoutDialog = forwardRef<HTMLDivElement, LogoutDialogProps>(
  ({ handleClose }, ref) => {
    const hasEncryptedRoom = !!getNativeRooms().find((room) => room.hasEncryptionStateEvent());
    const crossSigningActive = useCrossSigningActive();
    const [deviceSnapshot] = useDeviceList();
    const lastSignedInDevice = isLastSignedInDevice(deviceSnapshot) === true;

    const [logoutState, logout] = useAsyncCallback<LogoutAttemptOutcome, Error, []>(
      useCallback(() => attemptLogout(nativeSession()), [])
    );

    const ongoingLogout = logoutState.status === AsyncStatus.Loading;
    // attemptLogout resolves `retry` for a rejected or incomplete native
    // logout; an unexpected throw is treated the same way.
    const logoutNeedsRetry =
      (logoutState.status === AsyncStatus.Success && logoutState.data === 'retry') ||
      logoutState.status === AsyncStatus.Error;

    return (
      <Dialog variant="Surface" ref={ref}>
        <Header
          style={{
            padding: `0 ${config.space.S200} 0 ${config.space.S400}`,
            borderBottomWidth: config.borderWidth.B300,
          }}
          variant="Surface"
          size="500"
        >
          <Box grow="Yes">
            <Text size="H4">Logout</Text>
          </Box>
        </Header>
        <Box style={{ padding: config.space.S400 }} direction="Column" gap="400">
          {hasEncryptedRoom &&
            (crossSigningActive ? (
              deviceSnapshot?.ownVerification === 'unverified' && (
                <InfoCard
                  variant="Critical"
                  title="Unverified Device"
                  description="Verify your device before logging out to save your encrypted messages."
                />
              )
            ) : (
              <InfoCard
                variant="Critical"
                title="Alert"
                description="Enable device verification or export your encrypted data from settings to avoid losing access to your messages."
              />
            ))}
          {lastSignedInDevice && (
            <InfoCard
              variant="Critical"
              title={LAST_DEVICE_LOGOUT_TITLE}
              description={LAST_DEVICE_LOGOUT_WARNING}
            />
          )}
          <Text priority="400">You’re about to log out. Are you sure?</Text>
          {logoutNeedsRetry && (
            <Text style={{ color: color.Critical.Main }} size="T300">
              {LOGOUT_RETRY_COPY}
            </Text>
          )}
          <Box direction="Column" gap="200">
            <Button
              variant="Critical"
              onClick={() => {
                // The outcome lands in logoutState; `retry` renders LOGOUT_RETRY_COPY.
                void logout();
              }}
              disabled={ongoingLogout}
              before={ongoingLogout && <Spinner variant="Critical" fill="Solid" size="200" />}
            >
              <Text size="B400">Logout</Text>
            </Button>
            <Button variant="Secondary" fill="Soft" onClick={handleClose} disabled={ongoingLogout}>
              <Text size="B400">Cancel</Text>
            </Button>
          </Box>
        </Box>
      </Dialog>
    );
  }
);
