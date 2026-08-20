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

export function useDeviceList(): [undefined | NativeDevice[], RefreshDeviceList] {
  const queryClient = useQueryClient();
  const sessionGeneration = getActiveSession()?.sessionGeneration;
  const queryKey = useMemo(
    () => ['native-devices', sessionGeneration] as const,
    [sessionGeneration],
  );
  const { data: snapshot, refetch } = useQuery({
    queryKey,
    queryFn: getNativeDeviceSnapshot,
    enabled: sessionGeneration !== undefined,
    staleTime: 0,
    gcTime: Infinity,
    refetchOnMount: 'always',
    refetchOnWindowFocus: 'always',
  });

  const refreshDeviceList = useCallback(
    async (authoritativeSnapshot?: NativeDeviceSnapshot) => {
      if (sessionGeneration === undefined) return;
      if (authoritativeSnapshot) {
        queryClient.setQueryData(queryKey, authoritativeSnapshot);
        return;
      }
      await refetch();
    },
    [queryClient, queryKey, refetch, sessionGeneration],
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
        }),
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

  return [snapshot?.devices, refreshDeviceList];
}

export const useSplitCurrentDevice = (
  devices: NativeDevice[] | undefined,
): [NativeDevice | undefined, NativeDevice[] | undefined] => {
  const currentDevice = useMemo(() => devices?.find((device) => device.isCurrent), [devices]);
  const otherDevices = useMemo(() => devices?.filter((device) => !device.isCurrent), [devices]);
  return [currentDevice, otherDevices];
};

export type { NativeDevice, NativeDeviceSnapshot };
