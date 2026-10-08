import React, { useCallback, useEffect, useState } from 'react';
import { Badge, Text, Switch } from 'folds';
import { SequenceCard } from '../../../components/sequence-card';
import { SequenceCardStyle } from '../styles.css';
import { SettingTile } from '../../../components/setting-tile';
import { useSetting } from '../../../state/hooks/settings';
import { settingsAtom } from '../../../state/settings';
import { isNativeMatrixSession } from '../../verification/nativeVerification';
import { isSynaraDesktop } from '../../../utils/desktop';
import { pushEncryptedStateEventsSetting } from '../encryptedStateEvents';
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
import { setNativeAccountData, useNativeAccountData } from '../../../native/nativeAccountData';
import { SettingsPage, SettingsSection } from '../../../components/settings-layout';

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
  const [developerTools, setDeveloperTools] = useSetting(settingsAtom, 'developerTools');
  const [encryptedStateEvents, setEncryptedStateEventsSetting] = useSetting(
    settingsAtom,
    'encryptedStateEvents'
  );
  const [expand, setExpend] = useState(false);
  const [accountDataType, setAccountDataType] = useState<string | null>();

  useEffect(() => {
    void pushEncryptedStateEventsSetting(encryptedStateEvents);
  }, [encryptedStateEvents]);

  const setEncryptedStateEvents = (value: boolean) => {
    setEncryptedStateEventsSetting(value);
  };

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
      title="Developer Tools"
      description="Low-level account data and protocol tools. Changes here take effect immediately."
      requestClose={requestClose}
    >
      <SettingsSection title="Options">
        <SequenceCard
          className={SequenceCardStyle}
          variant="SurfaceVariant"
          direction="Column"
          gap="400"
        >
          <SettingTile
            title="Enable Developer Tools"
            after={<Switch variant="Primary" value={developerTools} onChange={setDeveloperTools} />}
          />
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
        {isSynaraDesktop() && (
          <SequenceCard
            className={SequenceCardStyle}
            variant="SurfaceVariant"
            direction="Column"
            gap="400"
          >
            <SettingTile
              title="Experimental Widgets"
              description="Widgets are configured under General."
            />
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
      </SettingsSection>
      {developerTools && !isNativeMatrixSession() && (
        <AccountData expand={expand} onExpandToggle={setExpend} onSelect={setAccountDataType} />
      )}
      {developerTools && isNativeMatrixSession() && (
        <SettingsSection title="Account Data">
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
        </SettingsSection>
      )}
    </SettingsPage>
  );
}
