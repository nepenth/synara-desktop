import { useCallback, useSyncExternalStore } from 'react';

/**
 * In-room thread view owner shared by the native timeline presenter and the
 * composer. RoomView mounts those as siblings; this module is the slot the
 * composer uses to key reply drafts without lifting router state in T2.
 */
const threadRootByRoom = new Map<string, string>();
const listenersByRoom = new Map<string, Set<() => void>>();

const emit = (roomId: string): void => {
  listenersByRoom.get(roomId)?.forEach((listener) => listener());
};

export const publishNativeThreadRoot = (
  roomId: string,
  threadRootEventId: string | undefined
): void => {
  const previous = threadRootByRoom.get(roomId);
  if (previous === threadRootEventId) return;
  if (threadRootEventId) threadRootByRoom.set(roomId, threadRootEventId);
  else threadRootByRoom.delete(roomId);
  emit(roomId);
};

export const nativeThreadRootForRoom = (roomId: string): string | undefined =>
  threadRootByRoom.get(roomId);

export const useNativeThreadRoot = (roomId: string): string | undefined => {
  const subscribe = useCallback(
    (listener: () => void) => {
      const listeners = listenersByRoom.get(roomId) ?? new Set<() => void>();
      listeners.add(listener);
      listenersByRoom.set(roomId, listeners);
      return () => {
        listeners.delete(listener);
        if (listeners.size === 0) listenersByRoom.delete(roomId);
      };
    },
    [roomId]
  );
  const getSnapshot = useCallback(() => nativeThreadRootForRoom(roomId), [roomId]);
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
};
