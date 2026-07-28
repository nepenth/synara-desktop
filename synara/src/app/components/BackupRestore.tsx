import React, { FormEventHandler, MouseEventHandler, useCallback, useState } from 'react';
import { useAtom } from 'jotai';
import { CryptoApi, KeyBackupInfo } from 'matrix-js-sdk/lib/crypto-api';
import {
  Badge,
  Box,
  Button,
  color,
  config,
  Icon,
  IconButton,
  Icons,
  Menu,
  percent,
  PopOut,
  ProgressBar,
  RectCords,
  Spinner,
  Text,
} from 'folds';
import FocusTrap from 'focus-trap-react';
import { BackupProgressStatus, backupRestoreProgressAtom } from '../state/backupRestore';
import { InfoCard } from './info-card';
import { AsyncStatus, useAsyncCallback } from '../hooks/useAsyncCallback';
import {
  useKeyBackupInfo,
  useKeyBackupStatus,
  useKeyBackupSync,
  useKeyBackupTrust,
  useNativeKeyBackup,
} from '../hooks/useKeyBackup';
import { stopPropagation } from '../utils/keyboard';
import { useRestoreBackupOnVerification } from '../hooks/useRestoreBackupOnVerification';
import { isNativeMatrixSession } from '../features/verification/nativeVerification';
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
type BackupSyncingProps = {
  count: number;
};
function BackupSyncing({ count }: BackupSyncingProps) {
  return (
    <Box as="span" gap="100" alignItems="Center">
      <Spinner size="50" variant="Primary" fill="Soft" />
      <Text as="span" size="L400" style={{ color: color.Primary.Main }}>
        Syncing ({count})
      </Text>
    </Box>
  );
}

function BackupProgressFetching() {
  return (
    <Box grow="Yes" gap="200" alignItems="Center">
      <Badge variant="Secondary" fill="Solid" radii="300">
        <Text size="L400">Restoring: 0%</Text>
      </Badge>
      <Box grow="Yes" direction="Column">
        <ProgressBar variant="Secondary" size="300" min={0} max={1} value={0} />
      </Box>
      <Spinner size="50" variant="Secondary" fill="Soft" />
    </Box>
  );
}

type BackupProgressProps = {
  total: number;
  downloaded: number;
};
function BackupProgress({ total, downloaded }: BackupProgressProps) {
  return (
    <Box grow="Yes" gap="200" alignItems="Center">
      <Badge variant="Secondary" fill="Solid" radii="300">
        <Text size="L400">Restoring: {`${Math.round(percent(0, total, downloaded))}%`}</Text>
      </Badge>
      <Box grow="Yes" direction="Column">
        <ProgressBar variant="Secondary" size="300" min={0} max={total} value={downloaded} />
      </Box>
      <Badge variant="Secondary" fill="Soft" radii="Pill">
        <Text size="L400">
          {downloaded} / {total}
        </Text>
      </Badge>
    </Box>
  );
}

type BackupTrustInfoProps = {
  crypto: CryptoApi;
  backupInfo: KeyBackupInfo;
};
function BackupTrustInfo({ crypto, backupInfo }: BackupTrustInfoProps) {
  const trust = useKeyBackupTrust(crypto, backupInfo);

  if (!trust) return null;

  return (
    <Box direction="Column">
      {trust.matchesDecryptionKey ? (
        <Text size="T200" style={{ color: color.Success.Main }}>
          <b>Backup has trusted decryption key.</b>
        </Text>
      ) : (
        <Text size="T200" style={{ color: color.Critical.Main }}>
          <b>Backup does not have trusted decryption key!</b>
        </Text>
      )}
      {trust.trusted ? (
        <Text size="T200" style={{ color: color.Success.Main }}>
          <b>Backup has trusted by signature.</b>
        </Text>
      ) : (
        <Text size="T200" style={{ color: color.Critical.Main }}>
          <b>Backup does not have trusted signature!</b>
        </Text>
      )}
    </Box>
  );
}

type BackupRestoreTileProps = {
  crypto: CryptoApi;
};
function LegacyBackupRestoreTile({ crypto }: BackupRestoreTileProps) {
  const [restoreProgress, setRestoreProgress] = useAtom(backupRestoreProgressAtom);
  const restoring =
    restoreProgress.status === BackupProgressStatus.Fetching ||
    restoreProgress.status === BackupProgressStatus.Loading;

  const backupEnabled = useKeyBackupStatus(crypto);
  const backupInfo = useKeyBackupInfo(crypto);
  const [remainingSession, syncFailure] = useKeyBackupSync();

  const [menuCords, setMenuCords] = useState<RectCords>();

  const handleMenu: MouseEventHandler<HTMLButtonElement> = (evt) => {
    setMenuCords(evt.currentTarget.getBoundingClientRect());
  };

  const [restoreState, restoreBackup] = useAsyncCallback<void, Error, []>(
    useCallback(async () => {
      await crypto.restoreKeyBackup({
        progressCallback(progress) {
          setRestoreProgress(progress);
        },
      });
    }, [crypto, setRestoreProgress])
  );

  const handleRestore = () => {
    setMenuCords(undefined);
    restoreBackup();
  };

  return (
    <InfoCard
      variant="Surface"
      title="Encryption Backup"
      after={
        <Box alignItems="Center" gap="200">
          {remainingSession === 0 ? (
            <BackupStatus enabled={backupEnabled} />
          ) : (
            <BackupSyncing count={remainingSession} />
          )}
          <IconButton
            aria-pressed={!!menuCords}
            size="300"
            variant="Surface"
            radii="300"
            onClick={handleMenu}
          >
            <Icon size="100" src={Icons.VerticalDots} />
          </IconButton>
          <PopOut
            anchor={menuCords}
            offset={5}
            position="Bottom"
            align="End"
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
                <Menu
                  style={{
                    padding: config.space.S100,
                  }}
                >
                  <Box direction="Column" gap="100">
                    <Box direction="Column" gap="200">
                      <InfoCard
                        variant="SurfaceVariant"
                        title="Backup Details"
                        description={
                          <>
                            <span>Version: {backupInfo?.version ?? 'NIL'}</span>
                            <br />
                            <span>Keys: {backupInfo?.count ?? 'NIL'}</span>
                          </>
                        }
                      />
                    </Box>
                    <Button
                      size="300"
                      variant="Success"
                      radii="300"
                      aria-disabled={restoreState.status === AsyncStatus.Loading || restoring}
                      onClick={
                        restoreState.status === AsyncStatus.Loading || restoring
                          ? undefined
                          : handleRestore
                      }
                      before={<Icon size="100" src={Icons.Download} />}
                    >
                      <Text size="B300">Restore Backup</Text>
                    </Button>
                  </Box>
                </Menu>
              </FocusTrap>
            }
          />
        </Box>
      }
    >
      {syncFailure && (
        <Text size="T200" style={{ color: color.Critical.Main }}>
          <b>{syncFailure}</b>
        </Text>
      )}
      {!backupEnabled && backupInfo === null && (
        <Text size="T200" style={{ color: color.Critical.Main }}>
          <b>No backup present on server!</b>
        </Text>
      )}
      {!syncFailure && !backupEnabled && backupInfo && (
        <BackupTrustInfo crypto={crypto} backupInfo={backupInfo} />
      )}
      {restoreState.status === AsyncStatus.Loading && !restoring && <BackupProgressFetching />}
      {restoreProgress.status === BackupProgressStatus.Fetching && <BackupProgressFetching />}
      {restoreProgress.status === BackupProgressStatus.Loading && (
        <BackupProgress
          total={restoreProgress.data.total}
          downloaded={restoreProgress.data.downloaded}
        />
      )}
      {restoreState.status === AsyncStatus.Error && (
        <Text size="T200" style={{ color: color.Critical.Main }}>
          <b>{restoreState.error.message}</b>
        </Text>
      )}
    </InfoCard>
  );
}

const nativeBackupActionLabel = (action: NativeBackupAction): string => {
  if (action === 'setup_required') return 'Set Up Backup';
  if (action === 'restore_required') return 'Restore Backup';
  return 'Check & Repair';
};

function NativeBackupRestoreTile() {
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

export function BackupRestoreTile({ crypto }: { crypto?: CryptoApi }) {
  if (isNativeMatrixSession()) {
    return <NativeBackupRestoreTile />;
  }
  if (!crypto) {
    return (
      <Text size="T200" style={{ color: color.Critical.Main }}>
        Encryption backup is unavailable.
      </Text>
    );
  }
  return <LegacyBackupRestoreTile crypto={crypto} />;
}

export function AutoRestoreBackupOnVerification() {
  useRestoreBackupOnVerification();

  return null;
}
