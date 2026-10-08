import { useCallback } from 'react';
import { AsyncState, useAsyncCallbackValue } from './useAsyncCallback';
import { useSpecVersions } from './useSpecVersions';

export const useMutualRoomsSupport = (): boolean => {
  const { unstable_features: unstableFeatures } = useSpecVersions();

  const supported =
    unstableFeatures?.['uk.half-shot.msc2666'] ||
    unstableFeatures?.['uk.half-shot.msc2666.mutual_rooms'] ||
    unstableFeatures?.['uk.half-shot.msc2666.query_mutual_rooms'];

  return !!supported;
};

/**
 * Rooms shared with another user. Native has no mutual-rooms (MSC2666)
 * command, so the list is always empty.
 */
export const useMutualRooms = (): AsyncState<string[], unknown> => {
  const [mutualRoomsState] = useAsyncCallbackValue(
    useCallback(async (): Promise<string[]> => [], [])
  );

  return mutualRoomsState;
};
