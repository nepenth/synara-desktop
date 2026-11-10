import { createContext, useContext } from 'react';
import { NavToActivePathAtom } from '../navToActivePath';

const NavToActivePathAtomContext = createContext<NavToActivePathAtom | null>(null);
export const NavToActivePathProvider = NavToActivePathAtomContext.Provider;

export const useNavToActivePathAtom = (): NavToActivePathAtom => {
  const anAtom = useContext(NavToActivePathAtomContext);

  if (!anAtom) {
    throw new Error('NavToActivePathAtom is not provided!');
  }

  return anAtom;
};

/** For code that also runs where no client is mounted (test harnesses). */
export const useOptionalNavToActivePathAtom = (): NavToActivePathAtom | null =>
  useContext(NavToActivePathAtomContext);
