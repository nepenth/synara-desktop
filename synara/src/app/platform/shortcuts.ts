import {
  setDesktopShortcuts,
  type DesktopShortcutApplyResult,
  type DesktopShortcutConfig,
} from '../utils/desktop';

export type PlatformShortcutConfig = DesktopShortcutConfig;
export type PlatformShortcutApplyResult = DesktopShortcutApplyResult;

export const setPlatformShortcuts = setDesktopShortcuts;
