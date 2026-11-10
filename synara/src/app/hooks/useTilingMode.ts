import { useSetting } from '../state/hooks/settings';
import { desktopPlatformSettingsAtom } from '../state/settings';
import { getDesktopWindowEnvironment, isSynaraDesktop } from '../utils/desktop';
import { resolveTilingMode } from '../utils/tilingMode';
import { isLinuxOS } from '../utils/user-agent';

/** The borderless Linux shell, the only one with an in-app title strip. */
export const isLinuxDesktopShell = (): boolean => isSynaraDesktop() && isLinuxOS();

/**
 * True while the Linux shell runs in tiling mode: the Tiling mode setting, or
 * on Auto what the shell detected at startup. A change to the setting applies
 * at once.
 */
export function useTilingMode(): boolean {
  const [tilingMode] = useSetting(desktopPlatformSettingsAtom, 'tilingMode');
  return resolveTilingMode(tilingMode, getDesktopWindowEnvironment(), isLinuxDesktopShell());
}
