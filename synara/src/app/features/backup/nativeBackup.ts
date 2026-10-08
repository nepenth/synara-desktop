import { invokeDesktopWithAvailability } from '../../utils/desktop';
import type {
  NativeBackupAction,
  NativeBackupAvailability,
  NativeBackupDeviceState,
  NativeBackupOperationResult,
  NativeBackupRecoveryState,
  NativeBackupStatus,
} from '../matrix-dto/generated';

export const NATIVE_BACKUP_CHANGED = 'synara-native-backup-changed';

export const nativeBackupErrorMessage = (): string =>
  'Native encryption backup is unavailable. Restart Synara and try again.';

const invokeNativeBackup = async <T>(
  command: string,
  args?: Record<string, unknown>,
  errorMessage = nativeBackupErrorMessage()
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
    window.dispatchEvent(new Event(NATIVE_BACKUP_CHANGED));
  }
};

export const getNativeBackupStatus = (): Promise<NativeBackupStatus> =>
  invokeNativeBackup('matrix_backup_status');

export const restoreNativeBackup = async (
  recoverySecret: string
): Promise<NativeBackupOperationResult> => {
  const result = await invokeNativeBackup<NativeBackupOperationResult>(
    'matrix_backup_restore',
    { recoverySecret },
    'Encryption backup restore failed. Check your recovery key or passphrase and try again.'
  );
  announceStatusChange();
  return result;
};

export const repairNativeBackup = async (
  recoverySecret: string
): Promise<NativeBackupOperationResult> => {
  const result = await invokeNativeBackup<NativeBackupOperationResult>(
    'matrix_backup_repair',
    { recoverySecret },
    'Encryption backup repair failed. Check your recovery key or passphrase and try again.'
  );
  announceStatusChange();
  return result;
};

export type {
  NativeBackupAction,
  NativeBackupAvailability,
  NativeBackupDeviceState,
  NativeBackupOperationResult,
  NativeBackupRecoveryState,
  NativeBackupStatus,
};
