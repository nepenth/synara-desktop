import React from 'react';
import { useAtomValue } from 'jotai';
import { SettingsModal } from '../../components/settings-layout';
import { appSettingsAtom, useCloseAppSettings } from '../../state/appSettings';
import { Settings } from './Settings';

/** Shows App Settings when something asks for it with `useOpenAppSettings`. */
export function AppSettingsRenderer() {
  const request = useAtomValue(appSettingsAtom);
  const close = useCloseAppSettings();
  if (!request) return null;
  return (
    <SettingsModal requestClose={close}>
      <Settings
        key={request.nonce}
        initialPage={request.page}
        initialSetting={request.setting}
        requestClose={close}
      />
    </SettingsModal>
  );
}
