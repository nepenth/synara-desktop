import { useCallback, useEffect, useMemo } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  getNativeDeviceSnapshot,
  NativeDevice,
  NativeDeviceSnapshot,
} from '../features/settings/devices/nativeDevices';
import { subscribeNativeVerificationUpdates } from '../features/verification/nativeVerification';
import { getActiveSession } from '../state/sessionBootstrap';

const DEVICE_LIST_UPDATED_EVENT = 'matrix-device-list-updated';

export type RefreshDeviceList = (snapshot?: NativeDeviceSnapshot) => Promise<void>;

export type DeviceListLoadState = {
  /** True while the first snapshot or a refetch is in flight. */
  fetching: boolean;
  /** Set when the most recent snapshot request was rejected. */
  error?: string;
};

const describeDeviceListError = (error: unknown): string => {
  if (error instanceof Error && error.message) return error.message;
  if (typeof error === 'object' && error !== null && 'message' in error) {
    const { message } = error as { message?: unknown };
    if (typeof message === 'string' && message) return message;
  }
  return 'Native Matrix device management is unavailable.';
};

export function useDeviceList(): [
  undefined | NativeDeviceSnapshot,
  RefreshDeviceList,
  DeviceListLoadState
] {
  const queryClient = useQueryClient();
  // The bootstrap `Session` only carries account identity; the authoritative
  // numeric generation lives on the native snapshot itself. Gate on the
  // presence of a session and key the cache on the identity so a re-login
  // under a different account never reads a stale list.
  const session = getActiveSession();
  const sessionKey = session ? `${session.userId}\u0000${session.deviceId}` : undefined;
  const queryKey = useMemo(() => ['native-devices', sessionKey] as const, [sessionKey]);
  const {
    data: snapshot,
    error,
    isFetching,
    refetch,
  } = useQuery({
    queryKey,
    queryFn: getNativeDeviceSnapshot,
    enabled: sessionKey !== undefined,
    staleTime: 0,
    gcTime: Infinity,
    refetchOnMount: 'always',
    refetchOnWindowFocus: 'always',
    retry: false,
  });

  const refreshDeviceList = useCallback(
    async (authoritativeSnapshot?: NativeDeviceSnapshot) => {
      if (sessionKey === undefined) return;
      if (authoritativeSnapshot) {
        queryClient.setQueryData(queryKey, authoritativeSnapshot);
        return;
      }
      await refetch();
    },
    [queryClient, queryKey, refetch, sessionKey]
  );

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    const unsubscribeVerification = subscribeNativeVerificationUpdates(() => {
      void refreshDeviceList();
    });
    void import('@tauri-apps/api/event')
      .then(({ listen }) =>
        listen<{ sessionGeneration: number }>(DEVICE_LIST_UPDATED_EVENT, (event) => {
          if (
            snapshot === undefined ||
            event.payload.sessionGeneration === snapshot.sessionGeneration
          ) {
            void refreshDeviceList();
          }
        })
      )
      .then((cleanup) => {
        if (disposed) cleanup();
        else unlisten = cleanup;
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
      unlisten?.();
      unsubscribeVerification();
    };
  }, [refreshDeviceList, snapshot]);

  const loadState = useMemo<DeviceListLoadState>(
    () => ({
      fetching: isFetching,
      error: error ? describeDeviceListError(error) : undefined,
    }),
    [error, isFetching]
  );

  return [snapshot, refreshDeviceList, loadState];
}

export const useSplitCurrentDevice = (
  devices: NativeDevice[] | undefined
): [NativeDevice | undefined, NativeDevice[] | undefined] => {
  const currentDevice = useMemo(() => devices?.find((device) => device.isCurrent), [devices]);
  const otherDevices = useMemo(() => devices?.filter((device) => !device.isCurrent), [devices]);
  return [currentDevice, otherDevices];
};

export type { NativeDevice, NativeDeviceSnapshot };
