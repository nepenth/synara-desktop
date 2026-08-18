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

const readString = (record: Record<string, unknown>, key: string): string | undefined => {
  const value = record[key];
  if (typeof value !== 'string') return undefined;
  const normalized = value.trim();
  return normalized.length > 0 ? normalized : undefined;
};

export const normalizePlatformSession = (value: unknown): Session | undefined => {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return undefined;
  const record = value as Record<string, unknown>;
  const baseUrl = readString(record, 'homeserverUrl');
  const userId = readString(record, 'userId');
  const deviceId = readString(record, 'deviceId');
  if (!baseUrl || !userId || !deviceId) return undefined;

  return {
    baseUrl,
    userId,
    deviceId,
    // Native Matrix credentials never cross IPC. This identity-only marker
    // preserves the legacy Session shape while the facade owns every request.
    accessToken: '',
  };
};

const canUsePlatformSessionStore = (status: PlatformSecretStoreStatus): boolean =>
  status.available && status.canPersistSession;

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
      const invokeResult = await invokeDesktopWithAvailability<unknown>('matrix_restore_session');
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
        persistence: 'host-only',
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

  setSession: async () => {
    const startedAtMs = performance.now();
    recordClientDiagnostic('session', 'platform-store.write-completed', {
      outcome: 'host-owned',
      durationMs: performance.now() - startedAtMs,
      persistence: 'disabled',
    });
    return false;
  },

  removeSession: async () => {
    const startedAtMs = performance.now();
    recordClientDiagnostic('session', 'platform-store.remove-completed', {
      outcome: 'host-owned',
      durationMs: performance.now() - startedAtMs,
    });
    return false;
  },
};
