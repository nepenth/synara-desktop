import { useEffect } from 'react';
import { isSynaraDesktop, listen } from './desktop';

/**
 * "The app is about to quit": anything holding unsaved state (a composer's
 * pending draft save) writes it now. Quitting does not reliably fire
 * `pagehide` or `beforeunload` in the desktop web views, so the app says so
 * itself.
 *
 * Two sources dispatch it: the page's own Quit (Ctrl+Q on Linux), before it
 * asks the shell to exit, and the shell, which emits
 * `NATIVE_BEFORE_QUIT_EVENT` when any other quit (tray, app menu, session
 * end) reaches it; the shell then spends a bounded time stopping sync, which
 * leaves the page time to handle it.
 */
export const BEFORE_QUIT_EVENT = 'synara:before-quit';

/** Emitted by the shell (`src-tauri/src/lib.rs`, `RunEvent::ExitRequested`). */
export const NATIVE_BEFORE_QUIT_EVENT = 'synara://before-quit';

export const announceBeforeQuit = (target: EventTarget = window): void => {
  target.dispatchEvent(new Event(BEFORE_QUIT_EVENT));
};

/** Forwards the shell's quit notice to `BEFORE_QUIT_EVENT`. Mount once. */
export function useNativeBeforeQuit(): void {
  useEffect(() => {
    if (!isSynaraDesktop()) return undefined;
    let unlisten: (() => void | Promise<void>) | undefined;
    let disposed = false;
    void listen<null>(NATIVE_BEFORE_QUIT_EVENT, () => announceBeforeQuit())
      .then((unsubscribe) => {
        if (disposed) {
          void unsubscribe?.();
          return;
        }
        unlisten = unsubscribe;
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
      void unlisten?.();
    };
  }, []);
}
