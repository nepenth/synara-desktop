import React from 'react';
import { useAtom } from 'jotai';
import { SettingsModal } from '../../components/settings-layout';
import { appSettingsAtom } from '../../state/appSettings';
import { Settings } from './Settings';

/** Shows App Settings when something asks for it with `useOpenAppSettings`. */
export function AppSettingsRenderer() {
  const [request, setRequest] = useAtom(appSettingsAtom);
  if (!request) return null;
  const close = () => setRequest(undefined);
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
