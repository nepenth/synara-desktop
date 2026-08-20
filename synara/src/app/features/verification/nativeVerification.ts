import { getSessionBootstrapResult } from '../../state/sessionBootstrap';
import { invokeDesktopWithAvailability, isSynaraDesktop, listen } from '../../utils/desktop';

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

export const NATIVE_VERIFICATION_CHANGED = 'synara-native-verification-changed';
export const VERIFICATION_UPDATED_EVENT = 'matrix-verification-updated';

export const isNativeVerificationTerminal = (phase: NativeVerificationPhase): boolean =>
  phase === 'done' || phase === 'mismatched' || phase === 'cancelled';

export const selectNativeVerificationRequest = (
  requests: NativeVerificationRequest[],
  currentFlowId?: string
): NativeVerificationRequest | undefined => {
  if (currentFlowId) {
    const current = requests.find((item) => item.flowId === currentFlowId);
    if (current) return current;
  }
  return requests.find((item) => !isNativeVerificationTerminal(item.phase)) ?? requests[0];
};

export const verificationRequestHasSasCodes = (request: NativeVerificationRequest): boolean => {
  const emoji = request.sas?.emoji;
  if (Array.isArray(emoji) && emoji.length > 0) return true;
  const decimals = request.sas?.decimals;
  return Array.isArray(decimals) && decimals.length === 3;
};

export const verificationRequestNeedsSasStart = (request: NativeVerificationRequest): boolean =>
  (request.direction === 'outgoing' && request.phase === 'ready') ||
  (request.direction === 'incoming' && request.phase === 'started') ||
  (request.phase === 'sas_ready' && !verificationRequestHasSasCodes(request));

export const announceNativeVerificationChanged = (): void => {
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new Event(NATIVE_VERIFICATION_CHANGED));
  }
};

export const subscribeNativeVerificationUpdates = (onUpdate: () => void): (() => void) => {
  let disposed = false;
  let unlisten: (() => void) | undefined;
  const onWindow = (): void => {
    onUpdate();
  };
  if (typeof window !== 'undefined') {
    window.addEventListener(NATIVE_VERIFICATION_CHANGED, onWindow);
  }
  void listen<{ sessionGeneration: number }>(VERIFICATION_UPDATED_EVENT, () => {
    onUpdate();
  })
    .then((cleanup) => {
      if (disposed) {
        void cleanup?.();
        return;
      }
      unlisten = () => {
        void cleanup?.();
      };
    })
    .catch(() => undefined);
  return () => {
    disposed = true;
    if (typeof window !== 'undefined') {
      window.removeEventListener(NATIVE_VERIFICATION_CHANGED, onWindow);
    }
    unlisten?.();
  };
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

const mutateNativeVerification = async <T>(
  command: string,
  args?: Record<string, unknown>
): Promise<T> => {
  const value = await invokeNativeVerification<T>(command, args);
  announceNativeVerificationChanged();
  return value;
};

export const startNativeVerification = (deviceId?: string): Promise<NativeVerificationRequest> =>
  mutateNativeVerification('matrix_verification_start', { deviceId });

export const acceptNativeVerification = (flowId: string): Promise<NativeVerificationRequest> =>
  mutateNativeVerification('matrix_verification_accept', { flowId });

export const beginNativeVerificationSas = (flowId: string): Promise<NativeVerificationRequest> =>
  mutateNativeVerification('matrix_verification_begin_sas', { flowId });

export const confirmNativeVerification = (flowId: string): Promise<NativeVerificationRequest> =>
  mutateNativeVerification('matrix_verification_confirm', { flowId });

export const mismatchNativeVerification = (flowId: string): Promise<NativeVerificationRequest> =>
  mutateNativeVerification('matrix_verification_mismatch', { flowId });

export const cancelNativeVerification = (flowId: string): Promise<NativeVerificationRequest> =>
  mutateNativeVerification('matrix_verification_cancel', { flowId });

export const dismissNativeVerification = (flowId: string): Promise<void> =>
  mutateNativeVerification('matrix_verification_dismiss', { flowId });

export const getNativeCryptoStatus = (): Promise<NativeCryptoStatus> =>
  invokeNativeVerification('matrix_crypto_status');
