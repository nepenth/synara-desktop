import { useCallback, useEffect } from 'react';
import {
  getRoomDirectoryVisibilityNative,
  setRoomDirectoryVisibilityNative,
} from '../features/common-settings/general/nativeRoomProfile';
import { useAsyncCallback } from './useAsyncCallback';

export const useRoomDirectoryVisibility = (roomId: string) => {
  const [visibilityState, loadVisibility] = useAsyncCallback(
    useCallback(async () => {
      const result = await getRoomDirectoryVisibilityNative(roomId);
      return result.visibility === 'public';
    }, [roomId])
  );

  useEffect(() => {
    loadVisibility();
  }, [loadVisibility]);

  const setVisibility = useCallback(
    async (visibility: boolean) => {
      await setRoomDirectoryVisibilityNative(roomId, visibility ? 'public' : 'private');
      // The write acknowledgement is not authoritative. Reload through the
      // native owner so the UI only displays the homeserver's value.
      await loadVisibility();
    },
    [roomId, loadVisibility]
  );

  return {
    visibilityState,
    setVisibility,
  };
};
