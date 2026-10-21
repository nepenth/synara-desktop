import { useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useRoomNavigate } from '../../hooks/useRoomNavigate';
import { useSpaceOptionally } from '../../hooks/useSpace';
import { useOpenUserRoomProfile } from '../../state/hooks/userRoomProfile';
import { getNativeRoom } from '../../native/nativeSession';
import { isRoomId } from '../../utils/matrix';
import { getHomeRoomPath, withSearchParam } from '../../pages/pathUtils';
import type { _RoomSearchParams } from '../../pages/paths';
import type { NativePermalinkHandler } from './nativeTimelineFormattedBody';

/**
 * In-app navigation for matrix.to links in timeline messages: a user opens
 * their profile, a joined room or event opens here, anything else opens the
 * room preview. Mirrors `useMentionClickHandler` for the legacy renderers.
 */
export const useNativePermalinkHandler = (roomId: string): NativePermalinkHandler => {
  const { navigateRoom, navigateSpace } = useRoomNavigate();
  const navigate = useNavigate();
  const openProfile = useOpenUserRoomProfile();
  const space = useSpaceOptionally();

  return useCallback<NativePermalinkHandler>(
    (target, anchor) => {
      if (target.kind === 'user') {
        openProfile(roomId, space?.roomId, target.userId, anchor.getBoundingClientRect());
        return;
      }
      const { roomIdOrAlias, eventId, viaServers } = target;
      const joined = isRoomId(roomIdOrAlias) ? getNativeRoom(roomIdOrAlias) : undefined;
      if (joined) {
        if (joined.isSpaceRoom()) navigateSpace(roomIdOrAlias);
        else navigateRoom(roomIdOrAlias, eventId);
        return;
      }
      const path = getHomeRoomPath(roomIdOrAlias, eventId);
      navigate(
        viaServers?.length
          ? withSearchParam<_RoomSearchParams>(path, { viaServers: viaServers.join(',') })
          : path
      );
    },
    [navigate, navigateRoom, navigateSpace, openProfile, roomId, space]
  );
};
