import { invokeDesktopWithAvailability } from '../../utils/desktop';

export type NativeRoomKeyTransferKind = 'export' | 'import';
export type NativeRoomKeyTransferPhase =
  | 'idle'
  | 'preparing'
  | 'in_flight'
  | 'succeeded'
  | 'failed'
  | 'cancelled';

export type NativeRoomKeyTransferStatus = {
  sessionGeneration: number;
  kind?: NativeRoomKeyTransferKind;
  phase: NativeRoomKeyTransferPhase;
  progressPercent?: number;
  keysProcessed: number;
  roomsTouched: number;
  fileLabel?: string;
  failureDiagnosticId?: string;
};

export type NativeRoomKeyTransferResult = {
  outcome: 'complete';
  fileLabel: string;
  keysProcessed: number;
  roomsTouched: number;
  totalKeysFound?: number;
  status: NativeRoomKeyTransferStatus;
};

export type NativeRoomKeyFileSelection = {
  selectionId: number;
  fileLabel: string;
};

export const nativeRoomKeyErrorMessage = (): string =>
  'Native room-key transfer is unavailable. Restart Synara and try again.';

const invokeNativeRoomKeys = async <T>(
  command: string,
  args?: Record<string, unknown>,
  errorMessage = nativeRoomKeyErrorMessage()
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

export const getNativeRoomKeyTransferStatus = (): Promise<NativeRoomKeyTransferStatus> =>
  invokeNativeRoomKeys('matrix_room_key_transfer_status');

export const exportNativeRoomKeys = (passphrase: string): Promise<NativeRoomKeyTransferResult> =>
  invokeNativeRoomKeys(
    'matrix_room_key_export',
    { passphrase },
    'Room-key export failed. Check the passphrase and try again.'
  );

export const selectNativeRoomKeyImport = (): Promise<NativeRoomKeyFileSelection | null> =>
  invokeNativeRoomKeys('matrix_room_key_import_select');

export const importNativeRoomKeys = (
  selectionId: number,
  passphrase: string
): Promise<NativeRoomKeyTransferResult> =>
  invokeNativeRoomKeys(
    'matrix_room_key_import',
    { selectionId, passphrase },
    'Room-key import failed. Check the selected file and passphrase, then try again.'
  );
