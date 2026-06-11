import test from 'node:test';
import assert from 'node:assert/strict';
import {
  applyDesktopSecretStoreCapability,
  getPlatformCapabilities,
  getPlatformIntegrationStatus,
  getPlatformSecretStoreBackendLabel,
  getPlatformSecretStoreSessionPersistence,
  getPlatformSecretStoreStatusDescription,
  getPlatformSecretStoreStatusLabel,
  getPlatformNotificationCount,
  getPlatformSecretStoreStatus,
  isDesktopPlatform,
  platformSessionStore,
  repairPlatformDeviceDisplayName,
  setPlatformBadgeCount,
  showPlatformNotification,
  setPlatformTrayState,
  supportsPlatformGlobalShortcuts,
} from '..';

test('platform capabilities describe local development runtime by default', () => {
  const originalWindow = globalThis.window;
  (globalThis as any).window = {};

  try {
    assert.deepEqual(getPlatformCapabilities(), {
      channel: 'local-dev-runtime',
      supportsSystemNotifications: false,
      supportsAppBadge: false,
      supportsTray: false,
      supportsGlobalShortcuts: false,
      supportsNativeFileSave: false,
      supportsNativeFileDrop: false,
      supportsAgentActions: false,
      supportsSecureSecretStore: false,
      supportsIntegrationStatus: false,
      supportsTrayState: false,
    });
    assert.equal(isDesktopPlatform(), false);
    assert.equal(supportsPlatformGlobalShortcuts(), false);
    assert.deepEqual(getPlatformSecretStoreStatus(), {
      available: false,
      backend: 'none',
      canPersistSession: false,
      reason: 'secure-secret-store-not-configured',
    });
  } finally {
    (globalThis as any).window = originalWindow;
  }
});

test('platform device display-name repair uses Matrix SDK device details', async () => {
  const updates: Array<{ deviceId: string; displayName: string }> = [];
  const client = {
    getDeviceId: () => 'DEVICE',
    getDevice: async () => ({ display_name: 'Legacy Device' }),
    setDeviceDetails: async (deviceId: string, details: { display_name: string }) => {
      updates.push({ deviceId, displayName: details.display_name });
      return {};
    },
  };

  assert.equal(await repairPlatformDeviceDisplayName(client, 'Synara macOS'), true);
  assert.deepEqual(updates, [{ deviceId: 'DEVICE', displayName: 'Synara macOS' }]);
});

test('platform device display-name repair skips already-correct or missing devices', async () => {
  const updates: Array<string> = [];
  const matchingClient = {
    getDeviceId: () => 'DEVICE',
    getDevice: async () => ({ display_name: 'Synara Linux' }),
    setDeviceDetails: async (deviceId: string) => {
      updates.push(deviceId);
      return {};
    },
  };
  const missingClient = {
    getDeviceId: () => undefined,
    getDevice: async () => undefined,
    setDeviceDetails: async (deviceId: string) => {
      updates.push(deviceId);
      return {};
    },
  };

  assert.equal(await repairPlatformDeviceDisplayName(matchingClient, 'Synara Linux'), false);
  assert.equal(await repairPlatformDeviceDisplayName(missingClient, 'Synara Linux'), false);
  assert.deepEqual(updates, []);
});

test('platform capabilities describe Tauri desktop runtime when bridge is present', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const originalWindow = globalThis.window;
  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      supportsIntegrationStatus: true,
      supportsTrayState: true,
      invoke: async (command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        return true;
      },
    },
  };

  try {
    const capabilities = getPlatformCapabilities();
    assert.equal(capabilities.channel, 'desktop-tauri');
    assert.equal(capabilities.supportsSystemNotifications, true);
    assert.equal(capabilities.supportsAppBadge, true);
    assert.equal(capabilities.supportsGlobalShortcuts, true);
    assert.equal(capabilities.supportsSecureSecretStore, false);
    assert.equal(capabilities.supportsIntegrationStatus, true);
    assert.equal(capabilities.supportsTrayState, true);
    assert.equal(isDesktopPlatform(), true);

    await setPlatformBadgeCount(9.9);
    await setPlatformTrayState({
      unreadCount: 3.8,
      highlightCount: 1,
      laterCount: -2,
      notificationInboxCount: 4,
      doNotDisturb: true,
    });
    assert.deepEqual(calls, [
      {
        command: 'desktop_set_badge_count',
        args: { count: 9 },
      },
      {
        command: 'desktop_update_tray_state',
        args: {
          state: {
            unreadCount: 3,
            highlightCount: 1,
            laterCount: 0,
            notificationInboxCount: 4,
            doNotDisturb: true,
          },
        },
      },
    ]);
  } finally {
    (globalThis as any).window = originalWindow;
  }
});

test('platform diagnostics reads desktop integration status when supported', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const originalWindow = globalThis.window;
  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      supportsIntegrationStatus: true,
      invoke: async (command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        return {
          platform: 'linux',
          desktopEnvironment: 'kde',
          sessionType: 'wayland',
          distroId: 'arch',
          distroName: 'Arch Linux',
          distroVersion: 'rolling',
          buildIdentity: 'debug',
          tray: { name: 'Tray', ready: true, supported: true, message: 'Ready' },
          notifications: { name: 'Notifications', ready: true, supported: true, message: 'Ready' },
          globalShortcuts: {
            name: 'Global Shortcuts',
            ready: false,
            supported: true,
            message: 'Permission needed',
          },
          filePortal: { name: 'File Portal', ready: true, supported: true, message: 'Ready' },
          mediaPortal: { name: 'Media Portal', ready: false, supported: false, message: 'Missing' },
        };
      },
    },
  };

  try {
    assert.deepEqual(await getPlatformIntegrationStatus(), {
      platform: 'linux',
      desktopEnvironment: 'kde',
      sessionType: 'wayland',
      distroId: 'arch',
      distroName: 'Arch Linux',
      distroVersion: 'rolling',
      buildIdentity: 'debug',
      tray: { name: 'Tray', ready: true, supported: true, message: 'Ready' },
      notifications: { name: 'Notifications', ready: true, supported: true, message: 'Ready' },
      globalShortcuts: {
        name: 'Global Shortcuts',
        ready: false,
        supported: true,
        message: 'Permission needed',
      },
      filePortal: { name: 'File Portal', ready: true, supported: true, message: 'Ready' },
      mediaPortal: { name: 'Media Portal', ready: false, supported: false, message: 'Missing' },
    });
    assert.deepEqual(calls, [{ command: 'desktop_get_integration_status', args: undefined }]);
  } finally {
    (globalThis as any).window = originalWindow;
  }
});

test('platform bridge defaults to secure secret store disabled before runtime sync', () => {
  const originalWindow = globalThis.window;
  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
    },
  };

  try {
    assert.equal(getPlatformCapabilities().supportsSecureSecretStore, false);
    assert.deepEqual(getPlatformSecretStoreStatus(), {
      available: false,
      backend: 'none',
      canPersistSession: false,
      reason: 'secure-secret-store-not-configured',
    });
  } finally {
    (globalThis as any).window = originalWindow;
  }
});

test('platform capabilities expose secure secret store only when bridge opts in', () => {
  const originalWindow = globalThis.window;
  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      supportsSecureSecretStore: true,
    },
  };

  try {
    assert.equal(getPlatformCapabilities().supportsSecureSecretStore, true);
    assert.deepEqual(getPlatformSecretStoreStatus(), {
      available: true,
      backend: 'desktop-native',
      canPersistSession: true,
    });
  } finally {
    (globalThis as any).window = originalWindow;
  }
});

test('platform secret-store helpers describe persistence behavior', () => {
  const persistentStatus = {
    available: true,
    backend: 'macos-keychain' as const,
    canPersistSession: true,
  };
  const scopedStatus = {
    available: true,
    backend: 'linux-keyutils' as const,
    canPersistSession: false,
  };
  const fallbackStatus = {
    available: false,
    backend: 'none' as const,
    canPersistSession: false,
  };

  assert.equal(getPlatformSecretStoreBackendLabel(persistentStatus.backend), 'macOS Keychain');
  assert.equal(getPlatformSecretStoreBackendLabel(scopedStatus.backend), 'Linux keyutils');
  assert.equal(getPlatformSecretStoreSessionPersistence(persistentStatus), 'persistent');
  assert.equal(getPlatformSecretStoreSessionPersistence(scopedStatus), 'session-scoped');
  assert.equal(getPlatformSecretStoreSessionPersistence(fallbackStatus), 'fallback');
  assert.equal(getPlatformSecretStoreStatusLabel(persistentStatus), 'Persistent');
  assert.equal(getPlatformSecretStoreStatusLabel(scopedStatus), 'Session scoped');
  assert.equal(getPlatformSecretStoreStatusLabel(fallbackStatus), 'Fallback');
  assert.equal(
    getPlatformSecretStoreStatusDescription(persistentStatus),
    'macOS Keychain is available for session storage.'
  );
  assert.equal(
    getPlatformSecretStoreStatusDescription(scopedStatus),
    'Linux keyutils is available, but stored sessions may not survive a restart.'
  );
  assert.equal(
    getPlatformSecretStoreStatusDescription({
      ...fallbackStatus,
      reason: 'secure-secret-store-not-configured',
    }),
    'Native credential storage is not configured for this runtime.'
  );
});

test('platform session store probes desktop secret-store status without bridge opt-in', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const originalWindow = globalThis.window;
  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      invoke: async (command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        return {
          available: false,
          backend: 'none',
          canPersistSession: false,
          reason: 'secure-secret-store-not-configured',
        };
      },
    },
  };

  try {
    assert.deepEqual(await platformSessionStore.getStatus(), {
      available: false,
      backend: 'none',
      canPersistSession: false,
      reason: 'secure-secret-store-not-configured',
    });
    assert.deepEqual(calls, [{ command: 'desktop_secret_store_status', args: undefined }]);
    assert.equal(getPlatformCapabilities().supportsSecureSecretStore, false);
  } finally {
    (globalThis as any).window = originalWindow;
  }
});

test('platform secret-store capability sync advertises persistence only when probed', () => {
  const originalWindow = globalThis.window;
  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
    },
  };

  try {
    applyDesktopSecretStoreCapability({
      available: true,
      backend: 'macos-keychain',
      canPersistSession: true,
    });
    assert.equal(getPlatformCapabilities().supportsSecureSecretStore, true);

    applyDesktopSecretStoreCapability({
      available: true,
      backend: 'linux-keyutils',
      canPersistSession: false,
    });
    assert.equal(getPlatformCapabilities().supportsSecureSecretStore, false);
  } finally {
    (globalThis as any).window = originalWindow;
  }
});

test('platform session store reads normalized desktop sessions when available', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const originalWindow = globalThis.window;
  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      supportsSecureSecretStore: true,
      invoke: async (command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        if (command === 'desktop_secret_store_status') {
          return { available: true, backend: 'macos-keychain', canPersistSession: true };
        }
        return {
          baseUrl: ' https://matrix.example.org ',
          userId: ' @alice:example.org ',
          deviceId: ' DEVICEID ',
          accessToken: ' access-token ',
          refreshToken: ' refresh-token ',
          expiresInMs: 1234,
        };
      },
    },
  };

  try {
    assert.deepEqual(await platformSessionStore.getSession(), {
      baseUrl: 'https://matrix.example.org',
      userId: '@alice:example.org',
      deviceId: 'DEVICEID',
      accessToken: 'access-token',
      refreshToken: 'refresh-token',
      expiresInMs: 1234,
    });
    assert.deepEqual(
      calls.map((call) => call.command),
      ['desktop_secret_store_status', 'desktop_get_session']
    );
  } finally {
    (globalThis as any).window = originalWindow;
  }
});

test('platform session store strips fallback-only fields before desktop persistence', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const originalWindow = globalThis.window;
  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      supportsSecureSecretStore: true,
      invoke: async (command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        if (command === 'desktop_secret_store_status') {
          return { available: true, backend: 'macos-keychain', canPersistSession: true };
        }
        return true;
      },
    },
  };

  try {
    assert.equal(
      await platformSessionStore.setSession({
        baseUrl: 'https://matrix.example.org',
        userId: '@alice:example.org',
        deviceId: 'DEVICEID',
        accessToken: 'access-token',
        fallbackSdkStores: true,
      }),
      true
    );
    assert.deepEqual(calls, [
      { command: 'desktop_secret_store_status', args: undefined },
      {
        command: 'desktop_set_session',
        args: {
          session: {
            baseUrl: 'https://matrix.example.org',
            userId: '@alice:example.org',
            deviceId: 'DEVICEID',
            accessToken: 'access-token',
          },
        },
      },
    ]);
  } finally {
    (globalThis as any).window = originalWindow;
  }
});

test('platform session store removes desktop sessions when available', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const originalWindow = globalThis.window;
  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      supportsSecureSecretStore: true,
      invoke: async (command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        if (command === 'desktop_secret_store_status') {
          return { available: true, backend: 'macos-keychain', canPersistSession: true };
        }
        return true;
      },
    },
  };

  try {
    assert.equal(await platformSessionStore.removeSession(), true);
    assert.deepEqual(calls, [
      { command: 'desktop_secret_store_status', args: undefined },
      { command: 'desktop_remove_session', args: undefined },
    ]);
  } finally {
    (globalThis as any).window = originalWindow;
  }
});

test('platform notification count preserves existing desktop badge semantics', () => {
  assert.equal(getPlatformNotificationCount([{ total: 4, highlight: 2 }, { total: 3 }], 1), 6);
});

test('platform notifications normalize shared requests before desktop delivery', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const originalWindow = globalThis.window;
  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      invoke: async (command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        return true;
      },
    },
  };

  try {
    const result = await showPlatformNotification({
      title: '  Reminder  ',
      body: '  Due now.  ',
      route: 'https://example.org',
      privacy: 'private',
      sound: 'silent',
    });

    assert.equal(result, true);
    assert.deepEqual(calls, [
      {
        command: 'desktop_notify',
        args: {
          notification: {
            title: 'Reminder',
            body: 'Due now.',
            route: undefined,
          },
        },
      },
    ]);
  } finally {
    (globalThis as any).window = originalWindow;
  }
});
