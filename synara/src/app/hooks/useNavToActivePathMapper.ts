import { useEffect } from 'react';
import { useSetAtom } from 'jotai';
import { useLocation } from 'react-router-dom';
import { useNavToActivePathAtom } from '../state/hooks/navToActivePath';

/** Record the current route as `navId`'s last place; `enabled` gates it per route. */
export const useNavToActivePathMapper = (navId: string, enabled = true) => {
  const location = useLocation();
  const setNavToActivePath = useSetAtom(useNavToActivePathAtom());

  useEffect(() => {
    if (!enabled) return;
    const { pathname, search, hash } = location;
    setNavToActivePath({
      type: 'PUT',
      navId,
      path: { pathname, search, hash },
    });
  }, [location, setNavToActivePath, navId, enabled]);
};
