import React, { useCallback, useEffect, useState } from 'react';
import { Switch } from 'folds';
import { SequenceCard } from '../../../components/sequence-card';
import { SequenceCardStyle } from '../styles.css';
import { SettingTile } from '../../../components/setting-tile';
import { useSetting } from '../../../state/hooks/settings';
import { settingsAtom } from '../../../state/settings';
import { isNativeMatrixSession } from '../../verification/nativeVerification';
import { pushEncryptedStateEventsSetting } from '../encryptedStateEvents';
import {
  AccountDataEditor,
  AccountDataSubmitCallback,
} from '../../../components/AccountDataEditor';
import { AccountData } from './AccountData';
import { setNativeAccountData, useNativeAccountData } from '../../../native/nativeAccountData';
import { SettingsPage, SettingsSection } from '../../../components/settings-layout';

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
      title="Developer tools"
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
            title="Enable developer tools"
            description="Show event sources, state editors and raw account data in rooms and settings."
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
      </SettingsSection>
      {developerTools && !isNativeMatrixSession() && (
        <AccountData expand={expand} onExpandToggle={setExpend} onSelect={setAccountDataType} />
      )}
    </SettingsPage>
  );
}
