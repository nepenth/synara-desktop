import { invokeDesktopWithAvailability, isSynaraDesktop } from './desktop';

/**
 * Contract with the shell (`src-tauri/src/desktop_rendering_host.rs`):
 * - Command `desktop_rendering_mode` → `{ available: boolean, saved: Mode |
 *   null, active: Mode, overriddenByEnvironment: boolean }` where Mode is
 *   `'safe' | 'balanced' | 'gpu'`.
 *   `available` is true only on Linux under Wayland. `saved` is the user's
 *   choice for the next start, `null` while they have not chosen and the
 *   package's default applies. `active` is what this process started with,
 *   and `overriddenByEnvironment` means the `SYNARA_RENDERING` environment
 *   variable decided instead of the saved choice.
 * - Command `desktop_set_rendering_mode { mode }` saves the choice and
 *   returns the same shape. It rejects where the setting is not available.
 */
export const DESKTOP_RENDERING_MODE_COMMAND = 'desktop_rendering_mode';
export const DESKTOP_SET_RENDERING_MODE_COMMAND = 'desktop_set_rendering_mode';

/**
 * `safe` draws everything on the CPU; `balanced` composites on the graphics
 * card without DMABUF presentation; `gpu` uses the graphics card fully.
 */
export type DesktopRenderingMode = 'safe' | 'balanced' | 'gpu';

export type DesktopRenderingStatus = {
  available: boolean;
  saved: DesktopRenderingMode | null;
  active: DesktopRenderingMode;
  overriddenByEnvironment: boolean;
};

export const DESKTOP_RENDERING_UNAVAILABLE: DesktopRenderingStatus = {
  available: false,
  saved: null,
  active: 'safe',
  overriddenByEnvironment: false,
};

const normalizeMode = (value: unknown): DesktopRenderingMode =>
  value === 'gpu' || value === 'balanced' ? value : 'safe';
const normalizeSavedMode = (value: unknown): DesktopRenderingMode | null =>
  value === null || value === undefined ? null : normalizeMode(value);

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
};

export const hardwareAccelerationView = (
  status: DesktopRenderingStatus
): HardwareAccelerationView => {
  if (status.overriddenByEnvironment) {
    return { mode: status.active, locked: true, restartNeeded: false };
  }
  const shown = status.saved ?? status.active;
  return { mode: shown, locked: false, restartNeeded: shown !== status.active };
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
