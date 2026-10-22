import { useCallback, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useSetting } from '../../state/hooks/settings';
import { settingsAtom } from '../../state/settings';
import { isSynaraDesktop, listen } from '../../utils/desktop';
import { isLinuxOS, isMacOS } from '../../utils/user-agent';
import { getSettingsPath } from '../../pages/pathUtils';
import {
  APP_SHORTCUT_ACTIONS,
  APP_SHORTCUT_MENU_EVENT,
  adjacentRoomLink,
  appShortcutAction,
  nextPageZoom,
  type AppShortcutAction,
} from './appShortcuts';

/**
 * Zoom (Cmd/Ctrl + = − 0), Settings (Cmd/Ctrl + ,) and previous/next room
 * (Alt + ↑/↓) for the signed-in desktop app. Quit lives on the Linux title
 * strip, which is mounted on every screen.
 */
export function DesktopAppShortcuts() {
  const [pageZoom, setPageZoom] = useSetting(settingsAtom, 'pageZoom');
  const navigate = useNavigate();

  const run = useCallback(
    (action: AppShortcutAction): boolean => {
      switch (action) {
        case 'zoom-in':
        case 'zoom-out':
        case 'zoom-reset':
          setPageZoom(nextPageZoom(pageZoom, action));
          return true;
        case 'settings':
          navigate(getSettingsPath());
          return true;
        case 'previous-room':
        case 'next-room': {
          const links = Array.from(
            document.querySelectorAll<HTMLAnchorElement>('a[data-room-nav-link]')
          );
          const target = adjacentRoomLink(links, action);
          if (!target) return false;
          target.click();
          target.scrollIntoView({ block: 'nearest' });
          return true;
        }
        default:
          return false;
      }
    },
    [navigate, pageZoom, setPageZoom]
  );

  useEffect(() => {
    if (!isSynaraDesktop()) return undefined;
    const platform = { macOS: isMacOS(), linux: isLinuxOS() };
    const onKeyDown = (evt: KeyboardEvent) => {
      if (evt.defaultPrevented) return;
      const action = appShortcutAction(evt, platform);
      if (!action || action === 'quit') return;
      if (run(action)) evt.preventDefault();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [run]);

  // macOS View and app menu items send the same actions.
  useEffect(() => {
    if (!isSynaraDesktop()) return undefined;
    let unlisten: (() => void | Promise<void>) | undefined;
    let disposed = false;
    void listen<string>(APP_SHORTCUT_MENU_EVENT, (evt) => {
      const action = APP_SHORTCUT_ACTIONS.find((candidate) => candidate === evt.payload);
      if (action && action !== 'quit') run(action);
    })
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
  }, [run]);

  return null;
}
