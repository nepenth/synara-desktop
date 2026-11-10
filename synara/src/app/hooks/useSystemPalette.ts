import { useSyncExternalStore } from 'react';
import { getDesktopSystemPalette, listenDesktopSystemPalette } from '../utils/desktop';
import { normalizeExternalPalette, type ExternalPalette } from '../utils/themeExternalPalette';

export type SystemPaletteState = {
  /** The shell answers `desktop_system_palette` (it has a palette provider). */
  providerAvailable: boolean;
  /** The desktop's palette now; `null` when there is none to follow. */
  palette: ExternalPalette | null;
};

const UNAVAILABLE: SystemPaletteState = { providerAvailable: false, palette: null };

let state: SystemPaletteState = UNAVAILABLE;
let started = false;
const listeners = new Set<() => void>();

const publish = (next: SystemPaletteState) => {
  state = next;
  listeners.forEach((listener) => listener());
};

/**
 * Asks the shell once, then follows its change event for the life of the
 * page. Both are needed: the event only reports changes, and a palette can
 * appear after startup (a theme set while Synara runs).
 */
const start = () => {
  if (started) return;
  started = true;
  void listenDesktopSystemPalette((payload) => {
    publish({ providerAvailable: true, palette: normalizeExternalPalette(payload) });
  });
  void getDesktopSystemPalette().then((result) => {
    if (!result.available) return;
    // An event that arrived first is newer than this answer.
    if (state.providerAvailable) return;
    publish({ providerAvailable: true, palette: normalizeExternalPalette(result.value) });
  });
};

const subscribe = (listener: () => void) => {
  listeners.add(listener);
  start();
  return () => {
    listeners.delete(listener);
  };
};

const getSnapshot = () => state;

/** The desktop's palette for "Follow system theme colors". */
export function useSystemPalette(): SystemPaletteState {
  return useSyncExternalStore(subscribe, getSnapshot, () => UNAVAILABLE);
}
