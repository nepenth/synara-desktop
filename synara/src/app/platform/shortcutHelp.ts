import type { PlatformIntegrationStatus } from './diagnostics';
import type { PlatformShortcutApplyResult } from './shortcuts';

export const BASELINE_SHORTCUT_HELP =
  'Check the session permissions for global shortcuts and try again.';

export const MACOS_SHORTCUT_HELP =
  'On macOS, grant Synara Input Monitoring permission in System Settings > Privacy & Security.';

export const KDE_WAYLAND_SHORTCUT_HELP =
  'On KDE Plasma Wayland, global shortcut capture can require manual registration in System Settings > Shortcuts.';

export const GNOME_WAYLAND_SHORTCUT_HELP =
  'On GNOME Wayland, global shortcuts may require portal or compositor permission. Check Settings > Keyboard > Keyboard Shortcuts.';

export const LINUX_WAYLAND_SHORTCUT_HELP =
  'On Wayland sessions, global shortcuts may require portal or compositor permission. Check your desktop environment shortcut settings.';

export const LINUX_X11_SHORTCUT_HELP =
  'On Linux X11, verify no other application has claimed the shortcut and check your desktop environment shortcut settings.';

export function isKdeWaylandStatus(status: PlatformIntegrationStatus): boolean {
  return (
    status.desktopEnvironment.toLowerCase().includes('kde') &&
    status.sessionType.toLowerCase().includes('wayland')
  );
}

export function selectShortcutPermissionHelp(status: PlatformIntegrationStatus): string {
  const platform = status.platform.toLowerCase();
  if (platform === 'darwin' || platform === 'macos') {
    return MACOS_SHORTCUT_HELP;
  }

  if (isKdeWaylandStatus(status)) {
    return KDE_WAYLAND_SHORTCUT_HELP;
  }

  const sessionType = status.sessionType.toLowerCase();
  const desktopEnvironment = status.desktopEnvironment.toLowerCase();
  if (sessionType.includes('wayland')) {
    return desktopEnvironment.includes('gnome')
      ? GNOME_WAYLAND_SHORTCUT_HELP
      : LINUX_WAYLAND_SHORTCUT_HELP;
  }

  if (sessionType.includes('x11') || platform === 'linux') {
    return LINUX_X11_SHORTCUT_HELP;
  }

  return BASELINE_SHORTCUT_HELP;
}

export function buildShortcutFailureMessage(
  result: PlatformShortcutApplyResult,
  status: PlatformIntegrationStatus
): string {
  const helper = selectShortcutPermissionHelp(status);
  return [result.message, result.fallbackCommand, helper]
    .filter((value): value is string => typeof value === 'string' && value.length > 0)
    .join(' ');
}
