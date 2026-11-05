import React, { useCallback } from 'react';
import { ConfirmDialog } from './confirm-dialog';
import { InlineMessage } from './inline-message';
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
  onClose: () => void;
};

/** "Log out of Synara?" with the key-safety advisories, on the shared confirm pattern. */
export function LogoutDialog({ onClose }: LogoutDialogProps) {
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
    <ConfirmDialog
      title="Log out of Synara?"
      description="This device stops receiving messages until you log in again."
      confirmLabel={logoutNeedsRetry ? 'Retry log out' : 'Log out'}
      busyLabel="Logging out…"
      tone="critical"
      busy={ongoingLogout}
      onConfirm={() => {
        // The outcome lands in logoutState; `retry` renders LOGOUT_RETRY_COPY.
        void logout();
      }}
      onCancel={onClose}
    >
      {hasEncryptedRoom &&
        (crossSigningActive ? (
          deviceSnapshot?.ownVerification === 'unverified' && (
            <InfoCard
              variant="Warning"
              title="Verify this device before logging out"
              description="An unverified device cannot restore your encrypted messages later."
            />
          )
        ) : (
          <InfoCard
            variant="Warning"
            title="Back up your keys before logging out"
            description="Set up device verification or export your encrypted data from Settings so you keep access to your messages."
          />
        ))}
      {lastSignedInDevice && (
        <InfoCard
          variant="Warning"
          title={LAST_DEVICE_LOGOUT_TITLE}
          description={LAST_DEVICE_LOGOUT_WARNING}
        />
      )}
      {logoutNeedsRetry && <InlineMessage tone="critical">{LOGOUT_RETRY_COPY}</InlineMessage>}
    </ConfirmDialog>
  );
}
