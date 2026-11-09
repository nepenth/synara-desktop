import { useCallback } from 'react';
import { useSetting } from '../../state/hooks/settings';
import { desktopPlatformSettingsAtom } from '../../state/settings';
import { ROOM_LIST_PANE_ID, ROOM_LIST_TOGGLE_ID, roomListFocusTarget } from './roomListPane';

export const useRoomListCollapsed = () =>
  useSetting(desktopPlatformSettingsAtom, 'roomListCollapsed');

const isEditable = (element: Element | null): boolean =>
  element instanceof HTMLElement &&
  (element.isContentEditable || element.tagName === 'INPUT' || element.tagName === 'TEXTAREA');

/**
 * Hide or show the room list. `revealFocus` (the shortcut) moves focus into
 * the list it shows, unless the user is typing: the composer keeps it.
 */
export const useToggleRoomList = () => {
  const [collapsed, setCollapsed] = useRoomListCollapsed();

  return useCallback(
    (options: { revealFocus?: boolean } = {}) => {
      const pane = document.getElementById(ROOM_LIST_PANE_ID);
      if (!collapsed) {
        // The hidden pane is inert, which would drop focus to <body>.
        if (pane?.contains(document.activeElement)) {
          document.getElementById(ROOM_LIST_TOGGLE_ID)?.focus();
        }
        setCollapsed(true);
        return;
      }
      setCollapsed(false);
      if (!options.revealFocus || !pane || isEditable(document.activeElement)) return;
      // After React drops `inert`.
      requestAnimationFrame(() => roomListFocusTarget(pane)?.focus());
    },
    [collapsed, setCollapsed]
  );
};
