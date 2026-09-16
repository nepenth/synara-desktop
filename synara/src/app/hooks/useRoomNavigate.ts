import { useCallback, useTransition } from 'react';
import { NavigateOptions, useLocation, useNavigate } from 'react-router-dom';
import { useAtomValue } from 'jotai';
import { getCanonicalAliasOrRoomId, getCanonicalAliasRoomId, isRoomAlias } from '../utils/matrix';
import {
  getDirectRoomPath,
  getDirectRoomThreadPath,
  getHomeRoomPath,
  getHomeRoomThreadPath,
  getSpacePath,
  getSpaceRoomPath,
  getSpaceRoomThreadPath,
} from '../pages/pathUtils';
import { useMatrixClient } from './useMatrixClient';
import { getOrphanParents, guessPerfectParent } from '../utils/room';
import { roomToParentsAtom } from '../state/room/roomToParents';
import { mDirectAtom } from '../state/mDirectList';
import { useSelectedSpace } from './router/useSelectedSpace';
import { settingsAtom } from '../state/settings';
import { useSetting } from '../state/hooks/settings';
import { getApprovalsOriginSpace } from '../routes/approvalsNavigation';
import { parseSynaraRouteDestination } from '../routes/synaraRoutes';

export const useRoomNavigate = () => {
  const navigate = useNavigate();
  const [, startTransition] = useTransition();
  const mx = useMatrixClient();
  const roomToParents = useAtomValue(roomToParentsAtom);
  const mDirects = useAtomValue(mDirectAtom);
  const selectedSpace = useSelectedSpace();
  const location = useLocation();
  const originSpace =
    parseSynaraRouteDestination(location.pathname)?.kind === 'approvals'
      ? getApprovalsOriginSpace(location.state)
      : undefined;
  const spaceSelectedId =
    selectedSpace ??
    (originSpace && isRoomAlias(originSpace)
      ? getCanonicalAliasRoomId(mx, originSpace)
      : originSpace);
  const [developerTools] = useSetting(settingsAtom, 'developerTools');

  const navigateSpace = useCallback(
    (roomId: string) => {
      const roomIdOrAlias = getCanonicalAliasOrRoomId(mx, roomId);
      startTransition(() => navigate(getSpacePath(roomIdOrAlias)));
    },
    [mx, navigate, startTransition]
  );

  const navigateRoom = useCallback(
    (roomId: string, eventId?: string, opts?: NavigateOptions) => {
      const roomIdOrAlias = getCanonicalAliasOrRoomId(mx, roomId);
      const openSpaceTimeline = developerTools && spaceSelectedId === roomId;

      const orphanParents = openSpaceTimeline ? [roomId] : getOrphanParents(roomToParents, roomId);
      if (orphanParents.length > 0) {
        let parentSpace: string;
        if (spaceSelectedId && orphanParents.includes(spaceSelectedId)) {
          parentSpace = spaceSelectedId;
        } else {
          parentSpace = guessPerfectParent(mx, roomId, orphanParents) ?? orphanParents[0];
        }

        const pSpaceIdOrAlias = getCanonicalAliasOrRoomId(mx, parentSpace);

        startTransition(() =>
          navigate(
            getSpaceRoomPath(pSpaceIdOrAlias, openSpaceTimeline ? roomId : roomIdOrAlias, eventId),
            opts
          )
        );
        return;
      }

      if (mDirects.has(roomId)) {
        startTransition(() => navigate(getDirectRoomPath(roomIdOrAlias, eventId), opts));
        return;
      }

      startTransition(() => navigate(getHomeRoomPath(roomIdOrAlias, eventId), opts));
    },
    [mx, navigate, startTransition, spaceSelectedId, roomToParents, mDirects, developerTools]
  );

  const navigateThread = useCallback(
    (roomId: string, threadRootId: string, opts?: NavigateOptions) => {
      const roomIdOrAlias = getCanonicalAliasOrRoomId(mx, roomId);
      const openSpaceTimeline = developerTools && spaceSelectedId === roomId;
      const orphanParents = openSpaceTimeline ? [roomId] : getOrphanParents(roomToParents, roomId);
      if (orphanParents.length > 0) {
        let parentSpace: string;
        if (spaceSelectedId && orphanParents.includes(spaceSelectedId)) {
          parentSpace = spaceSelectedId;
        } else {
          parentSpace = guessPerfectParent(mx, roomId, orphanParents) ?? orphanParents[0];
        }
        const pSpaceIdOrAlias = getCanonicalAliasOrRoomId(mx, parentSpace);
        startTransition(() =>
          navigate(
            getSpaceRoomThreadPath(
              pSpaceIdOrAlias,
              openSpaceTimeline ? roomId : roomIdOrAlias,
              threadRootId
            ),
            opts
          )
        );
        return;
      }
      if (mDirects.has(roomId)) {
        startTransition(() => navigate(getDirectRoomThreadPath(roomIdOrAlias, threadRootId), opts));
        return;
      }
      startTransition(() => navigate(getHomeRoomThreadPath(roomIdOrAlias, threadRootId), opts));
    },
    [mx, navigate, startTransition, spaceSelectedId, roomToParents, mDirects, developerTools]
  );

  return {
    navigateSpace,
    navigateRoom,
    navigateThread,
  };
};
