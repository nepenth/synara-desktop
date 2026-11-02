import type { Path } from 'react-router-dom';
import { getLocalStorageItem } from '../../state/utils/atomWithLocalStorage';
import { NAV_TO_ACTIVE_PATH_PREFIX } from '../../state/sessions';
import { joinPathComponent } from '../pathUtils';

/**
 * Cold start and "nothing selected" open the last room of a navigation scope
 * (Slack), not the Welcome page. `navToActivePath` already records the last
 * route per scope; this reads that store outside React (a route loader) and
 * accepts only room routes under the scope's base path(s). Utility routes
 * (create, join, search) and the base path itself are not restored. Home is
 * one list of rooms and DMs (NAV-12), so it passes both `/home/` and
 * `/direct/` and restores whichever was open last.
 */
export const lastActiveRoomPath = (
  activePaths: ReadonlyMap<string, Path> | Record<string, Path> | undefined,
  navId: string,
  basePath: string | readonly string[],
  utilitySegments: readonly string[]
): string | undefined => {
  const stored =
    activePaths instanceof Map
      ? activePaths.get(navId)
      : (activePaths as Record<string, Path> | undefined)?.[navId];
  if (!stored || typeof stored.pathname !== 'string') return undefined;
  const bases = (typeof basePath === 'string' ? [basePath] : basePath).map((path) =>
    path.endsWith('/') ? path : `${path}/`
  );
  const base = bases.find((path) => stored.pathname.startsWith(path));
  if (!base) return undefined;
  const rest = stored.pathname.slice(base.length);
  const [first] = rest.split('/');
  if (!first) return undefined;
  // Route constants carry a trailing slash (`create/`); compare the segment.
  if (utilitySegments.some((segment) => segment.replace(/\/+$/, '') === first)) return undefined;
  return joinPathComponent({
    pathname: stored.pathname,
    search: typeof stored.search === 'string' ? stored.search : '',
    hash: typeof stored.hash === 'string' ? stored.hash : '',
  });
};

/** The per-user `navToActivePath` store as persisted by `makeNavToActivePathAtom`. */
export const readStoredActivePaths = (userId: string): Record<string, Path> | undefined => {
  try {
    return getLocalStorageItem<Record<string, Path>>(`${NAV_TO_ACTIVE_PATH_PREFIX}${userId}`, {});
  } catch {
    return undefined;
  }
};
