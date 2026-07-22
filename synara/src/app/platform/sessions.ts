import { type Session } from '../state/sessions';
import {
  invokeDesktopWithAvailability,
  isDesktopBridgeAvailable,
  isSynaraDesktop,
} from '../utils/desktop';
import { syncDesktopSecretStoreCapability } from './capabilities';
import { getPlatformSecretStoreStatus, type PlatformSecretStoreStatus } from './secrets';
import { recordClientDiagnostic } from '../utils/clientDiagnostics';

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
  const storedAtMs = record.storedAtMs;
  const refreshToken = readString(record, 'refreshToken');
  const sessionGeneration = readString(record, 'sessionGeneration');

  return {
    baseUrl,
    userId,
    deviceId,
    accessToken,
    refreshToken,
    ...(sessionGeneration ? { sessionGeneration } : {}),
    expiresInMs:
      typeof expiresInMs === 'number' && Number.isFinite(expiresInMs) ? expiresInMs : undefined,
    storedAtMs:
      typeof storedAtMs === 'number' && Number.isFinite(storedAtMs) ? storedAtMs : undefined,
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
  if (session.sessionGeneration) envelope.sessionGeneration = session.sessionGeneration;
  if (typeof session.expiresInMs === 'number' && Number.isFinite(session.expiresInMs)) {
    envelope.expiresInMs = session.expiresInMs;
  }
  if (typeof session.storedAtMs === 'number' && Number.isFinite(session.storedAtMs)) {
    envelope.storedAtMs = session.storedAtMs;
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
    const startedAtMs = performance.now();
    if (!isSynaraDesktop()) {
      const status = await getPlatformSecretStoreStatus();
      recordClientDiagnostic('session', 'platform-store.status-completed', {
        outcome: status.available ? 'available' : 'unavailable',
        backend: status.backend,
        canPersistSession: status.canPersistSession,
        durationMs: performance.now() - startedAtMs,
      });
      return status;
    }

    try {
      const invokeResult = await invokeDesktopWithAvailability<unknown>(
        'desktop_secret_store_status'
      );
      if (!invokeResult.available) {
        recordClientDiagnostic('session', 'platform-store.status-completed', {
          outcome: 'bridge-unavailable',
          durationMs: performance.now() - startedAtMs,
        });
        return unavailableDesktopSecretStoreStatus();
      }
      const status =
        (await syncDesktopSecretStoreCapability(invokeResult.value)) ??
        unavailableDesktopSecretStoreStatus();
      recordClientDiagnostic('session', 'platform-store.status-completed', {
        outcome: status.available ? 'available' : 'unavailable',
        backend: status.backend,
        canPersistSession: status.canPersistSession,
        durationMs: performance.now() - startedAtMs,
      });
      return status;
    } catch (error) {
      recordClientDiagnostic('session', 'platform-store.status-completed', {
        outcome: 'error',
        durationMs: performance.now() - startedAtMs,
        errorType: error instanceof Error ? error.name : typeof error,
      });
      return unavailableDesktopSecretStoreStatus();
    }
  },

  getSession: async () => {
    const startedAtMs = performance.now();
    const status = await platformSessionStore.getStatus();
    if (!canUsePlatformSessionStore(status)) {
      recordClientDiagnostic('session', 'platform-store.read-completed', {
        outcome: 'store-unavailable',
        backend: status.backend,
        durationMs: performance.now() - startedAtMs,
      });
      return undefined;
    }

    if (!isDesktopBridgeAvailable()) {
      recordClientDiagnostic('session', 'platform-store.read-completed', {
        outcome: 'bridge-unavailable',
        backend: status.backend,
        durationMs: performance.now() - startedAtMs,
      });
      return undefined;
    }
    try {
      const invokeResult = await invokeDesktopWithAvailability<unknown>('desktop_get_session');
      if (!invokeResult.available) {
        recordClientDiagnostic('session', 'platform-store.read-completed', {
          outcome: 'bridge-unavailable',
          backend: status.backend,
          durationMs: performance.now() - startedAtMs,
        });
        return undefined;
      }
      const session = normalizePlatformSession(invokeResult.value);
      recordClientDiagnostic('session', 'platform-store.read-completed', {
        outcome: session ? 'found' : 'missing',
        backend: status.backend,
        durationMs: performance.now() - startedAtMs,
        hasRefreshToken: Boolean(session?.refreshToken),
        hasExpiryMetadata: typeof session?.expiresInMs === 'number',
      });
      return session;
    } catch (error) {
      recordClientDiagnostic('session', 'platform-store.read-completed', {
        outcome: 'error',
        backend: status.backend,
        durationMs: performance.now() - startedAtMs,
        errorType: error instanceof Error ? error.name : typeof error,
      });
      throw error;
    }
  },

  setSession: async (session) => {
    const startedAtMs = performance.now();
    const status = await platformSessionStore.getStatus();
    if (!canUsePlatformSessionStore(status)) {
      recordClientDiagnostic('session', 'platform-store.write-completed', {
        outcome: 'store-unavailable',
        backend: status.backend,
        durationMs: performance.now() - startedAtMs,
      });
      return false;
    }

    if (!isDesktopBridgeAvailable()) {
      recordClientDiagnostic('session', 'platform-store.write-completed', {
        outcome: 'bridge-unavailable',
        backend: status.backend,
        durationMs: performance.now() - startedAtMs,
      });
      return false;
    }
    try {
      const invokeResult = await invokeDesktopWithAvailability<boolean>('desktop_set_session', {
        session: serializePlatformSession(session),
      });
      const persisted = invokeResult.available && invokeResult.value === true;
      recordClientDiagnostic('session', 'platform-store.write-completed', {
        outcome: persisted
          ? 'persisted'
          : invokeResult.available
          ? 'unavailable'
          : 'bridge-unavailable',
        backend: status.backend,
        durationMs: performance.now() - startedAtMs,
        hasRefreshToken: Boolean(session.refreshToken),
        hasExpiryMetadata: typeof session.expiresInMs === 'number',
      });
      return persisted;
    } catch (error) {
      recordClientDiagnostic('session', 'platform-store.write-completed', {
        outcome: 'error',
        backend: status.backend,
        durationMs: performance.now() - startedAtMs,
        errorType: error instanceof Error ? error.name : typeof error,
      });
      throw error;
    }
  },

  removeSession: async () => {
    const startedAtMs = performance.now();
    const status = await platformSessionStore.getStatus();
    if (!canUsePlatformSessionStore(status)) {
      recordClientDiagnostic('session', 'platform-store.remove-completed', {
        outcome: 'store-unavailable',
        backend: status.backend,
        durationMs: performance.now() - startedAtMs,
      });
      return false;
    }

    if (!isDesktopBridgeAvailable()) {
      recordClientDiagnostic('session', 'platform-store.remove-completed', {
        outcome: 'bridge-unavailable',
        backend: status.backend,
        durationMs: performance.now() - startedAtMs,
      });
      return false;
    }
    try {
      const invokeResult = await invokeDesktopWithAvailability<boolean>('desktop_remove_session');
      const removed = invokeResult.available && invokeResult.value === true;
      recordClientDiagnostic('session', 'platform-store.remove-completed', {
        outcome: removed ? 'removed' : invokeResult.available ? 'missing' : 'bridge-unavailable',
        backend: status.backend,
        durationMs: performance.now() - startedAtMs,
      });
      return removed;
    } catch (error) {
      recordClientDiagnostic('session', 'platform-store.remove-completed', {
        outcome: 'error',
        backend: status.backend,
        durationMs: performance.now() - startedAtMs,
        errorType: error instanceof Error ? error.name : typeof error,
      });
      throw error;
    }
  },
};
