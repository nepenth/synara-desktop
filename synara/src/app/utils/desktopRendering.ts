import { invokeDesktopWithAvailability, isSynaraDesktop } from './desktop';

/**
 * Contract with the shell (`src-tauri/src/desktop_rendering_host.rs`):
 * - Command `desktop_rendering_mode` → `{ available: boolean, saved: Mode |
 *   null, active: Mode, overriddenByEnvironment: boolean, fallback: { mode:
 *   Mode, reason: 'web-view-crashed' | 'start-failed' } | null }` where Mode
 *   is `'safe' | 'balanced' | 'gpu'`.
 *   `available` is true only on Linux under Wayland. `saved` is the user's
 *   choice for the next start, `null` while they have not chosen and the
 *   package's default applies. `active` is what this process started with,
 *   and `overriddenByEnvironment` means the `SYNARA_RENDERING` environment
 *   variable decided instead of the saved choice. `fallback` is set while
 *   starts draw safely because `mode` failed: the web view crashed, or a
 *   start ended before the window had drawn.
 * - Command `desktop_set_rendering_mode { mode }` saves the choice, clears
 *   the fallback and returns the same shape. It rejects where the setting is
 *   not available.
 * - Command `desktop_rendering_started` tells the shell the app has drawn, so
 *   this start did not fail.
 */
export const DESKTOP_RENDERING_MODE_COMMAND = 'desktop_rendering_mode';
export const DESKTOP_SET_RENDERING_MODE_COMMAND = 'desktop_set_rendering_mode';
export const DESKTOP_RENDERING_STARTED_COMMAND = 'desktop_rendering_started';

/**
 * `safe` draws everything on the CPU; `balanced` composites on the graphics
 * card without DMABUF presentation; `gpu` uses the graphics card fully.
 */
export type DesktopRenderingMode = 'safe' | 'balanced' | 'gpu';

/** The mode that failed, and how; starts draw safely until a mode is chosen. */
export type DesktopRenderingFallback = {
  mode: DesktopRenderingMode;
  reason: 'web-view-crashed' | 'start-failed';
};

export type DesktopRenderingStatus = {
  available: boolean;
  saved: DesktopRenderingMode | null;
  active: DesktopRenderingMode;
  overriddenByEnvironment: boolean;
  fallback: DesktopRenderingFallback | null;
};

export const DESKTOP_RENDERING_UNAVAILABLE: DesktopRenderingStatus = {
  available: false,
  saved: null,
  active: 'safe',
  overriddenByEnvironment: false,
  fallback: null,
};

const normalizeMode = (value: unknown): DesktopRenderingMode =>
  value === 'gpu' || value === 'balanced' ? value : 'safe';
const normalizeSavedMode = (value: unknown): DesktopRenderingMode | null =>
  value === null || value === undefined ? null : normalizeMode(value);
const normalizeFallback = (value: unknown): DesktopRenderingFallback | null => {
  if (typeof value !== 'object' || value === null) return null;
  const fallback = value as Record<string, unknown>;
  return {
    mode: normalizeMode(fallback.mode),
    reason: fallback.reason === 'web-view-crashed' ? 'web-view-crashed' : 'start-failed',
  };
};

/** Anything but a well-formed, available answer is "not available". */
export const normalizeDesktopRenderingStatus = (value: unknown): DesktopRenderingStatus => {
  if (typeof value !== 'object' || value === null) return DESKTOP_RENDERING_UNAVAILABLE;
  const status = value as Record<string, unknown>;
  if (status.available !== true) return DESKTOP_RENDERING_UNAVAILABLE;
  return {
    available: true,
    saved: normalizeSavedMode(status.saved),
    active: normalizeMode(status.active),
    overriddenByEnvironment: status.overriddenByEnvironment === true,
    fallback: normalizeFallback(status.fallback),
  };
};

export type HardwareAccelerationView = {
  /**
   * The selected mode: the saved choice, or what is running when the
   * environment decides or nothing is saved.
   */
  mode: DesktopRenderingMode;
  /** The environment variable decides, so the choice cannot change anything. */
  locked: boolean;
  /** A saved choice differs from what this process started with. */
  restartNeeded: boolean;
  /** Synara switched to safe drawing after a failure; null otherwise. */
  fallback: DesktopRenderingFallback | null;
};

export const hardwareAccelerationView = (
  status: DesktopRenderingStatus
): HardwareAccelerationView => {
  if (status.overriddenByEnvironment) {
    return { mode: status.active, locked: true, restartNeeded: false, fallback: null };
  }
  // Starts stay safe until a mode is chosen, so the safe start is the choice.
  if (status.fallback) {
    return { mode: status.active, locked: false, restartNeeded: false, fallback: status.fallback };
  }
  const shown = status.saved ?? status.active;
  return { mode: shown, locked: false, restartNeeded: shown !== status.active, fallback: null };
};

const FALLBACK_MODE_LABELS: Record<DesktopRenderingMode, string> = {
  safe: 'Off',
  balanced: 'Balanced',
  gpu: 'Full',
};

/** Why Synara turned hardware acceleration off, for the Settings tile. */
export const hardwareAccelerationFallbackNote = (fallback: DesktopRenderingFallback): string => {
  const what =
    fallback.reason === 'web-view-crashed' ? 'the display crashed' : 'it closed while starting';
  return `Synara switched to Off because ${what} in ${
    FALLBACK_MODE_LABELS[fallback.mode]
  } mode. Choose a mode to try again.`;
};

/** The shell's rendering setting; "not available" off Linux Wayland. */
export const getDesktopRenderingStatus = async (): Promise<DesktopRenderingStatus> => {
  if (!isSynaraDesktop()) return DESKTOP_RENDERING_UNAVAILABLE;
  try {
    // An older shell has no such command; that is the normal "not available"
    // answer, so it is not logged as a failure.
    const result = await invokeDesktopWithAvailability<unknown>(
      DESKTOP_RENDERING_MODE_COMMAND,
      undefined,
      { suppressErrorDiagnostic: true }
    );
    return result.available
      ? normalizeDesktopRenderingStatus(result.value)
      : DESKTOP_RENDERING_UNAVAILABLE;
  } catch {
    return DESKTOP_RENDERING_UNAVAILABLE;
  }
};

/**
 * Tells the shell, once the app shell has drawn its first frames, that this
 * start did not fail, so the next start keeps the mode. Two animation frames
 * mean frames went through the compositor and the display; a hidden window
 * gets none, so a timer reports anyway after a while (the shell exits within
 * moments when drawing fails). Returns a cancel function for effect cleanup.
 */
export const reportDesktopRenderingStarted = (fallbackDelayMs = 10_000): (() => void) => {
  if (!isSynaraDesktop()) return () => undefined;
  let done = false;
  let frame = 0;
  let timer = 0;
  const report = () => {
    if (done) return;
    done = true;
    cancelAnimationFrame(frame);
    window.clearTimeout(timer);
    // An older shell has no such command; there is nothing to report then.
    void invokeDesktopWithAvailability(DESKTOP_RENDERING_STARTED_COMMAND, undefined, {
      suppressErrorDiagnostic: true,
    }).catch(() => undefined);
  };
  timer = window.setTimeout(report, fallbackDelayMs);
  frame = requestAnimationFrame(() => {
    frame = requestAnimationFrame(report);
  });
  return () => {
    done = true;
    cancelAnimationFrame(frame);
    window.clearTimeout(timer);
  };
};

/**
 * Saves the choice for the next start. Resolves to the new status, or
 * `undefined` when the shell could not save it.
 */
export const setDesktopRenderingMode = async (
  mode: DesktopRenderingMode
): Promise<DesktopRenderingStatus | undefined> => {
  if (!isSynaraDesktop()) return undefined;
  try {
    const result = await invokeDesktopWithAvailability<unknown>(
      DESKTOP_SET_RENDERING_MODE_COMMAND,
      { mode }
    );
    if (!result.available) return undefined;
    const status = normalizeDesktopRenderingStatus(result.value);
    return status.available ? status : undefined;
  } catch {
    return undefined;
  }
};
