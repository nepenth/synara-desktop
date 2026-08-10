import React, { FormEventHandler, useCallback } from 'react';
import { Badge, Box, Button, color, Spinner, Text } from 'folds';
import { InfoCard } from './info-card';
import { AsyncStatus, useAsyncCallback } from '../hooks/useAsyncCallback';
import { useNativeKeyBackup } from '../hooks/useNativeKeyBackup';
import {
  NativeBackupAction,
  repairNativeBackup,
  restoreNativeBackup,
  setupNativeBackup,
} from '../features/backup/nativeBackup';
import { PasswordInput } from './password-input';

type BackupStatusProps = {
  enabled: boolean;
};
function BackupStatus({ enabled }: BackupStatusProps) {
  return (
    <Box as="span" gap="100" alignItems="Center">
      <Badge variant={enabled ? 'Success' : 'Critical'} fill="Solid" size="200" radii="Pill" />
      <Text
        as="span"
        size="L400"
        style={{ color: enabled ? color.Success.Main : color.Critical.Main }}
      >
        {enabled ? 'Connected' : 'Disconnected'}
      </Text>
    </Box>
  );
}

const nativeBackupActionLabel = (action: NativeBackupAction): string => {
  if (action === 'setup_required') return 'Set Up Backup';
  if (action === 'restore_required') return 'Restore Backup';
  return 'Check & Repair';
};

export function BackupRestoreTile() {
  const { status, loading, error, refresh } = useNativeKeyBackup();
  const action = status?.action ?? 'restore_required';
  const [operationState, runOperation] = useAsyncCallback<void, Error, [string]>(
    useCallback(
      async (secret) => {
        if (action === 'setup_required') {
          await setupNativeBackup(secret);
        } else if (action === 'restore_required') {
          await restoreNativeBackup(secret);
        } else {
          await repairNativeBackup(secret);
        }
        refresh();
      },
      [action, refresh]
    )
  );
  const working = operationState.status === AsyncStatus.Loading;

  const handleSubmit: FormEventHandler<HTMLFormElement> = (event) => {
    event.preventDefault();
    if (working) return;
    const form = event.currentTarget;
    const input = form.recoveryInput as HTMLInputElement;
    if (!input.value) return;
    const confirmInput = form.confirmRecoveryInput as HTMLInputElement | undefined;
    if (confirmInput && input.value !== confirmInput.value) {
      confirmInput.setCustomValidity('Recovery passphrases do not match.');
      confirmInput.reportValidity();
      return;
    }
    confirmInput?.setCustomValidity('');
    runOperation(input.value)
      .then(() => {
        input.value = '';
        if (confirmInput) confirmInput.value = '';
      })
      .catch(() => undefined);
  };

  return (
    <InfoCard
      variant="Surface"
      title="Encryption Backup"
      description={
        status?.enabled
          ? 'This device is connected to your server-side encryption backup.'
          : 'Connect this device to restore encrypted message history.'
      }
      after={
        loading ? (
          <Spinner size="100" variant="Secondary" fill="Soft" />
        ) : (
          <BackupStatus enabled={status?.enabled ?? false} />
        )
      }
    >
      {status && (
        <Text size="T200">
          Version: {status.version ?? 'None'} · Keys: {status.keyCount ?? 0} · Device:{' '}
          {status.deviceState.replaceAll('_', ' ')}
        </Text>
      )}
      <Box as="form" onSubmit={handleSubmit} direction="Column" gap="100">
        <Text size="L400">
          {action === 'setup_required' ? 'New Recovery Passphrase' : 'Recovery Key or Passphrase'}
        </Text>
        <Box gap="200" alignItems="End">
          <Box grow="Yes">
            <PasswordInput
              name="recoveryInput"
              size="400"
              variant="Secondary"
              radii="300"
              required
              readOnly={working}
            />
          </Box>
          {action === 'setup_required' && (
            <Box grow="Yes">
              <PasswordInput
                name="confirmRecoveryInput"
                aria-label="Confirm recovery passphrase"
                size="400"
                variant="Secondary"
                radii="300"
                required
                readOnly={working}
                onChange={(event) => event.currentTarget.setCustomValidity('')}
              />
            </Box>
          )}
          <Button
            type="submit"
            size="400"
            variant={action === 'repair_required' ? 'Warning' : 'Success'}
            radii="300"
            disabled={working}
            before={working ? <Spinner size="100" variant="Secondary" fill="Soft" /> : undefined}
          >
            <Text size="B300">{nativeBackupActionLabel(action)}</Text>
          </Button>
        </Box>
      </Box>
      {(error || operationState.status === AsyncStatus.Error) && (
        <Text size="T200" style={{ color: color.Critical.Main }}>
          <b>
            {operationState.status === AsyncStatus.Error ? operationState.error.message : error}
          </b>
        </Text>
      )}
    </InfoCard>
  );
}
