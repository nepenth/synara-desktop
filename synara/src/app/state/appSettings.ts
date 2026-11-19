import { useCallback } from 'react';
import { atom, useAtomValue, useSetAtom } from 'jotai';
import type { SettingsLocation } from '../features/settings/settingsPages';

/**
 * App Settings opened over the current screen (not the `/settings/` route),
 * so closing it returns to the room you were in. `nonce` reopens the dialog
 * at a new place when it is already open.
 */
export type AppSettingsRequest = Partial<SettingsLocation> & {
  nonce: number;
  /** Where focus goes when Settings closes, e.g. the avatar that opened it. */
  returnFocus?: () => void;
};

export const appSettingsAtom = atom<AppSettingsRequest | undefined>(undefined);

let nextNonce = 0;
let lastClosedAt = Number.NEGATIVE_INFINITY;

/**
 * A press outside the dialog closes it on pointer-down; the click that
 * follows lands on whatever was under the backdrop. When that is the control
 * that toggles Settings, it must not open Settings again straight away.
 */
export const APP_SETTINGS_REOPEN_GUARD_MS = 400;

export const appSettingsJustClosed = (now = Date.now()): boolean =>
  now - lastClosedAt < APP_SETTINGS_REOPEN_GUARD_MS;

/** Records a close; `now` is for tests. */
export const noteAppSettingsClosed = (now = Date.now()): void => {
  lastClosedAt = now;
};

export type OpenAppSettingsOptions = { returnFocus?: () => void };

/** Opens App Settings; with a location, at that page and row. */
export const useOpenAppSettings = (): ((
  location?: SettingsLocation,
  options?: OpenAppSettingsOptions
) => void) => {
  const setRequest = useSetAtom(appSettingsAtom);
  return useCallback(
    (location?: SettingsLocation, options?: OpenAppSettingsOptions) => {
      nextNonce += 1;
      setRequest({ ...location, nonce: nextNonce, returnFocus: options?.returnFocus });
    },
    [setRequest]
  );
};

/** Closes App Settings and returns focus to whatever opened it. */
export const useCloseAppSettings = (): (() => void) => {
  const setRequest = useSetAtom(appSettingsAtom);
  return useCallback(() => {
    setRequest((current) => {
      if (current) {
        noteAppSettingsClosed();
        const focus = current.returnFocus;
        // After the dialog's focus trap has let go.
        if (focus) window.setTimeout(focus, 0);
      }
      return undefined;
    });
  }, [setRequest]);
};

export const useAppSettingsOpen = (): boolean => useAtomValue(appSettingsAtom) !== undefined;

/**
 * Opens App Settings, or closes it when it is open: the account avatar and
 * the Settings shortcut both toggle, so an accidental press is undone by
 * pressing again.
 */
export const useToggleAppSettings = (): ((
  location?: SettingsLocation,
  options?: OpenAppSettingsOptions
) => void) => {
  const open = useAppSettingsOpen();
  const openSettings = useOpenAppSettings();
  const closeSettings = useCloseAppSettings();
  return useCallback(
    (location?: SettingsLocation, options?: OpenAppSettingsOptions) => {
      if (open) {
        closeSettings();
        return;
      }
      // The press that just closed Settings from outside it lands here too.
      if (appSettingsJustClosed()) return;
      openSettings(location, options);
    },
    [open, openSettings, closeSettings]
  );
};
