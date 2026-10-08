import { invokeDesktopWithAvailability } from '../../utils/desktop';
import type {
  NativeSecretStorageOperationResult,
  NativeSecretStorageStatus,
} from '../matrix-dto/generated';
export type {
  NativeMissingSecret,
  NativeSecretStorageAction,
  NativeSecretStorageOperationResult,
  NativeSecretStorageState,
  NativeSecretStorageStatus,
} from '../matrix-dto/generated';

/**
 * Bootstrap and reset return Core's operation result plus, once, the
 * generated recovery key (the desktop shell's `DesktopSecretStorageSetup`).
 * The key is never part of Core's wire DTO.
 */
export type NativeSecretStorageSetup = NativeSecretStorageOperationResult & {
  recoveryKey?: string;
};

export const NATIVE_SECRET_STORAGE_CHANGED = 'synara-native-secret-storage-changed';

export const nativeSecretStorageErrorMessage = (): string =>
  'Native secret storage is unavailable. Restart Synara and try again.';

const invokeNativeSecretStorage = async <T>(
  command: string,
  args?: Record<string, unknown>,
  errorMessage = nativeSecretStorageErrorMessage()
): Promise<T> => {
  try {
    const result = await invokeDesktopWithAvailability<T>(command, args);
    if (!result.available || result.value === undefined) {
      throw new Error(errorMessage);
    }
    return result.value;
  } catch {
    throw new Error(errorMessage);
  }
};

const announceStatusChange = (): void => {
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new Event(NATIVE_SECRET_STORAGE_CHANGED));
  }
};

export const getNativeSecretStorageStatus = (): Promise<NativeSecretStorageStatus> =>
  invokeNativeSecretStorage('matrix_secret_storage_status');

export const bootstrapNativeSecretStorage = async (
  passphrase: string
): Promise<NativeSecretStorageSetup> => {
  const result = await invokeNativeSecretStorage<NativeSecretStorageSetup>(
    'matrix_secret_storage_bootstrap',
    { passphrase },
    'Secret storage setup failed. Check encryption backup status and try again.'
  );
  announceStatusChange();
  return result;
};

export const unlockNativeSecretStorage = async (
  recoverySecret: string
): Promise<NativeSecretStorageOperationResult> => {
  const result = await invokeNativeSecretStorage<NativeSecretStorageOperationResult>(
    'matrix_secret_storage_unlock',
    { recoverySecret },
    'Secret storage unlock failed. Check your recovery key or passphrase and try again.'
  );
  announceStatusChange();
  return result;
};

export const resetNativeSecretStorage = async (
  passphrase: string
): Promise<NativeSecretStorageSetup> => {
  const result = await invokeNativeSecretStorage<NativeSecretStorageSetup>(
    'matrix_secret_storage_reset',
    { passphrase },
    'Secret storage reset failed. Unlock secret storage and try again.'
  );
  announceStatusChange();
  return result;
};
