import { invokeDesktopWithAvailability, isSynaraDesktop } from '../utils/desktop';

export type NativeMatrixSessionIdentity = {
  userId: string;
  deviceId: string;
  homeserverUrl: string;
};

let activeIdentity: NativeMatrixSessionIdentity | undefined;

export const getActiveNativeMatrixSession = (): NativeMatrixSessionIdentity | undefined =>
  activeIdentity;

export const setActiveNativeMatrixSession = (
  identity: NativeMatrixSessionIdentity
): NativeMatrixSessionIdentity => {
  activeIdentity = identity;
  return identity;
};

export const clearActiveNativeMatrixSession = () => {
  activeIdentity = undefined;
};

type NativeSessionInvoke = (
  command: string
) => Promise<{ available: boolean; value?: NativeMatrixSessionIdentity }>;

export const restoreNativeMatrixSessionWith = async (
  desktop: boolean,
  invoke: NativeSessionInvoke
): Promise<NativeMatrixSessionIdentity | undefined> => {
  if (!desktop) return undefined;
  try {
    const result = await invoke('matrix_restore_session');
    if (!result.available || !result.value) {
      clearActiveNativeMatrixSession();
      return undefined;
    }
    return setActiveNativeMatrixSession(result.value);
  } catch {
    clearActiveNativeMatrixSession();
    return undefined;
  }
};

export const restoreActiveNativeMatrixSession = async (): Promise<
  NativeMatrixSessionIdentity | undefined
> =>
  restoreNativeMatrixSessionWith(isSynaraDesktop(), (command) =>
    invokeDesktopWithAvailability<NativeMatrixSessionIdentity>(command)
  );

export const hasActiveNativeMatrixSession = (): boolean => Boolean(getActiveNativeMatrixSession());
