import { useCallback, useState } from 'react';
import { markRoomsAsReadFromExplicitUserAction } from '../utils/notifications';
import { getNativeRoom } from '../native/nativeSession';
import { useAlive } from './useAlive';

/**
 * "Mark as Read" for a group of rooms from a menu. `markRead` resolves true
 * when every room was marked, so the menu closes only on success; otherwise
 * `markReadError` names the rooms that are still unread.
 */
export const useMarkRoomsAsRead = () => {
  const alive = useAlive();
  const [markingRead, setMarkingRead] = useState(false);
  const [markReadError, setMarkReadError] = useState<string>();

  const markRead = useCallback(
    async (roomIds: readonly string[]): Promise<boolean> => {
      setMarkingRead(true);
      setMarkReadError(undefined);
      try {
        const failure = await markRoomsAsReadFromExplicitUserAction(
          roomIds,
          (roomId) => getNativeRoom(roomId)?.name ?? roomId
        );
        if (alive()) setMarkReadError(failure);
        return failure === undefined;
      } finally {
        if (alive()) setMarkingRead(false);
      }
    },
    [alive]
  );

  return { markRead, markingRead, markReadError };
};
