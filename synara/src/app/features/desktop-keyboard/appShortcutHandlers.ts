import { useEffect } from 'react';
import type { AppShortcutAction } from './appShortcuts';

/**
 * Shortcuts whose action belongs to a mounted component (the thread pane's
 * close) are still dispatched by `DesktopAppShortcuts`; the component only
 * registers what the action does while it is mounted. The most recently
 * mounted handler wins. A handler returns false when it did not act, so the
 * key keeps its default.
 */
type AppShortcutHandler = () => boolean;

const handlers = new Map<AppShortcutAction, AppShortcutHandler[]>();

export const registerAppShortcutHandler = (
  action: AppShortcutAction,
  handler: AppShortcutHandler
): (() => void) => {
  const stack = handlers.get(action) ?? [];
  stack.push(handler);
  handlers.set(action, stack);
  return () => {
    const index = stack.lastIndexOf(handler);
    if (index >= 0) stack.splice(index, 1);
  };
};

export const runAppShortcutHandler = (action: AppShortcutAction): boolean =>
  handlers.get(action)?.at(-1)?.() ?? false;

export const useAppShortcutHandler = (action: AppShortcutAction, handler: AppShortcutHandler) => {
  useEffect(() => registerAppShortcutHandler(action, handler), [action, handler]);
};

/** A dialog or menu is open over the app (folds portals render into this container). */
export const portalOpen = (): boolean => {
  const portal = document.getElementById('portalContainer');
  return !!portal && portal.children.length > 0;
};
