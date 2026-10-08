import { ReactEventHandler, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useRoomNavigate } from './useRoomNavigate';
import { isRoomId, isUserId } from '../utils/matrix';
import { getHomeRoomPath, withSearchParam } from '../pages/pathUtils';
import { _RoomSearchParams } from '../pages/paths';
import { useOpenUserRoomProfile } from '../state/hooks/userRoomProfile';
import { useSpaceOptionally } from './useSpace';

import { getNativeRoom } from '../native/nativeSession';
export const useMentionClickHandler = (roomId: string): ReactEventHandler<HTMLElement> => {
  const { navigateRoom, navigateSpace } = useRoomNavigate();
  const navigate = useNavigate();
  const openProfile = useOpenUserRoomProfile();
  const space = useSpaceOptionally();

  const handleClick: ReactEventHandler<HTMLElement> = useCallback(
    (evt) => {
      evt.stopPropagation();
      evt.preventDefault();
      const target = evt.currentTarget;
      const mentionId = target.getAttribute('data-mention-id');
      if (typeof mentionId !== 'string') return;

      if (isUserId(mentionId)) {
        openProfile(roomId, space?.roomId, mentionId, target.getBoundingClientRect());
        return;
      }

      const eventId = target.getAttribute('data-mention-event-id') || undefined;
      if (isRoomId(mentionId) && getNativeRoom(mentionId)) {
        if (getNativeRoom(mentionId)?.isSpaceRoom()) navigateSpace(mentionId);
        else navigateRoom(mentionId, eventId);
        return;
      }

      const viaServers = target.getAttribute('data-mention-via') || undefined;
      const path = getHomeRoomPath(mentionId, eventId);

      navigate(viaServers ? withSearchParam<_RoomSearchParams>(path, { viaServers }) : path);
    },
    [navigate, navigateRoom, navigateSpace, roomId, space, openProfile]
  );

  return handleClick;
};
