import React, { useCallback, useEffect, useState } from 'react';
import { Badge, Box, Text, IconButton, Icon, Icons, Scroll, Switch } from 'folds';
import { Page, PageContent, PageHeader } from '../../../components/page';
import { SequenceCard } from '../../../components/sequence-card';
import { SequenceCardStyle } from '../styles.css';
import { SettingTile } from '../../../components/setting-tile';
import { useSetting } from '../../../state/hooks/settings';
import { settingsAtom } from '../../../state/settings';
import { useMatrixClient } from '../../../hooks/useMatrixClient';
import { isNativeMatrixSession } from '../../verification/nativeVerification';
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
