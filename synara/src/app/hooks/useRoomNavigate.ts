import { useCallback, useLayoutEffect, useMemo, useRef, useTransition } from 'react';
import { NavigateOptions, useLocation, useNavigate } from 'react-router-dom';
import { useAtomValue } from 'jotai';
import { getCanonicalAliasOrRoomId, getCanonicalAliasRoomId, isRoomAlias } from '../utils/matrix';
import { getSpacePath } from '../pages/pathUtils';
import { guessPerfectParent } from '../utils/room';
import { roomToParentsAtom } from '../state/room/roomToParents';
import { useNativeNavigationScope } from '../state/hooks/navigationUnread';
import { useSelectedSpace } from './router/useSelectedSpace';
import { settingsAtom } from '../state/settings';
import { useSetting } from '../state/hooks/settings';
import { getApprovalsOriginSpace } from '../routes/approvalsNavigation';
import { parseSynaraRouteDestination } from '../routes/synaraRoutes';
import { resolveRoomRoutePath, type RoomRouteScope, type RoomRouteTarget } from './roomRoutePath';

export const useRoomNavigate = () => {
  const navigate = useNavigate();
  const [, startTransition] = useTransition();
  const roomToParents = useAtomValue(roomToParentsAtom);
  const { roomIds: directRoomIds } = useNativeNavigationScope('direct');
  const selectedSpace = useSelectedSpace();
  const location = useLocation();
  const originSpace =
    parseSynaraRouteDestination(location.pathname)?.kind === 'approvals'
      ? getApprovalsOriginSpace(location.state)
      : undefined;
  const spaceSelectedId =
    selectedSpace ??
    (originSpace && isRoomAlias(originSpace) ? getCanonicalAliasRoomId(originSpace) : originSpace);
  const [developerTools] = useSetting(settingsAtom, 'developerTools');

  const directRoomIdSet = useMemo(() => new Set(directRoomIds), [directRoomIds]);
  const scope = useMemo<RoomRouteScope>(
    () => ({
      roomToParents,
      directRoomIds: directRoomIdSet,
      spaceSelectedId,
      developerTools,
      pickParent: guessPerfectParent,
      toIdOrAlias: getCanonicalAliasOrRoomId,
    }),
    [roomToParents, directRoomIdSet, spaceSelectedId, developerTools]
  );
  // Every room-list snapshot builds a new direct-room list. The callbacks read
  // the latest scope from a ref so their identity stays stable, and listeners
  // that depend on them are not re-registered on each room-list update.
  const scopeRef = useRef(scope);
  useLayoutEffect(() => {
    scopeRef.current = scope;
  }, [scope]);

  const roomPath = useCallback(
    (roomId: string, target: RoomRouteTarget = {}) =>
      resolveRoomRoutePath(roomId, target, scopeRef.current),
    []
  );

  const navigateSpace = useCallback(
    (roomId: string) => {
      const roomIdOrAlias = getCanonicalAliasOrRoomId(roomId);
      startTransition(() => navigate(getSpacePath(roomIdOrAlias)));
    },
    [navigate, startTransition]
  );

  const navigateRoom = useCallback(
    (roomId: string, eventId?: string, opts?: NavigateOptions) => {
      const path = roomPath(roomId, { eventId });
      startTransition(() => navigate(path, opts));
    },
    [navigate, startTransition, roomPath]
  );

  const navigateThread = useCallback(
    (roomId: string, threadRootId: string, opts?: NavigateOptions) => {
      const path = roomPath(roomId, { threadRootId });
      startTransition(() => navigate(path, opts));
    },
    [navigate, startTransition, roomPath]
  );

  return {
    navigateSpace,
    navigateRoom,
    navigateThread,
    roomPath,
  };
};
