import { isSynaraDesktop } from '../utils/desktop';
import { normalizePlatformSecretStoreStatus, type PlatformSecretStoreStatus } from './secrets';

export type PlatformChannel = 'local-dev-runtime' | 'desktop-tauri' | 'ios-native' | 'unknown';

// Windows builds report canPersistSession=false from the desktop bridge; never infer
// Keychain-equivalent persistence from the desktop channel alone.
export const applyDesktopSecretStoreCapability = (status: PlatformSecretStoreStatus): void => {
  if (!isSynaraDesktop() || !window.__SYNARA_DESKTOP__) return;

  window.__SYNARA_DESKTOP__.supportsSecureSecretStore =
    status.available && status.canPersistSession;
};

export const syncDesktopSecretStoreCapability = async (
  status: unknown
): Promise<PlatformSecretStoreStatus | undefined> => {
  const normalized = normalizePlatformSecretStoreStatus(status);
  if (normalized) {
    applyDesktopSecretStoreCapability(normalized);
  }
  return normalized;
};

export type PlatformCapabilities = {
  channel: PlatformChannel;
  supportsSystemNotifications: boolean;
  supportsAppBadge: boolean;
  supportsTray: boolean;
  supportsGlobalShortcuts: boolean;
  supportsNativeFileSave: boolean;
  supportsNativeFileDrop: boolean;
  supportsAgentActions: boolean;
  supportsUpdater: boolean;
  supportsSecureSecretStore: boolean;
  supportsIntegrationStatus: boolean;
  supportsTrayState: boolean;
};

export const getPlatformCapabilities = (): PlatformCapabilities => {
  const desktop = isSynaraDesktop();
  const desktopBridge = window.__SYNARA_DESKTOP__;

  return {
    channel: desktop ? 'desktop-tauri' : 'local-dev-runtime',
    supportsSystemNotifications: desktop,
    supportsAppBadge: desktop,
    supportsTray: desktop,
    supportsGlobalShortcuts: desktop,
    supportsNativeFileSave: desktop,
    supportsNativeFileDrop: desktop,
    supportsAgentActions: desktop,
    supportsUpdater: desktop && desktopBridge?.supportsUpdater === true,
    supportsSecureSecretStore: desktop && desktopBridge?.supportsSecureSecretStore === true,
    supportsIntegrationStatus: desktop && desktopBridge?.supportsIntegrationStatus === true,
    supportsTrayState: desktop && desktopBridge?.supportsTrayState === true,
  };
};

export const isDesktopPlatform = (): boolean =>
  getPlatformCapabilities().channel === 'desktop-tauri';

export const supportsPlatformSystemNotifications = (): boolean =>
  getPlatformCapabilities().supportsSystemNotifications;

export const supportsPlatformNativeFileSave = (): boolean =>
  getPlatformCapabilities().supportsNativeFileSave;

export const supportsPlatformNativeFileDrop = (): boolean =>
  getPlatformCapabilities().supportsNativeFileDrop;

export const supportsPlatformGlobalShortcuts = (): boolean =>
  getPlatformCapabilities().supportsGlobalShortcuts;

export const supportsPlatformTrayState = (): boolean => getPlatformCapabilities().supportsTrayState;

export const supportsPlatformUpdater = (): boolean => getPlatformCapabilities().supportsUpdater;
