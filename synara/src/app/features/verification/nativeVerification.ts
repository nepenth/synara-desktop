import { getSessionBootstrapResult } from '../../state/sessionBootstrap';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';

export type NativeVerificationDirection = 'incoming' | 'outgoing';
export type NativeVerificationPhase =
  | 'requested'
  | 'ready'
  | 'started'
  | 'sas_ready'
  | 'confirmed'
  | 'done'
  | 'mismatched'
  | 'cancelled';

export type NativeVerificationEmoji = {
  symbol: string;
  description: string;
};

export type NativeVerificationSas = {
  emoji?: NativeVerificationEmoji[];
  decimals?: [number, number, number];
};

export type NativeVerificationRequest = {
  flowId: string;
  otherUserId: string;
  otherDeviceId?: string;
  direction: NativeVerificationDirection;
  phase: NativeVerificationPhase;
  startedTs?: number;
  sas?: NativeVerificationSas;
};

export type NativeVerificationInbox = {
  sessionGeneration: number;
  requests: NativeVerificationRequest[];
};

export type NativeCryptoStatus = {
  sessionGeneration: number;
  encryptionEnabled: boolean;
  crossSigningState: 'unavailable' | 'not_set_up' | 'partial' | 'ready';
};

export const isNativeMatrixSession = (): boolean =>
  isSynaraDesktop() && getSessionBootstrapResult().source === 'native';

export const verificationRequestNeedsSasStart = (request: NativeVerificationRequest): boolean =>
  (request.direction === 'outgoing' && request.phase === 'ready') ||
  (request.direction === 'incoming' && request.phase === 'started');

export const verificationRequestHasSasCodes = (request: NativeVerificationRequest): boolean => {
  const emoji = request.sas?.emoji;
  if (Array.isArray(emoji) && emoji.length > 0) return true;
  const decimals = request.sas?.decimals;
  return Array.isArray(decimals) && decimals.length === 3;
};

export const nativeVerificationErrorMessage = (): string =>
  'Native device verification is unavailable. Restart Synara or try again from a connected device.';

const invokeNativeVerification = async <T>(
  command: string,
  args?: Record<string, unknown>
): Promise<T> => {
  const result = await invokeDesktopWithAvailability<T>(command, args);
  if (!result.available || result.value === undefined) {
    throw new Error(nativeVerificationErrorMessage());
  }
  return result.value;
};

export const listNativeVerificationRequests = (): Promise<NativeVerificationInbox> =>
  invokeNativeVerification('matrix_verification_list');

export const startNativeVerification = (deviceId?: string): Promise<NativeVerificationRequest> =>
  invokeNativeVerification('matrix_verification_start', { deviceId });

export const acceptNativeVerification = (flowId: string): Promise<NativeVerificationRequest> =>
  invokeNativeVerification('matrix_verification_accept', { flowId });

export const beginNativeVerificationSas = (flowId: string): Promise<NativeVerificationRequest> =>
  invokeNativeVerification('matrix_verification_begin_sas', { flowId });

export const confirmNativeVerification = (flowId: string): Promise<NativeVerificationRequest> =>
  invokeNativeVerification('matrix_verification_confirm', { flowId });

export const mismatchNativeVerification = (flowId: string): Promise<NativeVerificationRequest> =>
  invokeNativeVerification('matrix_verification_mismatch', { flowId });

export const cancelNativeVerification = (flowId: string): Promise<NativeVerificationRequest> =>
  invokeNativeVerification('matrix_verification_cancel', { flowId });

export const dismissNativeVerification = (flowId: string): Promise<void> =>
  invokeNativeVerification('matrix_verification_dismiss', { flowId });

export const getNativeCryptoStatus = (): Promise<NativeCryptoStatus> =>
  invokeNativeVerification('matrix_crypto_status');
