import { type Session } from '../state/sessions';
import { invokeDesktop, isSynaraDesktop } from '../utils/desktop';
import { syncDesktopSecretStoreCapability } from './capabilities';
import {
  getPlatformSecretStoreStatus,
  normalizePlatformSecretStoreStatus,
  type PlatformSecretStoreStatus,
} from './secrets';

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
      const status = await invokeDesktop<unknown>('desktop_secret_store_status');
      return (
        (await syncDesktopSecretStoreCapability(status)) ?? unavailableDesktopSecretStoreStatus()
      );
    } catch {
      return unavailableDesktopSecretStoreStatus();
    }
  },

  getSession: async () => {
    const status = await platformSessionStore.getStatus();
    if (!canUsePlatformSessionStore(status)) return undefined;

    const session = await invokeDesktop<unknown>('desktop_get_session');
    return normalizePlatformSession(session);
  },

  setSession: async (session) => {
    const status = await platformSessionStore.getStatus();
    if (!canUsePlatformSessionStore(status)) return false;

    const stored = await invokeDesktop<boolean>('desktop_set_session', {
      session: serializePlatformSession(session),
    });
    return stored === true;
  },

  removeSession: async () => {
    const status = await platformSessionStore.getStatus();
    if (!canUsePlatformSessionStore(status)) return false;

    const removed = await invokeDesktop<boolean>('desktop_remove_session');
    return removed === true;
  },
};
