import { invokeDesktopWithAvailability } from '../../utils/desktop';

export type NativeBackupAvailability = 'missing' | 'available';
export type NativeBackupDeviceState =
  | 'unavailable'
  | 'disconnected'
  | 'connecting'
  | 'downloading'
  | 'uploading'
  | 'ready';
export type NativeBackupRecoveryState = 'unknown' | 'not_set_up' | 'incomplete' | 'ready';
export type NativeBackupAction = 'setup_required' | 'restore_required' | 'repair_required' | 'none';

export type NativeBackupStatus = {
  sessionGeneration: number;
  availability: NativeBackupAvailability;
  enabled: boolean;
  version?: string;
  keyCount?: number;
  deviceState: NativeBackupDeviceState;
  recoveryState: NativeBackupRecoveryState;
  action: NativeBackupAction;
};

export type NativeBackupOperationResult = {
  outcome: 'complete' | 'already_configured';
  status: NativeBackupStatus;
};

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

export const setupNativeBackup = async (
  passphrase: string
): Promise<NativeBackupOperationResult> => {
  const result = await invokeNativeBackup<NativeBackupOperationResult>(
    'matrix_backup_setup',
    { passphrase },
    'Encryption backup setup failed. Check your recovery passphrase and try again.'
  );
  announceStatusChange();
  return result;
};

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
