import { getPlatformCapabilities } from './capabilities';

export type PlatformSecretStoreBackend =
  | 'none'
  | 'desktop-native'
  | 'macos-keychain'
  | 'linux-secret-service'
  | 'linux-keyutils'
  | 'ios-keychain'
  | 'unknown';

export type PlatformSecretStoreStatus = {
  available: boolean;
  backend: PlatformSecretStoreBackend;
  canPersistSession: boolean;
  reason?: string;
};

export type PlatformSecretStoreSessionPersistence = 'persistent' | 'session-scoped' | 'fallback';

const PLATFORM_SECRET_STORE_BACKENDS = new Set<PlatformSecretStoreBackend>([
  'none',
  'desktop-native',
  'macos-keychain',
  'linux-secret-service',
  'linux-keyutils',
  'ios-keychain',
  'unknown',
]);

const isPlatformSecretStoreBackend = (value: unknown): value is PlatformSecretStoreBackend =>
  typeof value === 'string' &&
  PLATFORM_SECRET_STORE_BACKENDS.has(value as PlatformSecretStoreBackend);

export const normalizePlatformSecretStoreStatus = (
  value: unknown
): PlatformSecretStoreStatus | undefined => {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return undefined;

  const record = value as Record<string, unknown>;
  const available = record.available === true;
  const canPersistSession =
    typeof record.canPersistSession === 'boolean' ? record.canPersistSession : available;
  const backend = isPlatformSecretStoreBackend(record.backend)
    ? record.backend
    : available
    ? 'unknown'
    : 'none';
  const reason =
    typeof record.reason === 'string' && record.reason.length > 0 ? record.reason : undefined;

  return {
    available,
    backend,
    canPersistSession,
    reason,
  };
};

export const getPlatformSecretStoreStatus = (): PlatformSecretStoreStatus => {
  const capabilities = getPlatformCapabilities();

  if (!capabilities.supportsSecureSecretStore) {
    return {
      available: false,
      backend: 'none',
      canPersistSession: false,
      reason: 'secure-secret-store-not-configured',
    };
  }

  if (capabilities.channel === 'ios-native') {
    return { available: true, backend: 'ios-keychain', canPersistSession: true };
  }

  if (capabilities.channel === 'desktop-tauri') {
    if (!capabilities.supportsSecureSecretStore) {
      return {
        available: false,
        backend: 'none',
        canPersistSession: false,
        reason: 'secure-secret-store-not-configured',
      };
    }

    return { available: true, backend: 'desktop-native', canPersistSession: true };
  }

  return {
    available: false,
    backend: 'none',
    canPersistSession: false,
    reason: 'secure-secret-store-not-configured',
  };
};

export const getPlatformSecretStoreBackendLabel = (backend: PlatformSecretStoreBackend): string => {
  switch (backend) {
    case 'desktop-native':
      return 'Desktop native store';
    case 'macos-keychain':
      return 'macOS Keychain';
    case 'linux-secret-service':
      return 'Linux Secret Service';
    case 'linux-keyutils':
      return 'Linux keyutils';
    case 'ios-keychain':
      return 'iOS Keychain';
    case 'none':
      return 'No native store';
    case 'unknown':
      return 'Unknown native store';
  }
};

export const getPlatformSecretStoreSessionPersistence = (
  status: PlatformSecretStoreStatus
): PlatformSecretStoreSessionPersistence => {
  if (!status.available) return 'fallback';
  return status.canPersistSession ? 'persistent' : 'session-scoped';
};

export const getPlatformSecretStoreStatusLabel = (status: PlatformSecretStoreStatus): string => {
  const persistence = getPlatformSecretStoreSessionPersistence(status);

  if (persistence === 'persistent') return 'Persistent';
  if (persistence === 'session-scoped') return 'Session scoped';
  return 'Fallback';
};

export const getPlatformSecretStoreStatusDescription = (
  status: PlatformSecretStoreStatus
): string => {
  const backendLabel = getPlatformSecretStoreBackendLabel(status.backend);
  const persistence = getPlatformSecretStoreSessionPersistence(status);

  if (persistence === 'persistent') {
    return `${backendLabel} is available for session storage.`;
  }

  if (persistence === 'session-scoped') {
    return `${backendLabel} is available, but stored sessions may not survive a restart.`;
  }

  if (status.reason === 'secure-secret-store-not-configured') {
    return 'Native credential storage is not configured for this runtime.';
  }

  if (status.reason === 'secure-secret-store-unavailable') {
    return `${backendLabel} is unavailable.`;
  }

  return 'Synara is using the fallback session store.';
};
