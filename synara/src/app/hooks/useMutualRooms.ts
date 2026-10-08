import { useCallback } from 'react';
import { AsyncState, useAsyncCallbackValue } from './useAsyncCallback';
import { fetchMutualRooms } from '../native/nativeRoomExtras';

/**
 * Core answers mutual rooms from the local room store, so the query is always
 * available on the native client (no MSC2666 server support needed).
 */
export const useMutualRoomsSupport = (): boolean => true;

/** Joined rooms shared with `userId`. */
export const useMutualRooms = (userId: string): AsyncState<string[], unknown> => {
  const [mutualRoomsState] = useAsyncCallbackValue(
    useCallback(async (): Promise<string[]> => fetchMutualRooms(userId), [userId])
  );

  return mutualRoomsState;
};
