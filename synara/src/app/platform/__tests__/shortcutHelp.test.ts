import assert from 'node:assert/strict';
import test from 'node:test';

import {
  BASELINE_SHORTCUT_HELP,
  GNOME_WAYLAND_SHORTCUT_HELP,
  KDE_WAYLAND_SHORTCUT_HELP,
  LINUX_X11_SHORTCUT_HELP,
  MACOS_SHORTCUT_HELP,
  buildShortcutFailureMessage,
  selectShortcutPermissionHelp,
} from '../shortcutHelp';
import type { PlatformIntegrationStatus } from '../diagnostics';

const baseStatus: PlatformIntegrationStatus = {
  platform: 'linux',
  desktopEnvironment: 'unknown',
  sessionType: 'unknown',
  distroId: 'unknown',
  distroName: 'Unknown',
  distroVersion: 'unknown',
  buildIdentity: 'test',
  tray: { name: 'Tray', ready: true, supported: true, message: 'Ready' },
  notifications: { name: 'Notifications', ready: true, supported: true, message: 'Ready' },
  globalShortcuts: { name: 'Global Shortcuts', ready: false, supported: true, message: 'Pending' },
  filePortal: { name: 'File Portal', ready: true, supported: true, message: 'Ready' },
  mediaPortal: { name: 'Media Portal', ready: true, supported: true, message: 'Ready' },
};

test('shortcut help selects macOS guidance without KDE strings', () => {
  const status = { ...baseStatus, platform: 'darwin', sessionType: 'unknown' };
  assert.equal(selectShortcutPermissionHelp(status), MACOS_SHORTCUT_HELP);
  assert.ok(!selectShortcutPermissionHelp(status).toLowerCase().includes('kde'));
});

test('shortcut help selects KDE Wayland guidance only for KDE Wayland', () => {
  const kdeWayland = {
    ...baseStatus,
    desktopEnvironment: 'KDE Plasma Wayland',
    sessionType: 'wayland',
  };
  assert.equal(selectShortcutPermissionHelp(kdeWayland), KDE_WAYLAND_SHORTCUT_HELP);

  const gnomeWayland = {
    ...baseStatus,
    desktopEnvironment: 'GNOME',
    sessionType: 'wayland',
  };
  assert.equal(selectShortcutPermissionHelp(gnomeWayland), GNOME_WAYLAND_SHORTCUT_HELP);
  assert.ok(
    !selectShortcutPermissionHelp(gnomeWayland).toLowerCase().includes('kde plasma wayland')
  );
});

test('shortcut help selects Linux X11 guidance for X11 sessions', () => {
  const status = {
    ...baseStatus,
    desktopEnvironment: 'KDE',
    sessionType: 'x11',
  };
  assert.equal(selectShortcutPermissionHelp(status), LINUX_X11_SHORTCUT_HELP);
});

test('buildShortcutFailureMessage appends platform-specific helper text', () => {
  const message = buildShortcutFailureMessage(
    {
      success: false,
      state: 'permission-needed',
      message: 'Shortcut registration needs permission on this desktop session.',
    },
    {
      ...baseStatus,
      platform: 'darwin',
    }
  );

  assert.ok(message.includes('Shortcut registration needs permission'));
  assert.ok(message.includes('macOS'));
  assert.ok(!message.toLowerCase().includes('kde plasma wayland'));
});

test('shortcut help falls back to baseline for unknown platforms', () => {
  assert.equal(
    selectShortcutPermissionHelp({ ...baseStatus, platform: 'windows', sessionType: 'unknown' }),
    BASELINE_SHORTCUT_HELP
  );
});
