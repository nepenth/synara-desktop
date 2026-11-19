import { useCallback } from 'react';
import { atom, useSetAtom } from 'jotai';
import type { SettingsLocation } from '../features/settings/settingsPages';

/**
 * App Settings opened over the current screen (not the `/settings/` route),
 * so closing it returns to the room you were in. `nonce` reopens the dialog
 * at a new place when it is already open.
 */
export type AppSettingsRequest = Partial<SettingsLocation> & { nonce: number };

export const appSettingsAtom = atom<AppSettingsRequest | undefined>(undefined);

let nextNonce = 0;

/** Opens App Settings; with a location, at that page and row. */
export const useOpenAppSettings = (): ((location?: SettingsLocation) => void) => {
  const setRequest = useSetAtom(appSettingsAtom);
  return useCallback(
    (location?: SettingsLocation) => {
      nextNonce += 1;
      setRequest({ ...location, nonce: nextNonce });
    },
    [setRequest]
  );
};
