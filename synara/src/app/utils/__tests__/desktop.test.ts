import test from 'node:test';
import assert from 'node:assert/strict';
import {
  buildDesktopNotificationRoomRoute,
  createDebouncedTrayStateUpdater,
  DESKTOP_FILE_IPC_CHUNK_SIZE,
  DESKTOP_FILE_IPC_INLINE_THRESHOLD,
  DESKTOP_TRAY_DND_TOGGLE_EVENT,
  DESKTOP_TRAY_STATE_DEBOUNCE_MS,
  flushPendingDesktopTrayStateUpdate,
  getDesktopIntegrationStatus,
  getDesktopNotificationCount,
  getDesktopPerformanceCapabilities,
  invokeDesktopWithAvailability,
  isDesktopBridgeAvailable,
  openDesktopExternalUrl,
  readDesktopDroppedFiles,
  saveDesktopFile,
  sendDesktopAgentAction,
  setDesktopBadgeCount,
  setDesktopShortcuts,
  setDesktopTrayState,
  shouldStreamDesktopFileIpc,
  showDesktopNotification,
  subscribeDesktopTrayDndToggle,
  type DesktopTrayState,
} from '../desktop';
import { clearDesktopDiagnostics, getDesktopDiagnosticEntries } from '../desktopDiagnostics';

type DesktopActionCallArgs = {
  action?: {
    id: string;
    title: string;
    url?: string;
  };
};

test('desktop badge count combines highlights and active Later items', () => {
  assert.equal(
    getDesktopNotificationCount(
      [{ total: 4, highlight: 2 }, { total: 3 }, { total: 0, highlight: 0 }],
      5
    ),
    10
  );
});

test('desktop badge count clamps negative values', () => {
  assert.equal(getDesktopNotificationCount([{ total: -1, highlight: -2 }], -5), 0);
});

test('setDesktopBadgeCount invokes the desktop bridge with a clamped count', async () => {
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
    await setDesktopBadgeCount(3.8);
    await setDesktopBadgeCount(-2);
  } finally {
    (globalThis as any).window = originalWindow;
  }

  assert.deepEqual(calls, [
    { command: 'desktop_set_badge_count', args: { count: 3 } },
    { command: 'desktop_set_badge_count', args: { count: 0 } },
  ]);
});

test('desktop actions use explicit IPC command names', async () => {
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
    assert.equal(
      await sendDesktopAgentAction({
        id: 'export',
        title: 'Export thread',
        kind: 'export',
        markdown: '# Thread',
      }),
      true
    );
    assert.deepEqual(
      await setDesktopShortcuts({
        show: 'CmdOrCtrl+Shift+C',
        later: 'CmdOrCtrl+Shift+L',
        notifications: 'CmdOrCtrl+Shift+N',
      }),
      {
        success: true,
        state: 'active',
        message: 'Desktop shortcuts are active.',
      }
    );
  } finally {
    (globalThis as any).window = originalWindow;
  }

  assert.deepEqual(
    calls.map((call) => call.command),
    ['desktop_agent_action', 'desktop_set_shortcuts']
  );
});

test('desktop shortcuts normalize structured bridge results', async () => {
  const originalWindow = globalThis.window;
  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      supportsGlobalShortcuts: true,
      invoke: async () => ({
        success: false,
        state: 'permission-needed',
        message: 'Grant shortcut permissions in system settings.',
        fallbackCommand: 'open-settings',
      }),
    },
  };

  try {
    assert.deepEqual(
      await setDesktopShortcuts({
        show: 'CmdOrCtrl+Shift+C',
        later: 'CmdOrCtrl+Shift+L',
        notifications: 'CmdOrCtrl+Shift+N',
      }),
      {
        success: false,
        state: 'permission-needed',
        message: 'Grant shortcut permissions in system settings.',
        fallbackCommand: 'open-settings',
      }
    );
  } finally {
    (globalThis as any).window = originalWindow;
  }
});

test('desktop tray state debounce coalesces rapid updates with trailing flush', async () => {
  const calls: DesktopTrayState[] = [];
  const scheduled: Array<{ callback: () => void; delayMs: number }> = [];
  const debounced = createDebouncedTrayStateUpdater(
    async (state) => {
      calls.push(state);
      return true;
    },
    DESKTOP_TRAY_STATE_DEBOUNCE_MS,
    {
      schedule: (callback, delayMs) => {
        scheduled.push({ callback, delayMs });
        return scheduled.length;
      },
      cancel: () => undefined,
    }
  );

  void debounced({
    unreadCount: 1,
    highlightCount: 0,
    laterCount: 0,
    notificationInboxCount: 0,
    doNotDisturb: false,
  });
  void debounced({
    unreadCount: 7,
    highlightCount: 2,
    laterCount: 1,
    notificationInboxCount: 3,
    doNotDisturb: true,
  });

  assert.equal(calls.length, 0);
  assert.equal(scheduled.length, 2);
  assert.equal(scheduled[0]?.delayMs, DESKTOP_TRAY_STATE_DEBOUNCE_MS);
  assert.equal(scheduled[1]?.delayMs, DESKTOP_TRAY_STATE_DEBOUNCE_MS);

  scheduled.at(-1)?.callback();
  assert.deepEqual(calls, [
    {
      unreadCount: 7,
      highlightCount: 2,
      laterCount: 1,
      notificationInboxCount: 3,
      doNotDisturb: true,
    },
  ]);
});

test('desktop tray state debounce flush applies the latest pending state immediately', async () => {
  const calls: DesktopTrayState[] = [];
  const debounced = createDebouncedTrayStateUpdater(
    async (state) => {
      calls.push(state);
      return true;
    },
    DESKTOP_TRAY_STATE_DEBOUNCE_MS,
    {
      schedule: () => 1,
      cancel: () => undefined,
    }
  );

  void debounced({
    unreadCount: 4,
    highlightCount: 0,
    laterCount: 0,
    notificationInboxCount: 0,
    doNotDisturb: false,
  });
  await debounced.flush();

  assert.deepEqual(calls, [
    {
      unreadCount: 4,
      highlightCount: 0,
      laterCount: 0,
      notificationInboxCount: 0,
      doNotDisturb: false,
    },
  ]);
});

test('desktop tray state is capability gated and clamps counts', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const originalWindow = globalThis.window;

  try {
    clearDesktopDiagnostics();
    (globalThis as any).window = {};
    assert.equal(isDesktopBridgeAvailable(), false);
    assert.equal(
      await setDesktopTrayState({
        unreadCount: 1,
        highlightCount: 1,
        laterCount: 1,
        notificationInboxCount: 1,
        doNotDisturb: false,
      }),
      false
    );

    (globalThis as any).window = {
      __SYNARA_DESKTOP__: {
        platform: 'tauri',
        supportsTrayState: true,
        invoke: async (command: string, args?: Record<string, unknown>) => {
          calls.push({ command, args });
          return true;
        },
      },
    };

    assert.equal(
      await setDesktopTrayState({
        unreadCount: 3.9,
        highlightCount: -1,
        laterCount: Number.NaN,
        notificationInboxCount: 2,
        doNotDisturb: true,
      }),
      true
    );
    await flushPendingDesktopTrayStateUpdate();
  } finally {
    (globalThis as any).window = originalWindow;
  }

  assert.deepEqual(calls, [
    {
      command: 'desktop_update_tray_state',
      args: {
        state: {
          unreadCount: 3,
          highlightCount: 0,
          laterCount: 0,
          notificationInboxCount: 2,
          doNotDisturb: true,
        },
      },
    },
  ]);
});

test('desktop invoke distinguishes missing bridge from explicit false failures', async () => {
  const originalWindow = globalThis.window;
  clearDesktopDiagnostics();

  try {
    (globalThis as any).window = {};
    assert.deepEqual(await invokeDesktopWithAvailability('desktop_update_tray_state'), {
      available: false,
    });

    (globalThis as any).window = {
      __SYNARA_DESKTOP__: {
        platform: 'tauri',
        supportsTrayState: true,
        invoke: async () => false,
      },
    };

    assert.equal(
      await setDesktopTrayState({
        unreadCount: 1,
        highlightCount: 0,
        laterCount: 0,
        notificationInboxCount: 0,
        doNotDisturb: false,
      }),
      false
    );
    await flushPendingDesktopTrayStateUpdate();
    assert.match(
      getDesktopDiagnosticEntries().join('\n'),
      /desktop_update_tray_state returned false/
    );
  } finally {
    (globalThis as any).window = originalWindow;
    clearDesktopDiagnostics();
  }
});

test('desktop shortcut failures are recorded in diagnostics', async () => {
  const originalWindow = globalThis.window;
  clearDesktopDiagnostics();

  try {
    (globalThis as any).window = {
      __SYNARA_DESKTOP__: {
        platform: 'tauri',
        supportsGlobalShortcuts: true,
        invoke: async () => ({
          success: false,
          state: 'failed',
          message: 'Shortcut registration failed.',
        }),
      },
    };

    const result = await setDesktopShortcuts({
      show: 'CmdOrCtrl+Shift+C',
      later: 'CmdOrCtrl+Shift+L',
      notifications: 'CmdOrCtrl+Shift+N',
    });

    assert.equal(result.success, false);
    assert.match(
      getDesktopDiagnosticEntries().join('\n'),
      /desktop_set_shortcuts Shortcut registration failed/
    );
  } finally {
    (globalThis as any).window = originalWindow;
    clearDesktopDiagnostics();
  }
});

test('desktop integration status falls back when the bridge does not support diagnostics', async () => {
  const originalWindow = globalThis.window;
  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      desktopEnvironment: 'gnome',
      sessionType: 'wayland',
      supportsIntegrationStatus: false,
    },
  };

  try {
    assert.deepEqual(await getDesktopIntegrationStatus(), {
      platform: 'tauri',
      desktopEnvironment: 'gnome',
      sessionType: 'wayland',
      distroId: 'unknown',
      distroName: 'unknown',
      distroVersion: 'unknown',
      buildIdentity: 'unknown',
      tray: {
        name: 'Tray',
        ready: false,
        supported: false,
        message: 'Tray support is unavailable in this client.',
      },
      notifications: {
        name: 'Notifications',
        ready: false,
        supported: false,
        message: 'Notification support is unavailable in this client.',
      },
      globalShortcuts: {
        name: 'Global Shortcuts',
        ready: false,
        supported: false,
        message: 'Global shortcut support is unavailable in this client.',
      },
      filePortal: {
        name: 'File Portal',
        ready: false,
        supported: false,
        message: 'File portal support is unavailable in this client.',
      },
      mediaPortal: {
        name: 'Media Portal',
        ready: false,
        supported: false,
        message: 'Media portal support is unavailable in this client.',
      },
    });
  } finally {
    (globalThis as any).window = originalWindow;
  }
});

test('desktop agent actions sanitize unsafe URLs and omit invalid payloads', async () => {
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
    assert.equal(
      await sendDesktopAgentAction({
        id: 'unsafe-url',
        title: 'Open private',
        url: 'https://127.0.0.1',
      }),
      false
    );

    assert.equal(
      await sendDesktopAgentAction({
        id: 'unsupported-kind',
        title: 'Execute',
        kind: 'shell',
        prompt: 'rm -rf /',
      }),
      false
    );

    const command = await sendDesktopAgentAction({
      id: '',
      title: 'Bad action',
      url: 'https://example.org',
    });
    assert.equal(command, false);

    assert.equal(
      await sendDesktopAgentAction({
        id: 'valid',
        title: 'Open safe',
        url: 'https://example.org',
      }),
      true
    );
  } finally {
    (globalThis as any).window = originalWindow;
  }

  assert.equal(calls.length, 1);
  assert.equal(calls[0]?.command, 'desktop_agent_action');
  const safeAction = calls[0]?.args as DesktopActionCallArgs | undefined;
  assert.equal(safeAction?.action?.id, 'valid');
  assert.equal(safeAction?.action?.title, 'Open safe');
  assert.equal(safeAction?.action?.url, 'https://example.org');
});

test('desktop external link opener invokes the desktop bridge only on desktop', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const originalWindow = globalThis.window;

  try {
    (globalThis as any).window = {};
    assert.equal(await openDesktopExternalUrl('https://example.org'), false);

    (globalThis as any).window = {
      __SYNARA_DESKTOP__: {
        platform: 'tauri',
        invoke: async (command: string, args?: Record<string, unknown>) => {
          calls.push({ command, args });
          return true;
        },
      },
    };

    assert.equal(await openDesktopExternalUrl('https://example.org/path'), true);
    assert.equal(await openDesktopExternalUrl('https://192.168.1.1/admin'), false);
    assert.equal(await openDesktopExternalUrl('https://169.254.169.254/latest/meta-data/'), false);
  } finally {
    (globalThis as any).window = originalWindow;
  }

  assert.deepEqual(calls, [
    {
      command: 'desktop_open_external_url',
      args: { url: 'https://example.org/path' },
    },
  ]);
});

test('desktop notifications trim text and reject external routes', async () => {
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
    assert.equal(
      await showDesktopNotification({
        title: '  Reminder  ',
        body: '  Review Later item.  ',
        route: 'https://example.org/private',
      }),
      true
    );
    assert.equal(
      await showDesktopNotification({
        title: 'Room',
        route: '/room/!room:example.org',
      }),
      true
    );
  } finally {
    (globalThis as any).window = originalWindow;
  }

  assert.deepEqual(calls, [
    {
      command: 'desktop_notify',
      args: {
        notification: {
          title: 'Reminder',
          body: 'Review Later item.',
          route: undefined,
        },
      },
    },
    {
      command: 'desktop_notify',
      args: {
        notification: {
          title: 'Room',
          body: undefined,
          route: '/room/!room:example.org',
        },
      },
    },
  ]);
});

test('desktop notification room routes encode room and event anchors', () => {
  assert.equal(
    buildDesktopNotificationRoomRoute('!room:example.org', '$event:example.org'),
    '/home/!room%3Aexample.org/%24event%3Aexample.org'
  );
});

test('desktop notification payloads include routes for message, later, and agent approvals', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const originalWindow = globalThis.window;
  const roomId = '!room:example.org';
  const eventId = '$event:example.org';
  const route = buildDesktopNotificationRoomRoute(roomId, eventId);

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
    await showDesktopNotification({
      title: 'Room',
      body: 'New inbox notification from Alice',
      route,
    });
    await showDesktopNotification({
      title: 'Reminder',
      body: 'A saved reminder is due.',
      route,
    });
    await showDesktopNotification({
      title: 'Approve command',
      body: 'Room: Run `npm test`',
      route,
    });
  } finally {
    (globalThis as any).window = originalWindow;
  }

  assert.deepEqual(calls, [
    {
      command: 'desktop_notify',
      args: {
        notification: {
          title: 'Room',
          body: 'New inbox notification from Alice',
          route,
        },
      },
    },
    {
      command: 'desktop_notify',
      args: {
        notification: {
          title: 'Reminder',
          body: 'A saved reminder is due.',
          route,
        },
      },
    },
    {
      command: 'desktop_notify',
      args: {
        notification: {
          title: 'Approve command',
          body: 'Room: Run `npm test`',
          route,
        },
      },
    },
  ]);
});

test('desktop file streaming threshold uses eight mebibytes', () => {
  assert.equal(DESKTOP_FILE_IPC_INLINE_THRESHOLD, 8 * 1024 * 1024);
  assert.equal(DESKTOP_FILE_IPC_CHUNK_SIZE, 1024 * 1024);
  assert.equal(shouldStreamDesktopFileIpc(DESKTOP_FILE_IPC_INLINE_THRESHOLD), false);
  assert.equal(shouldStreamDesktopFileIpc(DESKTOP_FILE_IPC_INLINE_THRESHOLD + 1), true);
});

test('desktop file save sends bytes and filename through the desktop bridge', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const originalWindow = globalThis.window;

  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      invoke: async (command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        return '/Users/example/Downloads/report.zip';
      },
    },
  };

  try {
    assert.equal(
      await saveDesktopFile(new Blob([new Uint8Array([80, 75])]), 'report.zip'),
      '/Users/example/Downloads/report.zip'
    );
  } finally {
    (globalThis as any).window = originalWindow;
  }

  assert.deepEqual(calls, [
    {
      command: 'desktop_save_file',
      args: {
        payload: {
          filename: 'report.zip',
          bytes: [80, 75],
        },
      },
    },
  ]);
});

test('desktop file save streams large blobs through begin chunk end commands', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const originalWindow = globalThis.window;
  const totalSize = DESKTOP_FILE_IPC_INLINE_THRESHOLD + 3;
  const blob = new Blob([new Uint8Array(totalSize).fill(42)]);

  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      invoke: async (command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        if (command === 'desktop_save_file_begin') {
          return { sessionId: 'save-session-1' };
        }
        if (command === 'desktop_save_file_end') {
          return '/Users/example/Downloads/large.bin';
        }
        return true;
      },
    },
  };

  try {
    assert.equal(await saveDesktopFile(blob, 'large.bin'), '/Users/example/Downloads/large.bin');
  } finally {
    (globalThis as any).window = originalWindow;
  }

  assert.equal(calls[0]?.command, 'desktop_save_file_begin');
  assert.deepEqual(calls[0]?.args, {
    filename: 'large.bin',
    totalSize,
  });
  assert.equal(calls.at(-1)?.command, 'desktop_save_file_end');
  assert.deepEqual(calls.at(-1)?.args, { sessionId: 'save-session-1' });
  assert.equal(calls.filter((call) => call.command === 'desktop_save_file_chunk').length, 9);
  assert.equal(
    calls.some((call) => call.command === 'desktop_save_file'),
    false
  );
});

test('desktop dropped file read streams large transfers through chunk commands', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const originalWindow = globalThis.window;
  const totalSize = DESKTOP_FILE_IPC_INLINE_THRESHOLD + 2;

  (globalThis as any).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      invoke: async (command: string, args?: Record<string, unknown>) => {
        calls.push({ command, args });
        if (command === 'desktop_read_dropped_files') {
          return [
            {
              name: 'large-drop.bin',
              transferId: 'drop-transfer-1',
              size: totalSize,
            },
          ];
        }
        if (command === 'desktop_read_dropped_file_chunk') {
          const offset = Number(args?.offset ?? 0);
          const length = Number(args?.length ?? 0);
          if (offset === 0) {
            return Array.from({ length }, () => 7);
          }
          return Array.from({ length }, () => 8);
        }
        return true;
      },
    },
  };

  try {
    const files = await readDesktopDroppedFiles(['/tmp/large-drop.bin']);
    assert.equal(files.length, 1);
    assert.equal(files[0]?.name, 'large-drop.bin');
    assert.equal(files[0]?.size, totalSize);
    const content = new Uint8Array(await files[0]!.arrayBuffer());
    assert.equal(content[0], 7);
    assert.equal(content[DESKTOP_FILE_IPC_INLINE_THRESHOLD], 8);
    assert.equal(content.at(-1), 8);
  } finally {
    (globalThis as any).window = originalWindow;
  }

  assert.deepEqual(calls[0], {
    command: 'desktop_read_dropped_files',
    args: { paths: ['/tmp/large-drop.bin'] },
  });
  assert.equal(
    calls.filter((call) => call.command === 'desktop_read_dropped_file_chunk').length,
    9
  );
  assert.deepEqual(calls.at(-1), {
    command: 'desktop_read_dropped_file_end',
    args: { transferId: 'drop-transfer-1' },
  });
});

const createWindowEventTarget = () => {
  const listeners = new Map<string, Set<() => void>>();

  return {
    addEventListener(type: string, listener: () => void) {
      const handlers = listeners.get(type) ?? new Set<() => void>();
      handlers.add(listener);
      listeners.set(type, handlers);
    },
    removeEventListener(type: string, listener: () => void) {
      listeners.get(type)?.delete(listener);
    },
    dispatchEvent(event: { type: string }) {
      listeners.get(event.type)?.forEach((listener) => listener());
      return true;
    },
  };
};

test('desktop tray DND toggle subscription flips handler on custom event', () => {
  let toggles = 0;
  const originalWindow = globalThis.window;
  (globalThis as any).window = createWindowEventTarget();

  const unsubscribe = subscribeDesktopTrayDndToggle(() => {
    toggles += 1;
  });

  window.dispatchEvent(new CustomEvent(DESKTOP_TRAY_DND_TOGGLE_EVENT));
  assert.equal(toggles, 1);

  unsubscribe();
  window.dispatchEvent(new CustomEvent(DESKTOP_TRAY_DND_TOGGLE_EVENT));
  assert.equal(toggles, 1);

  (globalThis as any).window = originalWindow;
});

test('desktop tray DND toggle maps to inverted showNotifications state', () => {
  let showNotifications = true;
  const originalWindow = globalThis.window;
  (globalThis as any).window = createWindowEventTarget();

  const unsubscribe = subscribeDesktopTrayDndToggle(() => {
    showNotifications = !showNotifications;
  });

  window.dispatchEvent(new CustomEvent(DESKTOP_TRAY_DND_TOGGLE_EVENT));
  assert.equal(showNotifications, false);

  window.dispatchEvent(new CustomEvent(DESKTOP_TRAY_DND_TOGGLE_EVENT));
  assert.equal(showNotifications, true);

  unsubscribe();
  (globalThis as any).window = originalWindow;
});

test('desktop performance capabilities fall back on the web', async () => {
  const originalWindow = globalThis.window;
  (globalThis as any).window = {};

  try {
    assert.deepEqual(await getDesktopPerformanceCapabilities(), {
      platform: 'web',
    });
  } finally {
    (globalThis as any).window = originalWindow;
  }
});
