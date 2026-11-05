import React, { useEffect, useState } from 'react';
import { Box, Button, Chip, Text } from 'folds';
import { useTranslation } from 'react-i18next';
import { SequenceCard } from '../../../components/sequence-card';
import { SettingTile } from '../../../components/setting-tile';
import { InlineMessage } from '../../../components/inline-message';
import { SettingsSection } from '../../../components/settings-layout';
import { SequenceCardStyle, SettingsQuietControl } from '../styles.css';
import {
  getPlatformIntegrationStatus,
  getPlatformSecretStoreSessionPersistence,
  getPlatformSecretStoreStatusDescription,
  getPlatformSecretStoreStatusLabel,
  isDesktopPlatform,
  platformSessionStore,
  type PlatformIntegrationStatus,
  type PlatformSecretStoreStatus,
} from '../../../platform';
import {
  getNativeStoreErrorWarningMessage,
  getSessionBootstrapResult,
  shouldSurfaceNativeStoreErrorWarning,
} from '../../../state/sessionBootstrap';

const LOADING_CHECK = {
  ready: false,
  supported: false,
  message: 'Loading integration state.',
};

const DEFAULT_INTEGRATION_STATUS: PlatformIntegrationStatus = {
  platform: 'web',
  desktopEnvironment: 'unknown',
  sessionType: 'unknown',
  distroId: 'unknown',
  distroName: 'unknown',
  distroVersion: 'unknown',
  buildIdentity: 'unknown',
  tray: { name: 'Tray', ...LOADING_CHECK },
  notifications: { name: 'Notifications', ...LOADING_CHECK },
  globalShortcuts: { name: 'Global Shortcuts', ...LOADING_CHECK },
  filePortal: { name: 'File Portal', ...LOADING_CHECK },
  mediaPortal: { name: 'Media Portal', ...LOADING_CHECK },
};

function formatSessionLabel(status: PlatformIntegrationStatus): string {
  const desktopEnvironment = status.desktopEnvironment.toLowerCase();
  const sessionType = status.sessionType.toLowerCase();

  if (desktopEnvironment.includes('kde') && sessionType.includes('wayland')) {
    return 'KDE Plasma Wayland';
  }
  if (desktopEnvironment.includes('kde') && sessionType.includes('x11')) {
    return 'KDE X11';
  }
  if (sessionType.includes('wayland')) return 'Wayland';
  if (sessionType.includes('x11')) return 'X11';
  if (status.sessionType && status.sessionType !== 'unknown') return status.sessionType;
  return status.desktopEnvironment && status.desktopEnvironment !== 'unknown'
    ? status.desktopEnvironment
    : 'Other';
}

function formatDistroLabel(status: PlatformIntegrationStatus): string {
  const versionLabel =
    status.distroVersion && status.distroVersion !== 'unknown' ? ` ${status.distroVersion}` : '';
  const idLabel = status.distroId && status.distroId !== 'unknown' ? ` (${status.distroId})` : '';
  const distroName =
    status.distroName && status.distroName !== 'unknown' ? status.distroName : 'Unknown';
  return `${distroName}${idLabel}${versionLabel}`;
}

function formatCheckLabel(ready: boolean, supported: boolean): string {
  if (!supported) return 'Unavailable';
  return ready ? 'Ready' : 'Not ready';
}

function buildDiagnosticsPayload(status: PlatformIntegrationStatus): string {
  return JSON.stringify(
    {
      platform: status.platform,
      desktopEnvironment: status.desktopEnvironment,
      sessionType: status.sessionType,
      distro: {
        id: status.distroId,
        name: status.distroName,
        version: status.distroVersion,
      },
      buildIdentity: status.buildIdentity,
      checks: {
        tray: status.tray,
        notifications: status.notifications,
        globalShortcuts: status.globalShortcuts,
        filePortal: status.filePortal,
        mediaPortal: status.mediaPortal,
      },
    },
    undefined,
    2
  );
}

/** Where this device keeps the session credentials (keychain, keyring, file). */
function SecretStoreTile() {
  const { t } = useTranslation();
  const [status, setStatus] = useState<PlatformSecretStoreStatus>();
  const [nativeStoreError, setNativeStoreError] = useState(
    () => getSessionBootstrapResult().nativeStoreError
  );

  useEffect(() => {
    let active = true;

    platformSessionStore.getStatus().then((nextStatus) => {
      if (!active) return;
      setStatus(nextStatus);
      setNativeStoreError(getSessionBootstrapResult().nativeStoreError);
    });

    return () => {
      active = false;
    };
  }, []);

  const persistence = status ? getPlatformSecretStoreSessionPersistence(status) : undefined;
  const badgeVariant =
    persistence === 'persistent'
      ? 'Success'
      : persistence === 'session-scoped'
        ? 'Warning'
        : status
          ? 'Critical'
          : 'Secondary';
  const statusLabel = status ? getPlatformSecretStoreStatusLabel(status) : 'Checking';
  const details = status ? getPlatformSecretStoreStatusDescription(status) : statusLabel;
  const showNativeStoreErrorWarning = shouldSurfaceNativeStoreErrorWarning(nativeStoreError, true);
  const nativeStoreErrorWarning = showNativeStoreErrorWarning
    ? t(
        'modernization.settings.secret_store.native_store_error',
        getNativeStoreErrorWarningMessage()
      )
    : undefined;

  return (
    <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
      <SettingTile
        title={t('modernization.settings.secret_store.status_title', 'Native session store')}
        description={details}
        after={
          <Chip variant={badgeVariant} radii="Pill" outlined>
            <Text size="B300">{statusLabel}</Text>
          </Chip>
        }
      >
        {nativeStoreErrorWarning && (
          <InlineMessage tone="critical">{nativeStoreErrorWarning}</InlineMessage>
        )}
      </SettingTile>
    </SequenceCard>
  );
}

const CHECKS = ['tray', 'notifications', 'globalShortcuts', 'filePortal', 'mediaPortal'] as const;

const CHECK_TITLES: Record<(typeof CHECKS)[number], string> = {
  tray: 'Tray',
  notifications: 'Notifications',
  globalShortcuts: 'Global shortcuts',
  filePortal: 'File portal',
  mediaPortal: 'Media portal',
};

/**
 * Desktop-only readiness of the tray, shortcuts and portals, plus where the
 * session is stored. Lives on Diagnostics: it describes this session rather
 * than offering a preference.
 */
export function DesktopIntegrationSection() {
  const [status, setStatus] = useState<PlatformIntegrationStatus>(DEFAULT_INTEGRATION_STATUS);
  const [copyOutcome, setCopyOutcome] = useState<'copied' | 'failed'>();

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      const nextStatus = await getPlatformIntegrationStatus();
      if (!cancelled) setStatus(nextStatus);
    };
    load();
    return () => {
      cancelled = true;
    };
  }, []);

  if (!isDesktopPlatform()) return null;

  const copyDiagnostics = async () => {
    try {
      await navigator.clipboard.writeText(buildDiagnosticsPayload(status));
      setCopyOutcome('copied');
    } catch {
      setCopyOutcome('failed');
    }
  };

  return (
    <SettingsSection
      title="Desktop integration"
      description={`${formatSessionLabel(status)} · ${formatDistroLabel(status)}`}
      after={
        <Button
          className={SettingsQuietControl}
          size="300"
          variant="Secondary"
          fill="None"
          radii="300"
          onClick={copyDiagnostics}
        >
          <Text size="B300">Copy diagnostics</Text>
        </Button>
      }
    >
      <SecretStoreTile />
      {CHECKS.map((check) => (
        <SequenceCard
          key={check}
          className={SequenceCardStyle}
          variant="SurfaceVariant"
          direction="Column"
        >
          <SettingTile
            title={CHECK_TITLES[check]}
            description={status[check].message}
            after={
              <Text size="T200" priority="300">
                {formatCheckLabel(status[check].ready, status[check].supported)}
              </Text>
            }
          />
        </SequenceCard>
      ))}
      {copyOutcome && (
        <Box>
          <InlineMessage tone={copyOutcome === 'copied' ? 'success' : 'critical'}>
            {copyOutcome === 'copied' ? 'Diagnostics copied.' : 'Clipboard copy failed.'}
          </InlineMessage>
        </Box>
      )}
    </SettingsSection>
  );
}
