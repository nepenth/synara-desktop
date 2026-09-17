import { getSessionBootstrapResult } from '../../state/sessionBootstrap';
import { invokeDesktopWithAvailability, isSynaraDesktop, listen } from '../../utils/desktop';

export type NativeVerificationDirection = 'incoming' | 'outgoing';
export type NativeVerificationPhase =
  | 'requested'
  | 'ready'
  | 'started'
  | 'keys_exchanging'
  | 'sas_ready'
  | 'confirmed'
  | 'done'
  | 'mismatched'
  | 'cancelled'
  | 'failed';

export type NativeVerificationEmoji = {
  symbol: string;
  description: string;
};

export type NativeVerificationSas = {
  emoji?: NativeVerificationEmoji[];
  decimals?: [number, number, number];
};

export type NativeVerificationQr = {
  imageDataUrl: string;
  scanned: boolean;
};

export type NativeVerificationRequest = {
  flowId: string;
  otherUserId: string;
  otherDeviceId?: string;
  direction: NativeVerificationDirection;
  phase: NativeVerificationPhase;
  startedTs?: number;
  sas?: NativeVerificationSas;
  qr?: NativeVerificationQr;
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
  phase === 'done' || phase === 'mismatched' || phase === 'cancelled' || phase === 'failed';

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

export const verificationRequestHasQr = (request: NativeVerificationRequest): boolean =>
  typeof request.qr?.imageDataUrl === 'string' && request.qr.imageDataUrl.startsWith('data:image/');

/** Skip SAS auto-start only when a renderable QR is actually shown. */
export const verificationRequestNeedsSasStart = (request: NativeVerificationRequest): boolean =>
  request.direction === 'outgoing' &&
  request.phase === 'ready' &&
  !verificationRequestHasQr(request);

export const verificationRequestCanFallbackToSas = (request: NativeVerificationRequest): boolean =>
  verificationRequestHasQr(request) &&
  !verificationRequestHasSasCodes(request) &&
  !isNativeVerificationTerminal(request.phase);

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

export const listNativeVerificationRequests = async (): Promise<NativeVerificationInbox> => {
  const inbox = await invokeNativeVerification<NativeVerificationInbox>('matrix_verification_list');
  return parseNativeVerificationInbox(inbox) ?? inbox;
};

const mutateNativeVerification = async <T>(
  command: string,
  args?: Record<string, unknown>
): Promise<T> => {
  const value = await invokeNativeVerification<T>(command, args);
  announceNativeVerificationChanged();
  if (isRecord(value) && typeof value.flowId === 'string') {
    return sanitizeNativeVerificationRequest(value as unknown as NativeVerificationRequest) as T;
  }
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

export const MAX_QR_IMAGE_DATA_URL_CHARS = 12288;

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

export const parseNativeVerificationQr = (value: unknown): NativeVerificationQr | undefined => {
  if (!isRecord(value)) return undefined;
  const imageDataUrl = value.imageDataUrl;
  if (typeof imageDataUrl !== 'string' || !imageDataUrl.startsWith('data:image/svg+xml')) {
    return undefined;
  }
  if (imageDataUrl.length === 0 || [...imageDataUrl].length > MAX_QR_IMAGE_DATA_URL_CHARS) {
    return undefined;
  }
  return { imageDataUrl, scanned: value.scanned === true };
};

export const sanitizeNativeVerificationRequest = (
  request: NativeVerificationRequest
): NativeVerificationRequest => {
  if (!request.qr) return request;
  const qr = parseNativeVerificationQr(request.qr);
  if (!qr) {
    const rest = { ...request };
    delete rest.qr;
    return rest;
  }
  return { ...request, qr };
};

export const parseNativeVerificationInbox = (
  value: unknown
): NativeVerificationInbox | undefined => {
  if (!isRecord(value)) return undefined;
  if (typeof value.sessionGeneration !== 'number' || !Array.isArray(value.requests)) {
    return undefined;
  }
  return {
    sessionGeneration: value.sessionGeneration,
    requests: value.requests.flatMap((item) => {
      if (!isRecord(item) || typeof item.flowId !== 'string') return [];
      const request = item as unknown as NativeVerificationRequest;
      return [sanitizeNativeVerificationRequest(request)];
    }),
  };
};
