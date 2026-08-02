import { useCallback, useEffect } from 'react';
import {
  getRoomDirectoryVisibilityNative,
  setRoomDirectoryVisibilityNative,
} from '../features/common-settings/general/nativeRoomProfile';
import { getActiveSession } from '../state/sessionBootstrap';
import { AsyncStatus, type AsyncState, useAsyncCallback } from './useAsyncCallback';

type VisibilityRead = {
  sessionGeneration?: string;
  nativeSessionGeneration: number;
  isPublic: boolean;
};

export const useRoomDirectoryVisibility = (roomId: string) => {
  // The native generation is authoritative for the read, while this stable
  // session identity gives React a generation key for clearing mounted UI
  // state when the native session is replaced.
  const sessionGeneration = getActiveSession()?.sessionGeneration;
  const [loadedVisibilityState, loadVisibility] = useAsyncCallback(
    useCallback(async () => {
      const result = await getRoomDirectoryVisibilityNative(roomId);
      return {
        sessionGeneration,
        nativeSessionGeneration: result.sessionGeneration,
        isPublic: result.visibility === 'public',
      } satisfies VisibilityRead;
    }, [roomId, sessionGeneration])
  );

  useEffect(() => {
    loadVisibility();
  }, [loadVisibility]);

  const visibilityState: AsyncState<boolean> =
    loadedVisibilityState.status === AsyncStatus.Success
      ? loadedVisibilityState.data.sessionGeneration === sessionGeneration
        ? { status: AsyncStatus.Success, data: loadedVisibilityState.data.isPublic }
        : { status: AsyncStatus.Loading }
      : loadedVisibilityState;

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
