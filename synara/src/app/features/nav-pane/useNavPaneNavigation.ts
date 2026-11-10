import { useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAtomValue } from 'jotai';
import { ScreenSize, useScreenSizeContext } from '../../hooks/useScreenSize';
import { useNavToActivePathAtom } from '../../state/hooks/navToActivePath';
import {
  getHomePath,
  getInboxInvitesPath,
  getInboxLaterPath,
  getInboxNotificationsPath,
  getInboxPath,
  getSpaceLobbyPath,
  getSpacePath,
  joinPathComponent,
} from '../../pages/pathUtils';
import { getCanonicalAliasOrRoomId } from '../../utils/matrix';
import { activityLandingPage } from './navBarBadges';

/**
 * Where Home, a space and Activity open: the scope's last place on desktop
 * (the former rail's rule), the list itself on Mobile.
 */
export const useNavPaneNavigation = () => {
  const navigate = useNavigate();
  const screenSize = useScreenSizeContext();
  const navToActivePath = useAtomValue(useNavToActivePathAtom());
  const mobile = screenSize === ScreenSize.Mobile;

  const openHome = useCallback(() => {
    const activePath = navToActivePath.get('home');
    if (activePath && !mobile) {
      navigate(joinPathComponent(activePath));
      return;
    }
    navigate(getHomePath());
  }, [navigate, navToActivePath, mobile]);

  const openSpace = useCallback(
    (spaceId: string) => {
      const spacePath = getSpacePath(getCanonicalAliasOrRoomId(spaceId));
      if (mobile) {
        navigate(spacePath);
        return;
      }
      const activePath = navToActivePath.get(spaceId);
      if (activePath && activePath.pathname.startsWith(spacePath)) {
        navigate(joinPathComponent(activePath));
        return;
      }
      navigate(getSpaceLobbyPath(getCanonicalAliasOrRoomId(spaceId)));
    },
    [navigate, navToActivePath, mobile]
  );

  const openActivity = useCallback(
    (inviteCount: number, laterActive: number) => {
      if (mobile) {
        navigate(getInboxPath());
        return;
      }
      const activePath = navToActivePath.get('inbox');
      if (activePath) {
        navigate(joinPathComponent(activePath));
        return;
      }
      const page = activityLandingPage(inviteCount, laterActive);
      navigate(
        page === 'invites'
          ? getInboxInvitesPath()
          : page === 'later'
            ? getInboxLaterPath()
            : getInboxNotificationsPath()
      );
    },
    [navigate, navToActivePath, mobile]
  );

  return { openHome, openSpace, openActivity };
};
