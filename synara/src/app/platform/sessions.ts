import { type Session } from '../state/sessions';
import {
  invokeDesktopWithAvailability,
  isDesktopBridgeAvailable,
  isSynaraDesktop,
} from '../utils/desktop';
import { syncDesktopSecretStoreCapability } from './capabilities';
import { getPlatformSecretStoreStatus, type PlatformSecretStoreStatus } from './secrets';

export type PlatformSessionStore = {
  getStatus: () => Promise<PlatformSecretStoreStatus>;
  getSession: () => Promise<Session | undefined>;
  setSession: (session: Session) => Promise<boolean>;
  removeSession: () => Promise<boolean>;
};

const readString = (record: Record<string, unknown>, key: keyof Session): string | undefined => {
  const value = record[key];
  if (typeof value !== 'string') return undefined;
  const normalized = value.trim();
  return normalized.length > 0 ? normalized : undefined;
};

export const normalizePlatformSession = (value: unknown): Session | undefined => {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return undefined;
  const record = value as Record<string, unknown>;
  const baseUrl = readString(record, 'baseUrl');
  const userId = readString(record, 'userId');
  const deviceId = readString(record, 'deviceId');
  const accessToken = readString(record, 'accessToken');

  if (!baseUrl || !userId || !deviceId || !accessToken) return undefined;

  const expiresInMs = record.expiresInMs;
  const refreshToken = readString(record, 'refreshToken');

  return {
    baseUrl,
    userId,
    deviceId,
    accessToken,
    refreshToken,
    expiresInMs:
      typeof expiresInMs === 'number' && Number.isFinite(expiresInMs) ? expiresInMs : undefined,
  };
};

const canUsePlatformSessionStore = (status: PlatformSecretStoreStatus): boolean =>
  status.available && status.canPersistSession;

const serializePlatformSession = (session: Session) => {
  const envelope: Omit<Session, 'fallbackSdkStores'> = {
    baseUrl: session.baseUrl,
    userId: session.userId,
    deviceId: session.deviceId,
    accessToken: session.accessToken,
  };

  if (session.refreshToken) envelope.refreshToken = session.refreshToken;
  if (typeof session.expiresInMs === 'number' && Number.isFinite(session.expiresInMs)) {
    envelope.expiresInMs = session.expiresInMs;
  }

  return envelope;
};

const unavailableDesktopSecretStoreStatus = (): PlatformSecretStoreStatus => ({
  available: false,
  backend: 'none',
  canPersistSession: false,
  reason: 'secure-secret-store-unavailable',
});

export const platformSessionStore: PlatformSessionStore = {
  getStatus: async () => {
    if (!isSynaraDesktop()) {
      return getPlatformSecretStoreStatus();
    }

    try {
      const invokeResult = await invokeDesktopWithAvailability<unknown>(
        'desktop_secret_store_status'
      );
      if (!invokeResult.available) {
        return unavailableDesktopSecretStoreStatus();
      }
      return (
        (await syncDesktopSecretStoreCapability(invokeResult.value)) ??
        unavailableDesktopSecretStoreStatus()
      );
    } catch {
      return unavailableDesktopSecretStoreStatus();
    }
  },

  getSession: async () => {
    const status = await platformSessionStore.getStatus();
    if (!canUsePlatformSessionStore(status)) return undefined;

    if (!isDesktopBridgeAvailable()) {
      return undefined;
    }
    const invokeResult = await invokeDesktopWithAvailability<unknown>('desktop_get_session');
    if (!invokeResult.available) {
      return undefined;
    }
    return normalizePlatformSession(invokeResult.value);
  },

  setSession: async (session) => {
    const status = await platformSessionStore.getStatus();
    if (!canUsePlatformSessionStore(status)) return false;

    if (!isDesktopBridgeAvailable()) {
      return false;
    }
    const invokeResult = await invokeDesktopWithAvailability<boolean>('desktop_set_session', {
      session: serializePlatformSession(session),
    });
    if (!invokeResult.available) {
      return false;
    }
    return invokeResult.value === true;
  },

  removeSession: async () => {
    const status = await platformSessionStore.getStatus();
    if (!canUsePlatformSessionStore(status)) return false;

    if (!isDesktopBridgeAvailable()) {
      return false;
    }
    const invokeResult = await invokeDesktopWithAvailability<boolean>('desktop_remove_session');
    if (!invokeResult.available) {
      return false;
    }
    return invokeResult.value === true;
  },
};
