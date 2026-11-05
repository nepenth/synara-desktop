import { ReactNode } from 'react';
import { matchPath, useLocation, useMatch } from 'react-router-dom';
import { ScreenSize, useScreenSizeContext } from '../hooks/useScreenSize';
import { DIRECT_PATH, EXPLORE_PATH, HOME_PATH, INBOX_PATH, SPACE_PATH } from './paths';

type MobileFriendlyClientNavProps = {
  children: ReactNode;
};
export function MobileFriendlyClientNav({ children }: MobileFriendlyClientNavProps) {
  const screenSize = useScreenSizeContext();
  const homeMatch = useMatch({ path: HOME_PATH, caseSensitive: true, end: true });
  const directMatch = useMatch({ path: DIRECT_PATH, caseSensitive: true, end: true });
  const spaceMatch = useMatch({ path: SPACE_PATH, caseSensitive: true, end: true });
  const exploreMatch = useMatch({ path: EXPLORE_PATH, caseSensitive: true, end: true });
  const inboxMatch = useMatch({ path: INBOX_PATH, caseSensitive: true, end: true });

  if (
    screenSize === ScreenSize.Mobile &&
    !(homeMatch || directMatch || spaceMatch || exploreMatch || inboxMatch)
  ) {
    return null;
  }

  return children;
}

type MobileFriendlyPageNavProps = {
  /** The pane's base path(s); Home serves both `/home/` and `/direct/` (NAV-12). */
  path: string | readonly string[];
  children: ReactNode;
};
export function MobileFriendlyPageNav({ path, children }: MobileFriendlyPageNavProps) {
  const screenSize = useScreenSizeContext();
  const { pathname } = useLocation();
  const paths: readonly string[] = typeof path === 'string' ? [path] : path;
  const exactPath = paths.some(
    (base) => matchPath({ path: base, caseSensitive: true, end: true }, pathname) !== null
  );

  if (screenSize === ScreenSize.Mobile && !exactPath) {
    return null;
  }

  return children;
}
