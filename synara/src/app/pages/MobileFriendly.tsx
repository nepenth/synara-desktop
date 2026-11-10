import { ReactNode } from 'react';
import { matchPath, useLocation } from 'react-router-dom';
import { ScreenSize, useScreenSizeContext } from '../hooks/useScreenSize';

type MobileFriendlyPageNavProps = {
  /**
   * The pane's base path(s); Home serves both `/home/` and `/direct/` (NAV-12).
   * On Mobile the list (with its bottom bar) shows only on these exact paths;
   * deeper routes show the page alone with a Back button. `[]`: never.
   */
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
