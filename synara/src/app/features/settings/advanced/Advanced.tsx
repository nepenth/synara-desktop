import React, { useCallback, useEffect, useId, useState } from 'react';
import { Badge, Box, Button, Input, Spinner, Switch, Text } from 'folds';
import { useTranslation } from 'react-i18next';
import { SequenceCard } from '../../../components/sequence-card';
import { useSetting } from '../../../state/hooks/settings';
import { settingsAtom } from '../../../state/settings';
import { SettingTile } from '../../../components/setting-tile';
import { SequenceCardStyle, SettingsQuietControl } from '../styles.css';
import { reloadApplication } from '../../../../client/initMatrix';
import { isSynaraDesktop } from '../../../utils/desktop';
import {
  closeExperimentalWidgets,
  widgetsSettingsDescription,
} from '../../widgets/experimentalWidgets';
import { isSafeWidgetUrl } from '../../widgets/widgetUrl';
import {
  refreshRtcTransportsNative,
  snapshotRtcTransportsNative,
  type NativeRtcTransportsSnapshot,
} from '../../matrix-rtc/nativeRtcTransports';
import {
  rtcCallAvailabilityCopy,
  rtcTransportsDiagnosticCopy,
} from '../../matrix-rtc/liveCallChrome';
import { isNativeMatrixSession } from '../../verification/nativeVerification';
import { pushEncryptedStateEventsSetting } from '../encryptedStateEvents';
import {
  AccountDataEditor,
  AccountDataSubmitCallback,
} from '../../../components/AccountDataEditor';
import { setNativeAccountData, useNativeAccountData } from '../../../native/nativeAccountData';
import { AccountData } from './AccountData';
import { SettingsPage, SettingsSection } from '../../../components/settings-layout';

/** Room widgets and agent widget URLs; desktop app only. */
function WidgetsSetting() {
  const [experimentalWidgetsEnabled, setExperimentalWidgetsEnabled] = useSetting(
    settingsAtom,
    'experimentalWidgetsEnabled'
  );
  const [agentWidgetEntries, setAgentWidgetEntries] = useSetting(
    settingsAtom,
    'agentWidgetEntries'
  );
  const [agentName, setAgentName] = useState('');
  const [agentUrl, setAgentUrl] = useState('');
  const agentNameId = useId();
  const agentUrlId = useId();

  if (!isSynaraDesktop()) return null;

  return (
    <SequenceCard
      className={SequenceCardStyle}
      variant="SurfaceVariant"
      direction="Column"
      gap="400"
    >
      <SettingTile
        title="Experimental widgets"
        description={widgetsSettingsDescription}
        after={
          <Box gap="200" alignItems="Center">
            <Badge variant="Secondary" fill="Soft" radii="Pill" outlined>
              <Text as="span" size="L400">
                Beta
              </Text>
            </Badge>
            <Switch
              variant="Primary"
              aria-label="Experimental widgets"
              value={experimentalWidgetsEnabled}
              onChange={(value) => {
                if (!value) {
                  void closeExperimentalWidgets();
                }
                setExperimentalWidgetsEnabled(value);
              }}
            />
          </Box>
        }
      />
      {experimentalWidgetsEnabled && (
        <Box direction="Column" gap="200">
          <Text size="T200" priority="300">
            Agent widget URLs stay on this device. Loopback is allowed only here; room-state widgets
            cannot load localhost.
          </Text>
          {agentWidgetEntries.map((entry) => (
            <Box key={entry.id} justifyContent="SpaceBetween" alignItems="Center" gap="200">
              <Box direction="Column" grow="Yes">
                <Text size="T300">{entry.name}</Text>
                <Text size="T200" priority="300">
                  {entry.url}
                </Text>
              </Box>
              <Button
                className={SettingsQuietControl}
                size="300"
                variant="Critical"
                fill="Soft"
                onClick={() =>
                  setAgentWidgetEntries(agentWidgetEntries.filter((item) => item.id !== entry.id))
                }
              >
                <Text size="B300">Remove</Text>
              </Button>
            </Box>
          ))}
          <Box direction="Column" gap="200">
            <Box direction="Column" gap="100">
              <Text as="label" size="L400" priority="300" htmlFor={agentNameId}>
                Agent name
              </Text>
              <Input
                id={agentNameId}
                variant="Background"
                size="400"
                value={agentName}
                onChange={(evt) => setAgentName(evt.currentTarget.value)}
              />
            </Box>
            <Box direction="Column" gap="100">
              <Text as="label" size="L400" priority="300" htmlFor={agentUrlId}>
                Widget URL
              </Text>
              <Input
                id={agentUrlId}
                variant="Background"
                size="400"
                placeholder="https://… or http://127.0.0.1:…"
                value={agentUrl}
                onChange={(evt) => setAgentUrl(evt.currentTarget.value)}
              />
            </Box>
            <Button
              className={SettingsQuietControl}
              size="300"
              variant="Secondary"
              fill="Soft"
              disabled={!agentName.trim() || !isSafeWidgetUrl(agentUrl.trim(), true)}
              onClick={() => {
                const name = agentName.trim();
                const url = agentUrl.trim();
                if (!name || !url || !isSafeWidgetUrl(url, true)) return;
                setAgentWidgetEntries([
                  ...agentWidgetEntries,
                  { id: `agent-${Date.now()}`, name, url },
                ]);
                setAgentName('');
                setAgentUrl('');
              }}
            >
              <Text size="B300">Add agent widget</Text>
            </Button>
          </Box>
        </Box>
      )}
    </SequenceCard>
  );
}

function Experimental() {
  const { t } = useTranslation();
  const [indexedMessageSearch, setIndexedMessageSearch] = useSetting(
    settingsAtom,
    'indexedMessageSearch'
  );
  const [showHiddenEvents, setShowHiddenEvents] = useSetting(settingsAtom, 'showHiddenEvents');
  const [encryptedStateEvents, setEncryptedStateEvents] = useSetting(
    settingsAtom,
    'encryptedStateEvents'
  );

  useEffect(() => {
    void pushEncryptedStateEventsSetting(encryptedStateEvents);
  }, [encryptedStateEvents]);

  return (
    <SettingsSection
      title="Experimental"
      description="Features still being tested. They can change or go away."
    >
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title={t('modernization.settings.indexed_message_search.title', 'Indexed message search')}
          description={t(
            'modernization.settings.indexed_message_search.description',
            'Local experimental index of messages this device has already seen, including encrypted rooms. Not complete homeserver history. Changing this requires a session reload.'
          )}
          after={
            <Switch
              variant="Primary"
              value={indexedMessageSearch}
              onChange={setIndexedMessageSearch}
            />
          }
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Show hidden events"
          description="Show state and unsupported events that are normally hidden from the timeline."
          after={
            <Switch variant="Primary" value={showHiddenEvents} onChange={setShowHiddenEvents} />
          }
        />
      </SequenceCard>
      <WidgetsSetting />
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Encrypted state events (experimental)"
          description="MSC4362. Older clients will not see room name, topic, or avatar. This cannot be turned off for rooms that already opted in; this device will still decrypt those rooms."
          after={
            <Switch
              variant="Primary"
              value={encryptedStateEvents}
              onChange={setEncryptedStateEvents}
            />
          }
        />
      </SequenceCard>
    </SettingsSection>
  );
}

/** Whether this desktop app can place calls; desktop app only. */
function CallAvailabilitySetting() {
  const [snapshot, setSnapshot] = useState<NativeRtcTransportsSnapshot | null>(null);

  useEffect(() => {
    let cancelled = false;
    snapshotRtcTransportsNative()
      .then((next) => {
        if (!cancelled) setSnapshot(next);
      })
      .catch(() => {
        if (!cancelled) setSnapshot(null);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (!isSynaraDesktop()) return null;

  return (
    <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
      <SettingTile
        title="Voice and video calls"
        description={rtcCallAvailabilityCopy({
          status: snapshot?.status ?? null,
          transports: snapshot?.transports,
        })}
      />
    </SequenceCard>
  );
}

/** The MatrixRTC transports the homeserver offers, with a manual refresh. */
function CallTransportSetting() {
  const [snapshot, setSnapshot] = useState<NativeRtcTransportsSnapshot | null>(null);
  const [unavailable, setUnavailable] = useState(false);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async (refresh: boolean) => {
    setBusy(true);
    try {
      const next = refresh
        ? await refreshRtcTransportsNative()
        : await snapshotRtcTransportsNative();
      setSnapshot(next);
      setUnavailable(next === null);
    } catch {
      setSnapshot(null);
      setUnavailable(true);
    } finally {
      setBusy(false);
    }
  }, []);

  useEffect(() => {
    load(false).catch(() => {
      setSnapshot(null);
      setUnavailable(true);
    });
  }, [load]);

  return (
    <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
      <SettingTile title="Call transport">
        <Box direction="Column" gap="100">
          <Text size="T300" data-testid="matrix-rtc-transport-diagnostic">
            {unavailable || !snapshot
              ? 'Synara could not read the call transports for your homeserver.'
              : rtcTransportsDiagnosticCopy(snapshot)}
          </Text>
          <Box>
            <Button
              className={SettingsQuietControl}
              size="300"
              variant="Secondary"
              fill="Soft"
              outlined
              radii="300"
              disabled={busy}
              onClick={() => {
                load(true).catch(() => undefined);
              }}
            >
              {busy ? <Spinner size="300" /> : <Text size="T300">Refresh transport</Text>}
            </Button>
          </Box>
        </Box>
      </SettingTile>
    </SequenceCard>
  );
}

function Calls() {
  return (
    <SettingsSection title="Calls">
      <CallAvailabilitySetting />
      <CallTransportSetting />
    </SettingsSection>
  );
}

type DeveloperProps = {
  accountDataExpanded: boolean;
  onAccountDataExpandedChange: (expand: boolean) => void;
  onSelectAccountData: (type: string | null) => void;
};
function Developer({
  accountDataExpanded,
  onAccountDataExpandedChange,
  onSelectAccountData,
}: DeveloperProps) {
  const [developerTools, setDeveloperTools] = useSetting(settingsAtom, 'developerTools');

  return (
    <SettingsSection
      title="Developer"
      description="Low-level account data and protocol tools. Changes here take effect immediately."
    >
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Enable developer tools"
          description="Show event sources, state editors and raw account data in rooms and settings."
          after={<Switch variant="Primary" value={developerTools} onChange={setDeveloperTools} />}
        />
      </SequenceCard>
      {developerTools && !isNativeMatrixSession() && (
        <AccountData
          expand={accountDataExpanded}
          onExpandToggle={onAccountDataExpandedChange}
          onSelect={onSelectAccountData}
        />
      )}
    </SettingsSection>
  );
}

function Maintenance() {
  return (
    <SettingsSection title="Maintenance">
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Reload application"
          description="Reset navigation and notification caches, then reload the application. Your native message store and login are kept."
          after={
            <Button
              className={SettingsQuietControl}
              onClick={() => reloadApplication()}
              variant="Critical"
              fill="None"
              size="300"
              radii="300"
              outlined
            >
              <Text size="B300">Reload</Text>
            </Button>
          }
        />
      </SequenceCard>
    </SettingsSection>
  );
}

type AdvancedProps = {
  requestClose: () => void;
};
/** Advanced: experimental features, call status, developer tools and maintenance. */
export function Advanced({ requestClose }: AdvancedProps) {
  // Kept here so the list stays open after the editor closes.
  const [accountDataExpanded, setAccountDataExpanded] = useState(false);
  const [accountDataType, setAccountDataType] = useState<string | null>();

  const editingContent = useNativeAccountData(
    accountDataType ?? '',
    undefined,
    typeof accountDataType === 'string'
  );
  const submitAccountData: AccountDataSubmitCallback = useCallback(
    (type, content) => setNativeAccountData(type, content as Record<string, unknown>),
    []
  );

  if (accountDataType !== undefined) {
    return (
      <AccountDataEditor
        type={accountDataType ?? undefined}
        content={accountDataType ? (editingContent ?? undefined) : undefined}
        submitChange={submitAccountData}
        requestClose={() => setAccountDataType(undefined)}
      />
    );
  }

  return (
    <SettingsPage
      title="Advanced"
      description="Experimental features, call status, developer tools and maintenance."
      requestClose={requestClose}
    >
      <Experimental />
      <Calls />
      <Developer
        accountDataExpanded={accountDataExpanded}
        onAccountDataExpandedChange={setAccountDataExpanded}
        onSelectAccountData={setAccountDataType}
      />
      <Maintenance />
    </SettingsPage>
  );
}
