import { useCallback, useEffect, useRef } from 'react';
import { atom, useAtom, useAtomValue, useSetAtom } from 'jotai';
import { useSetting } from '../../state/hooks/settings';
import { desktopPlatformSettingsAtom } from '../../state/settings';
import {
  ROOM_LIST_CURRENT_ROW,
  ROOM_LIST_PANE_ID,
  ROOM_LIST_TOGGLE_ID,
  roomListFocusTarget,
} from './roomListPane';

/** The user's choice: the list is hidden (saved per device). */
export const useRoomListCollapsed = () =>
  useSetting(desktopPlatformSettingsAtom, 'roomListCollapsed');

/**
 * The list is hidden for now by the app, not by the user: at Tablet width a
 * thread pane hides it to make room (L2). Never saved; the user's own show or
 * hide clears it.
 */
const roomListAutoHiddenAtom = atom(false);

/** Hidden by the user's choice or, for now, by the app. */
export const useRoomListHidden = (): boolean => {
  const [collapsed] = useRoomListCollapsed();
  const autoHidden = useAtomValue(roomListAutoHiddenAtom);
  return collapsed || autoHidden;
};

/**
 * While `active`, hide a shown list for the time being and show it again
 * afterwards. A list the user hid stays theirs (nothing to restore), and a
 * list the user shows or hides meanwhile keeps their choice.
 */
export const useAutoHideRoomList = (active: boolean) => {
  const [collapsed] = useRoomListCollapsed();
  const setAutoHidden = useSetAtom(roomListAutoHiddenAtom);
  const collapsedRef = useRef(collapsed);
  collapsedRef.current = collapsed;
  useEffect(() => {
    if (!active || collapsedRef.current) return undefined;
    setAutoHidden(true);
    return () => setAutoHidden(false);
  }, [active, setAutoHidden]);
};

const isEditable = (element: Element | null): boolean =>
  element instanceof HTMLElement &&
  (element.isContentEditable || element.tagName === 'INPUT' || element.tagName === 'TEXTAREA');

/**
 * Hide or show the room list. `revealFocus` (the shortcut) moves focus into
 * the list it shows, unless the user is typing: the composer keeps it.
 */
export const useToggleRoomList = () => {
  const [savedCollapsed, setCollapsed] = useRoomListCollapsed();
  const [autoHidden, setAutoHidden] = useAtom(roomListAutoHiddenAtom);
  const collapsed = savedCollapsed || autoHidden;

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
      // Showing it is the user's choice, also over an automatic hide.
      setAutoHidden(false);
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
    [collapsed, setAutoHidden, setCollapsed]
  );
};
