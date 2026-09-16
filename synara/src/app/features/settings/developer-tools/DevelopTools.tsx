import React, { useCallback, useEffect, useState } from 'react';
import { Badge, Box, Text, IconButton, Icon, Icons, Scroll, Switch, Button, Input } from 'folds';
import { Page, PageContent, PageHeader } from '../../../components/page';
import { SequenceCard } from '../../../components/sequence-card';
import { SequenceCardStyle, SettingsQuietControl } from '../styles.css';
import { SettingTile } from '../../../components/setting-tile';
import { useSetting } from '../../../state/hooks/settings';
import { settingsAtom } from '../../../state/settings';
import { useMatrixClient } from '../../../hooks/useMatrixClient';
import { isNativeMatrixSession } from '../../verification/nativeVerification';
import { closeExperimentalWidgets } from '../../widgets/experimentalWidgets';
import { isSafeWidgetUrl } from '../../widgets/widgetUrl';
import { isSynaraDesktop } from '../../../utils/desktop';
import {
  AccountDataEditor,
  AccountDataSubmitCallback,
} from '../../../components/AccountDataEditor';
import { AccountData } from './AccountData';
import {
  getPlatformSecretStoreBackendLabel,
  getPlatformSecretStoreSessionPersistence,
  getPlatformSecretStoreStatusDescription,
  getPlatformSecretStoreStatusLabel,
  platformSessionStore,
  type PlatformSecretStoreStatus,
} from '../../../platform';

function NativeSessionStoreStatus() {
  const [status, setStatus] = useState<PlatformSecretStoreStatus>();

  useEffect(() => {
    let active = true;

    platformSessionStore.getStatus().then((nextStatus) => {
      if (active) setStatus(nextStatus);
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
  const backendLabel = status ? getPlatformSecretStoreBackendLabel(status.backend) : 'Checking';
  const statusLabel = status ? getPlatformSecretStoreStatusLabel(status) : 'Checking';
  const details = status ? getPlatformSecretStoreStatusDescription(status) : backendLabel;

  return (
    <SettingTile
      title="Native Session Store"
      description={details}
      after={
        <Badge variant={badgeVariant} fill="Soft" radii="Pill" outlined>
          <Text as="span" size="L400">
            {statusLabel}
          </Text>
        </Badge>
      }
    />
  );
}

type DeveloperToolsProps = {
  requestClose: () => void;
};
export function DeveloperTools({ requestClose }: DeveloperToolsProps) {
  const mx = useMatrixClient();
  const [developerTools, setDeveloperTools] = useSetting(settingsAtom, 'developerTools');
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
  const [expand, setExpend] = useState(false);
  const [accountDataType, setAccountDataType] = useState<string | null>();

  const submitAccountData: AccountDataSubmitCallback = useCallback(
    async (type, content) => {
      await mx.setAccountData(type as any, content as any);
    },
    [mx]
  );

  if (accountDataType !== undefined) {
    return (
      <AccountDataEditor
        type={accountDataType ?? undefined}
        content={
          accountDataType ? mx.getAccountData(accountDataType as any)?.getContent() : undefined
        }
        submitChange={submitAccountData}
        requestClose={() => setAccountDataType(undefined)}
      />
    );
  }

  return (
    <Page>
      <PageHeader outlined={false}>
        <Box grow="Yes" gap="200">
          <Box grow="Yes" alignItems="Center" gap="200">
            <Text size="H3" truncate>
              Developer Tools
            </Text>
          </Box>
          <Box shrink="No">
            <IconButton
              className={SettingsQuietControl}
              onClick={requestClose}
              variant="Surface"
              fill="None"
              aria-label="Close"
            >
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
                <Text size="L400">Options</Text>
                <SequenceCard
                  className={SequenceCardStyle}
                  variant="SurfaceVariant"
                  direction="Column"
                  gap="400"
                >
                  <SettingTile
                    title="Enable Developer Tools"
                    after={
                      <Switch
                        variant="Primary"
                        value={developerTools}
                        onChange={setDeveloperTools}
                      />
                    }
                  />
                </SequenceCard>
                {isSynaraDesktop() && (
                  <SequenceCard
                    className={SequenceCardStyle}
                    variant="SurfaceVariant"
                    direction="Column"
                    gap="400"
                  >
                    <SettingTile
                      title="Experimental Widgets"
                      description="Widgets run third-party or local web code as the logged-in user for any granted capabilities. Default off."
                      after={
                        <Switch
                          variant="Primary"
                          value={experimentalWidgetsEnabled}
                          onChange={(value) => {
                            if (!value) {
                              void closeExperimentalWidgets();
                            }
                            setExperimentalWidgetsEnabled(value);
                          }}
                        />
                      }
                    />
                    {experimentalWidgetsEnabled && (
                      <Box direction="Column" gap="200">
                        <Text size="T200" priority="300">
                          Agent widget URLs stay on this device. Loopback is allowed only here;
                          room-state widgets cannot load localhost.
                        </Text>
                        {agentWidgetEntries.map((entry) => (
                          <Box
                            key={entry.id}
                            justifyContent="SpaceBetween"
                            alignItems="Center"
                            gap="200"
                          >
                            <Box direction="Column" grow="Yes">
                              <Text size="T300">{entry.name}</Text>
                              <Text size="T200" priority="300">
                                {entry.url}
                              </Text>
                            </Box>
                            <Button
                              size="300"
                              variant="Critical"
                              fill="Soft"
                              onClick={() =>
                                setAgentWidgetEntries(
                                  agentWidgetEntries.filter((item) => item.id !== entry.id)
                                )
                              }
                            >
                              <Text size="B300">Remove</Text>
                            </Button>
                          </Box>
                        ))}
                        <Box direction="Column" gap="200">
                          <Input
                            variant="Background"
                            size="400"
                            placeholder="Agent name"
                            value={agentName}
                            onChange={(evt) => setAgentName(evt.currentTarget.value)}
                          />
                          <Input
                            variant="Background"
                            size="400"
                            placeholder="https://… or http://127.0.0.1:…"
                            value={agentUrl}
                            onChange={(evt) => setAgentUrl(evt.currentTarget.value)}
                          />
                          <Button
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
                )}
                {developerTools && (
                  <SequenceCard
                    className={SequenceCardStyle}
                    variant="SurfaceVariant"
                    direction="Column"
                    gap="400"
                  >
                    <NativeSessionStoreStatus />
                  </SequenceCard>
                )}
              </Box>
              {developerTools && !isNativeMatrixSession() && (
                <AccountData
                  expand={expand}
                  onExpandToggle={setExpend}
                  onSelect={setAccountDataType}
                />
              )}
              {developerTools && isNativeMatrixSession() && (
                <Box direction="Column" gap="100">
                  <Text size="L400">Account Data</Text>
                  <SequenceCard
                    className={SequenceCardStyle}
                    variant="SurfaceVariant"
                    direction="Column"
                    gap="400"
                  >
                    <SettingTile
                      title="Global"
                      description="Account-data browsing is not available in this native session."
                    />
                  </SequenceCard>
                </Box>
              )}
            </Box>
          </PageContent>
        </Scroll>
      </Box>
    </Page>
  );
}
