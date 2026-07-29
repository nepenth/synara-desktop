import { useCallback, useEffect, useMemo } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  getNativeDeviceSnapshot,
  NativeDevice,
  NativeDeviceSnapshot,
} from '../features/settings/devices/nativeDevices';

const DEVICES_QUERY_KEY = ['native-devices'];
const DEVICE_LIST_UPDATED_EVENT = 'matrix-device-list-updated';

export type RefreshDeviceList = (snapshot?: NativeDeviceSnapshot) => Promise<void>;

export function useDeviceList(): [undefined | NativeDevice[], RefreshDeviceList] {
  const queryClient = useQueryClient();
  const { data: snapshot, refetch } = useQuery({
    queryKey: DEVICES_QUERY_KEY,
    queryFn: getNativeDeviceSnapshot,
    staleTime: 0,
    gcTime: Infinity,
    refetchOnMount: 'always',
    refetchOnWindowFocus: 'always',
  });

  const refreshDeviceList = useCallback(
    async (authoritativeSnapshot?: NativeDeviceSnapshot) => {
      if (authoritativeSnapshot) {
        queryClient.setQueryData(DEVICES_QUERY_KEY, authoritativeSnapshot);
        return;
      }
      await refetch();
    },
    [queryClient, refetch]
  );

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
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
    };
  }, [refreshDeviceList, snapshot]);

  return [snapshot?.devices, refreshDeviceList];
}

export const useSplitCurrentDevice = (
  devices: NativeDevice[] | undefined
): [NativeDevice | undefined, NativeDevice[] | undefined] => {
  const currentDevice = useMemo(() => devices?.find((device) => device.isCurrent), [devices]);
  const otherDevices = useMemo(() => devices?.filter((device) => !device.isCurrent), [devices]);
  return [currentDevice, otherDevices];
};

export type { NativeDevice, NativeDeviceSnapshot };
