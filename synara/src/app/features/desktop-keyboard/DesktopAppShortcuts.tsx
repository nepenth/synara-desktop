import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useStore } from 'jotai';
import { useSetting } from '../../state/hooks/settings';
import { settingsAtom } from '../../state/settings';
import { searchModalAtom } from '../../state/searchModal';
import { roomToUnreadAtom } from '../../state/room/roomToUnread';
import { isSynaraDesktop, listen } from '../../utils/desktop';
import { isLinuxOS, isMacOS } from '../../utils/user-agent';
import {
  getHomePath,
  getInboxNotificationsPath,
  getSettingsPath,
  joinPathComponent,
} from '../../pages/pathUtils';
import { useOptionalNavToActivePathAtom } from '../../state/hooks/navToActivePath';
import { useNavigateToApprovals } from '../../hooks/useNavigateToApprovals';
import { useToggleRoomList } from '../room-list-pane/useRoomListPane';
import { ROOM_LIST_PANE_ID } from '../room-list-pane/roomListPane';
import {
  APP_SHORTCUT_ACTIONS,
  APP_SHORTCUT_MENU_EVENT,
  appShortcut,
  appShortcutAction,
  nextPageZoom,
  type AppShortcutAction,
} from './appShortcuts';
import { portalOpen, runAppShortcutHandler } from './appShortcutHandlers';
import {
  adjacentRoomTarget,
  adjacentUnreadTarget,
  currentRoomListNavSource,
  roomListNavTargets,
} from './roomListNavigation';
import { moveFocusRegion } from './focusRegions';
import { ShortcutHelpSheet } from './ShortcutHelpSheet';

const ROOM_ROW = 'a[data-room-nav-link]';

/**
 * Once the route change has rendered, scroll the open room's row into view
 * (a virtual list renders it a frame or two after it is scrolled to) and,
 * when focus was on a row, move focus with it.
 */
const revealRoomRow = (roomId: string, moveFocus: boolean, framesLeft = 10) => {
  requestAnimationFrame(() => {
    const scope = document.getElementById(ROOM_LIST_PANE_ID) ?? document;
    const row = scope.querySelector<HTMLElement>(
      `${ROOM_ROW}[data-room-nav-id="${CSS.escape(roomId)}"]`
    );
    if (!row || row.getAttribute('aria-current') !== 'page') {
      if (framesLeft > 0) revealRoomRow(roomId, moveFocus, framesLeft - 1);
      return;
    }
    row.scrollIntoView({ block: 'nearest' });
    if (moveFocus) row.focus({ preventScroll: true });
  });
};

/**
 * The one dispatcher for the shortcuts in `appShortcuts.ts`: window keys, and
 * on macOS the View and app menu items, which send the same actions. Quit
 * lives on the Linux title strip, which is mounted on every screen.
 */
export function DesktopAppShortcuts() {
  const [pageZoom, setPageZoom] = useSetting(settingsAtom, 'pageZoom');
  const navigate = useNavigate();
  const store = useStore();
  const toggleRoomList = useToggleRoomList();
  const navToActivePathAtom = useOptionalNavToActivePathAtom();
  const navigateToApprovals = useNavigateToApprovals();
  const [helpOpen, setHelpOpen] = useState(false);
  const platform = useMemo(() => ({ macOS: isMacOS(), linux: isLinuxOS() }), []);

  const goToRoom = useCallback(
    (
      action: Extract<
        AppShortcutAction,
        'previous-room' | 'next-room' | 'previous-unread-room' | 'next-unread-room'
      >
    ): boolean => {
      const source = currentRoomListNavSource();
      if (!source) return false;
      const targets = roomListNavTargets(source);
      const direction =
        action === 'next-room' || action === 'next-unread-room' ? 'next' : 'previous';
      const target =
        action === 'previous-room' || action === 'next-room'
          ? adjacentRoomTarget(targets, source.selectedRoomId, direction)
          : adjacentUnreadTarget(targets, source.selectedRoomId, direction, (roomId) =>
              store.get(roomToUnreadAtom).has(roomId)
            );
      if (!target) return false;
      const active = document.activeElement;
      const focusOnRow = active instanceof HTMLElement && active.matches(ROOM_ROW);
      target.section.reveal?.(target.index);
      navigate(target.path);
      revealRoomRow(target.roomId, focusOnRow);
      return true;
    },
    [navigate, store]
  );

  const run = useCallback(
    (action: AppShortcutAction): boolean => {
      switch (action) {
        case 'zoom-in':
        case 'zoom-out':
        case 'zoom-reset':
          setPageZoom(nextPageZoom(pageZoom, action));
          return true;
        case 'settings':
        case 'go-settings':
          navigate(getSettingsPath());
          return true;
        // Home and Activity reopen where the user last was in them, as
        // their bar buttons do.
        case 'go-home': {
          const last = navToActivePathAtom && store.get(navToActivePathAtom).get('home');
          navigate(last ? joinPathComponent(last) : getHomePath());
          return true;
        }
        case 'go-activity': {
          const last = navToActivePathAtom && store.get(navToActivePathAtom).get('inbox');
          navigate(last ? joinPathComponent(last) : getInboxNotificationsPath());
          return true;
        }
        case 'go-approvals':
          navigateToApprovals();
          return true;
        case 'search':
          // A second press closes it; another dialog keeps the key.
          if (store.get(searchModalAtom)) store.set(searchModalAtom, false);
          else if (!portalOpen()) store.set(searchModalAtom, true);
          return true;
        case 'shortcut-help':
          if (helpOpen) setHelpOpen(false);
          else if (!portalOpen()) setHelpOpen(true);
          return true;
        case 'previous-room':
        case 'next-room':
        case 'previous-unread-room':
        case 'next-unread-room':
          return goToRoom(action);
        case 'focus-next-region':
        case 'focus-previous-region':
          if (portalOpen()) return false;
          return moveFocusRegion(document, action === 'focus-next-region' ? 'next' : 'previous');
        case 'close-thread':
          return runAppShortcutHandler(action);
        case 'toggle-room-list':
          toggleRoomList({ revealFocus: true });
          return true;
        default:
          return false;
      }
    },
    [
      goToRoom,
      helpOpen,
      navToActivePathAtom,
      navigate,
      navigateToApprovals,
      pageZoom,
      setPageZoom,
      store,
      toggleRoomList,
    ]
  );

  useEffect(() => {
    const desktop = isSynaraDesktop();
    const onKeyDown = (evt: KeyboardEvent) => {
      if (evt.defaultPrevented) return;
      const action = appShortcutAction(evt, platform);
      if (!action) return;
      const shortcut = appShortcut(action);
      if (shortcut.external || (shortcut.desktopOnly && !desktop)) return;
      if (run(action)) evt.preventDefault();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [platform, run]);

  // macOS View and app menu items send the same actions.
  useEffect(() => {
    if (!isSynaraDesktop()) return undefined;
    let unlisten: (() => void | Promise<void>) | undefined;
    let disposed = false;
    void listen<string>(APP_SHORTCUT_MENU_EVENT, (evt) => {
      const action = APP_SHORTCUT_ACTIONS.find((candidate) => candidate === evt.payload);
      if (action && !appShortcut(action).external) run(action);
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

  return helpOpen ? (
    <ShortcutHelpSheet platform={platform} onClose={() => setHelpOpen(false)} />
  ) : null;
}
