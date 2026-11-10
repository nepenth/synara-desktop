import type { TilingModeSetting } from '../state/settings';
import type { DesktopWindowEnvironment } from './desktop';

/**
 * Tiling mode changes window chrome and defaults only, never layout (layout
 * follows window width). Only the Linux shell has it: macOS and Windows keep
 * their native title bars whatever the setting says.
 */
export const resolveTilingMode = (
  setting: TilingModeSetting,
  environment: DesktopWindowEnvironment,
  linuxDesktop: boolean
): boolean => {
  if (!linuxDesktop) return false;
  if (setting === 'on') return true;
  if (setting === 'off') return false;
  return environment === 'tiling';
};
