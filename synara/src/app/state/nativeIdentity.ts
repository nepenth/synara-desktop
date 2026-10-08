import { useSyncExternalStore } from 'react';

/**
 * The signed-in account as the last definitive native session snapshot
 * reported it. Components read their own user id here instead of through the
 * js-sdk-shaped client facade.
 */
export type NativeIdentity = {
  userId?: string;
  deviceId?: string;
  homeserverUrl?: string;
};

let identity: NativeIdentity = {};
const listeners = new Set<() => void>();

/** Record the active identity; `{}` clears it on logout. */
export const setNativeIdentity = (next: NativeIdentity): void => {
  if (
    next.userId === identity.userId &&
    next.deviceId === identity.deviceId &&
    next.homeserverUrl === identity.homeserverUrl
  ) {
    return;
  }
  identity = { ...next };
  listeners.forEach((listener) => listener());
};

export const getNativeIdentity = (): NativeIdentity => identity;

/** The signed-in user's Matrix id, or `undefined` when signed out. */
export const getMyUserId = (): string | undefined => identity.userId;

/** The signed-in user's Matrix id, or `''` when signed out. */
export const getSafeMyUserId = (): string => identity.userId ?? '';

const subscribe = (listener: () => void): (() => void) => {
  listeners.add(listener);
  return () => listeners.delete(listener);
};

/** React to identity changes (login, logout, account switch). */
export const useNativeIdentity = (): NativeIdentity =>
  useSyncExternalStore(subscribe, getNativeIdentity, getNativeIdentity);

/** The signed-in user's Matrix id for render, `''` when signed out. */
export const useMyUserId = (): string => useNativeIdentity().userId ?? '';
