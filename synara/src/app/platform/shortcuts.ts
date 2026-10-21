import {
  setDesktopShortcuts,
  type DesktopShortcutApplyResult,
  type DesktopShortcutConfig,
} from '../utils/desktop';

export type PlatformShortcutConfig = DesktopShortcutConfig;
export type PlatformShortcutApplyResult = DesktopShortcutApplyResult;

export const setPlatformShortcuts = setDesktopShortcuts;

/**
 * Settings-form check for the three global shortcuts. An empty value turns
 * that shortcut off; enabled shortcuts must differ (case and spaces ignored,
 * as in the Rust parser). Rust validates again before registering.
 */
export const desktopShortcutConfigProblem = (
  values: readonly string[]
): 'duplicate' | undefined => {
  const enabled = values
    .map((value) => value.trim().replace(/\s+/g, '').toLowerCase())
    .filter((value) => value.length > 0);
  return new Set(enabled).size === enabled.length ? undefined : 'duplicate';
};
