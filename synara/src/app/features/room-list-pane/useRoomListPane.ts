import { useCallback } from 'react';
import { useSetting } from '../../state/hooks/settings';
import { desktopPlatformSettingsAtom } from '../../state/settings';
import {
  ROOM_LIST_CURRENT_ROW,
  ROOM_LIST_PANE_ID,
  ROOM_LIST_TOGGLE_ID,
  roomListFocusTarget,
} from './roomListPane';

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
        // The hidden pane is inert, which would drop focus to <body>: hand it
        // to the page header's "Show room list" button, which mounts with the
        // hidden state.
        const focusWasInPane = !!pane?.contains(document.activeElement);
        setCollapsed(true);
        if (focusWasInPane) {
          const focusToggle = (framesLeft: number) =>
            requestAnimationFrame(() => {
              const toggle = document.getElementById(ROOM_LIST_TOGGLE_ID);
              if (toggle) toggle.focus();
              else if (framesLeft > 0) focusToggle(framesLeft - 1);
            });
          focusToggle(10);
        }
        return;
      }
      setCollapsed(false);
      if (!options.revealFocus || !pane || isEditable(document.activeElement)) return;
      // Focus once React has dropped `inert` (an inert element ignores
      // focus) and the open room's row is back: at zero width the list's
      // virtualizer renders no rows, so for a few frames only the search
      // field exists. Fall back to it when no room row appears.
      const focusWhenShown = (framesLeft: number) =>
        requestAnimationFrame(() => {
          const current = pane.querySelector<HTMLElement>(ROOM_LIST_CURRENT_ROW);
          if (pane.hasAttribute('inert') || (!current && framesLeft > 0)) {
            if (framesLeft > 0) focusWhenShown(framesLeft - 1);
            return;
          }
          (current ?? roomListFocusTarget(pane))?.focus();
        });
      focusWhenShown(20);
    },
    [collapsed, setCollapsed]
  );
};
