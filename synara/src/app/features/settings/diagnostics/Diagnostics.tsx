import React, { useCallback, useEffect, useState } from 'react';
import { useAtomValue } from 'jotai';
import { Box, Button, Header, Icon, IconButton, Icons, Scroll, Switch, Text } from 'folds';
import { Page, PageContent, PageHeader } from '../../../components/page';
import { SequenceCard } from '../../../components/sequence-card';
import { SettingTile } from '../../../components/setting-tile';
import { useSetting } from '../../../state/hooks/settings';
import { desktopPlatformSettingsAtom } from '../../../state/settings';
import {
  clearPlatformDiagnostics,
  getPlatformDiagnosticsStatus,
  readPlatformDiagnosticsReport,
  savePlatformFile,
  type PlatformDiagnosticsStatus,
} from '../../../platform';
import { copyToClipboard } from '../../../utils/dom';
import {
  refreshDesktopDiagnosticsConfig,
  updateDesktopDiagnosticsConfig,
} from '../../../utils/clientDiagnostics';
import {
  compactDiagnosticsReport,
  MAX_CLIPBOARD_REPORT_CHARS,
} from '../../../utils/diagnosticsReport';
import { SequenceCardStyle } from '../styles.css';

const formatBytes = (bytes: number): string => {
  if (!Number.isFinite(bytes) || bytes <= 0) return '0 KB';
  if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
};

const formatTimestamp = (timestampMs?: number): string => {
  if (typeof timestampMs !== 'number' || !Number.isFinite(timestampMs)) return 'Not available';
  return new Date(timestampMs).toLocaleString();
};

const diagnosticsFilename = (): string => {
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
  return `synara-diagnostics-${timestamp}.json`;
};

type DiagnosticsProps = {
  requestClose: () => void;
};

export function Diagnostics({ requestClose }: DiagnosticsProps) {
  const platformSettings = useAtomValue(desktopPlatformSettingsAtom);
  const [enabled, setEnabled] = useSetting(
    desktopPlatformSettingsAtom,
    'desktopDiagnosticsEnabled'
  );
  const [performance, setPerformance] = useSetting(
    desktopPlatformSettingsAtom,
    'desktopDiagnosticsPerformance'
  );
  const [session, setSession] = useSetting(
    desktopPlatformSettingsAtom,
    'desktopDiagnosticsSession'
  );
  const [roomState, setRoomState] = useSetting(
    desktopPlatformSettingsAtom,
    'desktopDiagnosticsRoomState'
  );
  const [overlay, setOverlay] = useSetting(
    desktopPlatformSettingsAtom,
    'desktopDiagnosticsOverlay'
  );

  const [status, setStatus] = useState<PlatformDiagnosticsStatus>();
  const [busyAction, setBusyAction] = useState<'copy' | 'export' | 'clear'>();
  const [feedback, setFeedback] = useState<string>();

  const refreshStatus = useCallback(async () => {
    try {
      setStatus(await getPlatformDiagnosticsStatus());
    } catch {
      setStatus({ available: false, entryCount: 0, sizeBytes: 0 });
    }
  }, []);

  useEffect(() => {
    void refreshStatus();
  }, [refreshStatus]);

  useEffect(() => {
    refreshDesktopDiagnosticsConfig(platformSettings);
  }, [platformSettings]);

  const handleEnabledChange = (nextEnabled: boolean) => {
    updateDesktopDiagnosticsConfig({
      desktopDiagnosticsEnabled: nextEnabled,
      desktopDiagnosticsOverlay: nextEnabled ? overlay : false,
    });
    setEnabled(nextEnabled);
    if (!nextEnabled && overlay) setOverlay(false);
    setFeedback(nextEnabled ? 'Diagnostic capture enabled.' : 'Diagnostic capture paused.');
  };

  const handlePerformanceChange = (nextEnabled: boolean) => {
    updateDesktopDiagnosticsConfig({
      desktopDiagnosticsPerformance: nextEnabled,
      desktopDiagnosticsOverlay: nextEnabled ? overlay : false,
    });
    setPerformance(nextEnabled);
    if (!nextEnabled && overlay) setOverlay(false);
  };

  const readReport = async (): Promise<string | undefined> => {
    const report = await readPlatformDiagnosticsReport();
    if (!report?.trim()) {
      setFeedback('No diagnostic records are available yet.');
      return undefined;
    }
    return report;
  };

  const handleCopy = async () => {
    setBusyAction('copy');
    setFeedback(undefined);
    try {
      const report = await readReport();
      if (!report) return;
      copyToClipboard(compactDiagnosticsReport(report));
      await refreshStatus();
      setFeedback(
        report.length > MAX_CLIPBOARD_REPORT_CHARS
          ? 'Recent diagnostics copied. Export the report for the complete capture.'
          : 'Diagnostics copied.'
      );
    } catch {
      setFeedback('Diagnostics could not be copied.');
    } finally {
      setBusyAction(undefined);
    }
  };

  const handleExport = async () => {
    setBusyAction('export');
    setFeedback(undefined);
    try {
      const report = await readReport();
      if (!report) return;
      const path = await savePlatformFile(
        new Blob([report], { type: 'application/json' }),
        diagnosticsFilename()
      );
      await refreshStatus();
      setFeedback(path ? 'Diagnostic report saved.' : 'Diagnostic report was not saved.');
    } catch {
      setFeedback('Diagnostic report could not be exported.');
    } finally {
      setBusyAction(undefined);
    }
  };

  const handleClear = async () => {
    if (!window.confirm('Clear all diagnostic records stored by this desktop client?')) return;
    setBusyAction('clear');
    setFeedback(undefined);
    try {
      const cleared = await clearPlatformDiagnostics();
      setFeedback(cleared ? 'Diagnostics cleared.' : 'Diagnostics could not be cleared.');
      await refreshStatus();
    } catch {
      setFeedback('Diagnostics could not be cleared.');
    } finally {
      setBusyAction(undefined);
    }
  };

  const categoryControlsDisabled = !enabled;
  const reportActionsDisabled = busyAction !== undefined || status?.available !== true;

  return (
    <Page>
      <PageHeader outlined={false}>
        <Box grow="Yes" gap="200">
          <Box grow="Yes" alignItems="Center" gap="200">
            <Text size="H3" truncate>
              Diagnostics
            </Text>
          </Box>
          <Box shrink="No">
            <IconButton onClick={requestClose} variant="Surface">
              <Icon src={Icons.Cross} />
            </IconButton>
          </Box>
        </Box>
      </PageHeader>
      <Box grow="Yes">
        <Scroll hideTrack visibility="Hover">
          <PageContent>
            <Box direction="Column" gap="700">
              <Box direction="Column" gap="100">
                <Text size="L400">Diagnostic Capture</Text>
                <SequenceCard
                  className={SequenceCardStyle}
                  variant="SurfaceVariant"
                  direction="Column"
                  gap="400"
                >
                  <SettingTile
                    title="Enable Diagnostic Capture"
                    description="Record additional privacy-filtered evidence while reproducing a problem. Normal client behavior is unchanged."
                    after={
                      <Switch variant="Primary" value={enabled} onChange={handleEnabledChange} />
                    }
                  />
                  <SettingTile
                    title="Performance"
                    description="Capture frame cadence, long tasks, rendered timeline size, and slow timeline operations."
                    after={
                      <Switch
                        variant="Primary"
                        value={enabled && performance}
                        disabled={categoryControlsDisabled}
                        onChange={handlePerformanceChange}
                      />
                    }
                  />
                  <SettingTile
                    title="Session Persistence"
                    description="Capture credential-store availability, bootstrap source, token-refresh outcomes, and sync lifecycle without recording credentials."
                    after={
                      <Switch
                        variant="Primary"
                        value={enabled && session}
                        disabled={categoryControlsDisabled}
                        onChange={(nextEnabled) => {
                          updateDesktopDiagnosticsConfig({
                            desktopDiagnosticsSession: nextEnabled,
                          });
                          setSession(nextEnabled);
                        }}
                      />
                    }
                  />
                  <SettingTile
                    title="Room State and Positioning"
                    description="Capture room-open decisions, read-marker outcomes, recent-room updates, pagination, anchoring, and unexpected scroll movement."
                    after={
                      <Switch
                        variant="Primary"
                        value={enabled && roomState}
                        disabled={categoryControlsDisabled}
                        onChange={(nextEnabled) => {
                          updateDesktopDiagnosticsConfig({
                            desktopDiagnosticsRoomState: nextEnabled,
                          });
                          setRoomState(nextEnabled);
                        }}
                      />
                    }
                  />
                  <SettingTile
                    title="Performance Overlay"
                    description="Show live frame rate, long-task, timeline-row, and memory counters over the client."
                    after={
                      <Switch
                        variant="Primary"
                        value={enabled && performance && overlay}
                        disabled={!enabled || !performance}
                        onChange={(nextEnabled) => {
                          updateDesktopDiagnosticsConfig({
                            desktopDiagnosticsOverlay: nextEnabled,
                          });
                          setOverlay(nextEnabled);
                        }}
                      />
                    }
                  />
                </SequenceCard>
              </Box>

              <Box direction="Column" gap="100">
                <Text size="L400">Stored Report</Text>
                <SequenceCard
                  className={SequenceCardStyle}
                  variant="SurfaceVariant"
                  direction="Column"
                  gap="400"
                >
                  <SettingTile
                    title={
                      status?.available === false
                        ? 'Native diagnostics unavailable'
                        : 'Local diagnostics'
                    }
                    description={
                      status?.available === false
                        ? 'Diagnostic report storage is not available in this client.'
                        : `${status?.entryCount ?? 0} records · ${formatBytes(
                            status?.sizeBytes ?? 0
                          )}`
                    }
                  />
                  {status?.available && status.entryCount > 0 && (
                    <Box direction="Column" gap="100">
                      <Text size="T200" priority="300">
                        {`Oldest record: ${formatTimestamp(status.oldestTimestampMs)}`}
                      </Text>
                      <Text size="T200" priority="300">
                        {`Newest record: ${formatTimestamp(status.newestTimestampMs)}`}
                      </Text>
                    </Box>
                  )}
                  <Box wrap="Wrap" gap="200">
                    <Button
                      size="300"
                      variant="Primary"
                      fill="Soft"
                      radii="300"
                      disabled={reportActionsDisabled}
                      onClick={handleExport}
                    >
                      <Text size="B300">
                        {busyAction === 'export' ? 'Exporting…' : 'Export report'}
                      </Text>
                    </Button>
                    <Button
                      size="300"
                      variant="Secondary"
                      fill="Soft"
                      radii="300"
                      disabled={reportActionsDisabled}
                      onClick={handleCopy}
                    >
                      <Text size="B300">{busyAction === 'copy' ? 'Copying…' : 'Copy recent'}</Text>
                    </Button>
                    <Button
                      size="300"
                      variant="Critical"
                      fill="None"
                      radii="300"
                      disabled={reportActionsDisabled}
                      onClick={handleClear}
                    >
                      <Text size="B300">
                        {busyAction === 'clear' ? 'Clearing…' : 'Clear records'}
                      </Text>
                    </Button>
                  </Box>
                  {feedback && <Text size="T200">{feedback}</Text>}
                </SequenceCard>
              </Box>

              <Box direction="Column" gap="100">
                <Header size="300">
                  <Text size="L400">Privacy</Text>
                </Header>
                <SequenceCard
                  className={SequenceCardStyle}
                  variant="SurfaceVariant"
                  direction="Column"
                  gap="200"
                >
                  <Text size="T300">
                    Diagnostic reports are stored only on this device until you export or clear
                    them. Synara excludes message bodies, access and refresh tokens, Matrix user,
                    room and event identifiers, server URLs, and attachment contents.
                  </Text>
                  <Text size="T200" priority="300">
                    Review an exported report before sharing it outside your trusted support or
                    development team.
                  </Text>
                </SequenceCard>
              </Box>
            </Box>
          </PageContent>
        </Scroll>
      </Box>
    </Page>
  );
}
