import { getBadgeCount } from '../notifications/badgeSummary';
import { recordDesktopDiagnostic } from './desktopDiagnostics';
import { getHomeRoomPath } from '../pages/pathUtils';
import {
  normalizeAgentActionPayload,
  type AgentActionPayload,
  type NormalizedAgentActionPayload,
} from '../agents/agentActions';

type TauriInternals = {
  invoke?: <T = unknown>(command: string, args?: Record<string, unknown>) => Promise<T>;
  convertFileSrc?: (filePath: string, protocol?: string) => string;
  transformCallback?: <T>(callback: (response: T) => void, once?: boolean) => number;
};

type SynaraDesktopBridge = {
  platform: 'tauri';
  supportsTray?: boolean;
  supportsGlobalShortcuts?: boolean;
  supportsIntegrationStatus?: boolean;
  supportsTrayState?: boolean;
  desktopEnvironment?: string;
  sessionType?: string;
  supportsUpdater?: boolean;
  supportsMediaPermissions?: boolean;
  supportsSecureSecretStore?: boolean;
  supportsSpellcheck?: boolean;
  invoke?: <T = unknown>(command: string, args?: Record<string, unknown>) => Promise<T>;
  routes?: {
    later?: string;
    notifications?: string;
    settings?: string;
  };
};

const MAX_DESKTOP_ACTION_URL_LENGTH = 2_048;
const MAX_DESKTOP_ROUTE_LENGTH = 2_048;
const FILE_MIME_BY_EXTENSION: Record<string, string> = {
  avif: 'image/avif',
  gif: 'image/gif',
  heic: 'image/heic',
  heif: 'image/heif',
  jpeg: 'image/jpeg',
  jpg: 'image/jpeg',
  md: 'text/markdown',
  markdown: 'text/markdown',
  mov: 'video/quicktime',
  mp3: 'audio/mpeg',
  mp4: 'video/mp4',
  pdf: 'application/pdf',
  png: 'image/png',
  svg: 'image/svg+xml',
  txt: 'text/plain',
  wav: 'audio/wav',
  webm: 'video/webm',
  webp: 'image/webp',
};

export type DesktopShortcutConfig = {
  show: string;
  later: string;
  notifications: string;
};

export type DesktopIntegrationCheck = {
  name: string;
  ready: boolean;
  supported: boolean;
  message: string;
};

export type DesktopIntegrationStatus = {
  platform: string;
  desktopEnvironment: string;
  sessionType: string;
  distroId: string;
  distroName: string;
  distroVersion: string;
  buildIdentity: string;
  tray: DesktopIntegrationCheck;
  notifications: DesktopIntegrationCheck;
  globalShortcuts: DesktopIntegrationCheck;
  filePortal: DesktopIntegrationCheck;
  mediaPortal: DesktopIntegrationCheck;
};

export type DesktopShortcutApplyState =
  | 'active'
  | 'permission-needed'
  | 'unsupported'
  | 'unknown'
  | 'failed';

export type DesktopShortcutApplyResult = {
  success: boolean;
  state: DesktopShortcutApplyState;
  message: string;
  fallbackCommand?: string;
};

export type DesktopTrayState = {
  unreadCount: number;
  highlightCount: number;
  laterCount: number;
  notificationInboxCount: number;
  doNotDisturb: boolean;
};

export const DESKTOP_TRAY_STATE_DEBOUNCE_MS = 500;

export type DesktopTrayStateUpdater = (state: DesktopTrayState) => Promise<boolean>;

export type DebouncedDesktopTrayStateUpdater = DesktopTrayStateUpdater & {
  flush: () => Promise<boolean | undefined>;
  cancel: () => void;
};

type DesktopTrayStateDebounceScheduler = {
  schedule: (callback: () => void, delayMs: number) => number;
  cancel: (handle: number) => void;
};

const scheduleTimeout = (callback: () => void, delayMs: number): number => {
  if (typeof window !== 'undefined' && typeof window.setTimeout === 'function') {
    return window.setTimeout(callback, delayMs);
  }
  return setTimeout(callback, delayMs) as unknown as number;
};

const cancelTimeout = (handle: number): void => {
  if (typeof window !== 'undefined' && typeof window.clearTimeout === 'function') {
    window.clearTimeout(handle);
    return;
  }
  clearTimeout(handle);
};

const defaultTrayStateDebounceScheduler: DesktopTrayStateDebounceScheduler = {
  schedule: scheduleTimeout,
  cancel: cancelTimeout,
};

export const createDebouncedTrayStateUpdater = (
  updater: DesktopTrayStateUpdater,
  waitMs = DESKTOP_TRAY_STATE_DEBOUNCE_MS,
  scheduler: DesktopTrayStateDebounceScheduler = defaultTrayStateDebounceScheduler
): DebouncedDesktopTrayStateUpdater => {
  let timeoutId: number | undefined;
  let pendingState: DesktopTrayState | undefined;
  let flushResolvers: Array<(result: boolean | undefined) => void> = [];

  const settleFlushWaiters = (result: boolean | undefined) => {
    const resolvers = flushResolvers;
    flushResolvers = [];
    resolvers.forEach((resolve) => resolve(result));
  };

  const invokePendingState = async (): Promise<boolean | undefined> => {
    timeoutId = undefined;
    const stateToSend = pendingState;
    pendingState = undefined;
    if (!stateToSend) {
      settleFlushWaiters(undefined);
      return undefined;
    }

    const result = await updater(stateToSend);
    settleFlushWaiters(result);
    return result;
  };

  const debounced: DebouncedDesktopTrayStateUpdater = async (state) => {
    pendingState = state;
    if (timeoutId !== undefined) {
      scheduler.cancel(timeoutId);
    }

    return new Promise<boolean>((resolve) => {
      flushResolvers.push((result) => resolve(result === true));
      timeoutId = scheduler.schedule(() => {
        void invokePendingState();
      }, waitMs);
    });
  };

  debounced.flush = async () => {
    if (timeoutId !== undefined) {
      scheduler.cancel(timeoutId);
      timeoutId = undefined;
    }
    return invokePendingState();
  };

  debounced.cancel = () => {
    if (timeoutId !== undefined) {
      scheduler.cancel(timeoutId);
      timeoutId = undefined;
    }
    pendingState = undefined;
    settleFlushWaiters(undefined);
  };

  return debounced;
};

export type DesktopPerformanceCapabilities = {
  platform: string;
  appVersion?: string;
  buildRevision?: string;
  buildBranch?: string;
  buildLabel?: string;
  fps?: number;
  memoryUsageBytes?: number;
  webviewEngine?: string;
  hardwareAccelerationPolicy?: string;
  smoothScrollingEnabled?: boolean;
  softwareRenderingOverrideDetected?: boolean;
  dmabufFastPathDisabled?: boolean;
};

export type DesktopNotificationPermission =
  | 'granted'
  | 'denied'
  | 'prompt'
  | 'prompt-with-rationale';

export type DesktopNotificationAction = {
  id: string;
  label: string;
};

export type DesktopNotificationActionContext = {
  kind: string;
  roomId?: string;
  eventId?: string;
};

export type DesktopNotificationPayload = {
  title: string;
  body?: string;
  route?: string;
  actions?: DesktopNotificationAction[];
  actionContext?: DesktopNotificationActionContext;
};

export type DesktopNotificationActionEventPayload = {
  actionId: string;
  context?: DesktopNotificationActionContext;
};

export type DesktopNativeFileDropPayload = {
  phase: 'enter' | 'over' | 'drop' | 'leave';
  paths: string[];
  x: number;
  y: number;
};

export const DESKTOP_FILE_IPC_INLINE_THRESHOLD = 8 * 1024 * 1024;
export const DESKTOP_FILE_IPC_CHUNK_SIZE = 1024 * 1024;

type DesktopDroppedFilePayload = {
  name: string;
  bytes?: number[];
  transferId?: string;
  size?: number;
};

type DesktopSaveFileBeginResult = {
  sessionId: string;
};

type DesktopImageResourceSize = {
  width: number;
  height: number;
};

export const shouldStreamDesktopFileIpc = (byteLength: number): boolean =>
  byteLength > DESKTOP_FILE_IPC_INLINE_THRESHOLD;

declare global {
  interface Window {
    __SYNARA_DESKTOP__?: SynaraDesktopBridge;
    __TAURI_INTERNALS__?: TauriInternals;
    __TAURI_EVENT_PLUGIN_INTERNALS__: {
      unregisterListener: (event: string, eventId: number) => void;
    };
  }
}

export type DesktopAgentActionPayload = AgentActionPayload;

export type DesktopAgentActionEventPayload = {
  action: DesktopAgentActionPayload;
};

export type DesktopEvent<T> = {
  event: string;
  id: number;
  payload: T;
};

export type DesktopUnlisten = () => void | Promise<void>;

const normalizeActionField = (
  value: unknown,
  maxLength = MAX_DESKTOP_ACTION_URL_LENGTH
): string => {
  if (typeof value !== 'string') return '';
  const normalized = value.trim();
  return normalized.slice(0, maxLength);
};

const getBridge = (): SynaraDesktopBridge | undefined => window.__SYNARA_DESKTOP__;

const getDesktopInvoke = ():
  | (<T = unknown>(command: string, args?: Record<string, unknown>) => Promise<T>)
  | undefined => window.__SYNARA_DESKTOP__?.invoke ?? window.__TAURI_INTERNALS__?.invoke;

export const isDesktopBridgeAvailable = (): boolean => typeof getDesktopInvoke() === 'function';

const normalizeValue = (value: unknown): string => {
  if (typeof value !== 'string') return '';
  return value.trim();
};

const clampCount = (value: unknown): number => {
  if (typeof value !== 'number' || !Number.isFinite(value)) return 0;
  return Math.max(0, Math.floor(value));
};

export const sanitizeDesktopNotificationRoute = (value: unknown): string | undefined => {
  const normalized = normalizeActionField(value, MAX_DESKTOP_ROUTE_LENGTH);
  if (!normalized || normalized.includes('://')) return undefined;
  if (!normalized.startsWith('/') && !normalized.startsWith('#')) return undefined;
  return normalized;
};

export const buildDesktopNotificationRoomRoute = (roomId: string, eventId?: string): string =>
  getHomeRoomPath(roomId, eventId);

const toShortcutApplyState = (value: unknown): DesktopShortcutApplyState | undefined => {
  if (value === 'active') return 'active';
  if (value === 'permission-needed') return 'permission-needed';
  if (value === 'unsupported') return 'unsupported';
  if (value === 'unknown') return 'unknown';
  if (value === 'failed') return 'failed';
  return undefined;
};

const normalizeDesktopShortcutApplyResult = (result: unknown): DesktopShortcutApplyResult => {
  if (result === true) {
    return {
      success: true,
      state: 'active',
      message: 'Desktop shortcuts are active.',
    };
  }

  if (result && typeof result === 'object') {
    const candidate = result as Record<string, unknown>;
    const state = toShortcutApplyState(candidate.state);
    if (typeof candidate.success === 'boolean' && state && typeof candidate.message === 'string') {
      return {
        success: candidate.success,
        state,
        message: candidate.message,
        fallbackCommand:
          typeof candidate.fallbackCommand === 'string' ? candidate.fallbackCommand : undefined,
      };
    }
  }

  return {
    success: false,
    state: 'unsupported',
    message: 'Desktop shortcut registration is not available in this client.',
  };
};

const unavailableCheck = (name: string, message: string): DesktopIntegrationCheck => ({
  name,
  ready: false,
  supported: false,
  message,
});

const normalizeDesktopIntegrationStatus = (result: unknown): DesktopIntegrationStatus => {
  const readCheck = (
    value: unknown,
    fallbackName: string,
    fallbackMessage: string
  ): DesktopIntegrationCheck => {
    if (!value || typeof value !== 'object') {
      return unavailableCheck(fallbackName, fallbackMessage);
    }
    const candidate = value as Record<string, unknown>;
    if (
      typeof candidate.name === 'string' &&
      typeof candidate.ready === 'boolean' &&
      typeof candidate.supported === 'boolean' &&
      typeof candidate.message === 'string'
    ) {
      return {
        name: candidate.name,
        ready: candidate.ready,
        supported: candidate.supported,
        message: candidate.message,
      };
    }
    return unavailableCheck(fallbackName, fallbackMessage);
  };

  if (result && typeof result === 'object') {
    const candidate = result as Record<string, unknown>;
    return {
      platform: normalizeValue(candidate.platform) || 'web',
      desktopEnvironment: normalizeValue(candidate.desktopEnvironment) || 'unknown',
      sessionType: normalizeValue(candidate.sessionType) || 'unknown',
      distroId: normalizeValue(candidate.distroId) || 'unknown',
      distroName: normalizeValue(candidate.distroName) || 'unknown',
      distroVersion: normalizeValue(candidate.distroVersion) || 'unknown',
      buildIdentity: normalizeValue(candidate.buildIdentity) || 'unknown',
      tray: readCheck(candidate.tray, 'Tray', 'Tray support is unavailable in this client.'),
      notifications: readCheck(
        candidate.notifications,
        'Notifications',
        'Notification support is unavailable in this client.'
      ),
      globalShortcuts: readCheck(
        candidate.globalShortcuts,
        'Global Shortcuts',
        'Global shortcut support is unavailable in this client.'
      ),
      filePortal: readCheck(
        candidate.filePortal,
        'File Portal',
        'File portal support is unavailable in this client.'
      ),
      mediaPortal: readCheck(
        candidate.mediaPortal,
        'Media Portal',
        'Media portal support is unavailable in this client.'
      ),
    };
  }

  return {
    platform: getBridge()?.platform || 'web',
    desktopEnvironment: getBridge()?.desktopEnvironment || 'unknown',
    sessionType: getBridge()?.sessionType || 'unknown',
    distroId: 'unknown',
    distroName: 'unknown',
    distroVersion: 'unknown',
    buildIdentity: 'unknown',
    tray: unavailableCheck('Tray', 'Tray support is unavailable in this client.'),
    notifications: unavailableCheck(
      'Notifications',
      'Notification support is unavailable in this client.'
    ),
    globalShortcuts: unavailableCheck(
      'Global Shortcuts',
      'Global shortcut support is unavailable in this client.'
    ),
    filePortal: unavailableCheck(
      'File Portal',
      'File portal support is unavailable in this client.'
    ),
    mediaPortal: unavailableCheck(
      'Media Portal',
      'Media portal support is unavailable in this client.'
    ),
  };
};

export const isSynaraDesktop = (): boolean =>
  window.__SYNARA_DESKTOP__?.platform === 'tauri' ||
  typeof window.__TAURI_INTERNALS__?.invoke === 'function';

export type DesktopInvokeResult<T> =
  | { available: false }
  | { available: true; value: T | undefined };

export type DesktopInvokeOptions = {
  /**
   * Secret-bearing callers map their rejection to a static product diagnostic
   * themselves. This prevents a duplicate generic entry before that mapping.
   */
  suppressErrorDiagnostic?: boolean;
};

const SAFE_NATIVE_DIAGNOSTIC_IDS = new Set([
  'd0.4-send-sdk-http-failed',
  'd0.4-send-sdk-http-network-failed',
  'd0.4-send-sdk-http-request-failed',
  'd0.4-send-sdk-http-refresh-failed',
  'd0.4-send-sdk-http-forbidden',
  'd0.4-send-sdk-http-auth-failed',
  'd0.4-send-sdk-http-rate-limited',
  'd0.4-send-sdk-http-invalid-request',
  'd0.4-send-sdk-http-not-found',
  'd0.4-send-sdk-http-api-failed',
  'd0.4-send-sdk-auth-required',
  'd0.4-send-sdk-insufficient-data',
  'd0.4-send-sdk-crypto-store-state',
  'd0.4-send-sdk-no-olm-machine',
  'd0.4-send-sdk-crypto-store-failed',
  'd0.4-send-sdk-olm-failed',
  'd0.4-send-sdk-megolm-failed',
  'd0.4-send-sdk-state-store-failed',
  'd0.4-send-sdk-wrong-room-state',
  'd0.4-send-sdk-concurrent-request-failed',
  'd0.4-send-sdk-failed',
]);

/**
 * Tauri/native rejection values are untrusted: a server body, URL, credential,
 * or token can be embedded in any of their fields. Never log their contents.
 */
export const formatDesktopInvokeError = (error: unknown): string => {
  if (error && typeof error === 'object' && !Array.isArray(error)) {
    const diagnosticId = (error as Record<string, unknown>).diagnosticId;
    if (typeof diagnosticId === 'string' && SAFE_NATIVE_DIAGNOSTIC_IDS.has(diagnosticId)) {
      return `native command rejected (${diagnosticId})`;
    }
  }
  return 'native command rejected';
};

export const invokeDesktopWithAvailability = async <T = unknown>(
  command: string,
  args?: Record<string, unknown>,
  options: DesktopInvokeOptions = {}
): Promise<DesktopInvokeResult<T>> => {
  const invoke = getDesktopInvoke();
  if (!invoke) {
    return { available: false };
  }

  try {
    return { available: true, value: await invoke<T>(command, args) };
  } catch (error) {
    if (!options.suppressErrorDiagnostic) {
      recordDesktopDiagnostic(`${command} failed: ${formatDesktopInvokeError(error)}`);
    }
    throw error;
  }
};

export const invokeDesktop = async <T = unknown>(
  command: string,
  args?: Record<string, unknown>
): Promise<T | undefined> => {
  const result = await invokeDesktopWithAvailability<T>(command, args);
  if (!result.available) return undefined;
  return result.value;
};

export const convertDesktopFileSrc = (handle: string, protocol: string): string | undefined => {
  if (!handle || !isSynaraDesktop()) return undefined;
  return window.__TAURI_INTERNALS__?.convertFileSrc?.(handle, protocol);
};

export const enableDesktopSpellcheck = async (): Promise<boolean> => {
  if (!isDesktopBridgeAvailable() || getBridge()?.supportsSpellcheck !== true) {
    return false;
  }

  try {
    const result = await invokeDesktopWithAvailability<void>('desktop_enable_spellcheck');
    return result.available;
  } catch {
    return false;
  }
};

const recordDesktopInvokeFailure = (command: string, detail: string): void => {
  recordDesktopDiagnostic(`${command} ${detail}`);
};

export const listen = async <T>(
  event: string,
  handler: (event: DesktopEvent<T>) => void
): Promise<DesktopUnlisten | undefined> => {
  const internals = window.__TAURI_INTERNALS__;
  if (!internals?.invoke || !internals.transformCallback) return undefined;

  const eventId = await internals.invoke<number>('plugin:event|listen', {
    event,
    target: { kind: 'Any' },
    handler: internals.transformCallback(handler),
  });

  return async () => {
    window.__TAURI_EVENT_PLUGIN_INTERNALS__?.unregisterListener?.(event, eventId);
    await internals.invoke?.('plugin:event|unlisten', { event, eventId });
  };
};

export const setDesktopBadgeCount = async (count: number): Promise<void> => {
  if (!Number.isFinite(count)) return;
  await invokeDesktop('desktop_set_badge_count', {
    count: Math.max(0, Math.floor(count)),
  });
};

export const sendDesktopAgentAction = async (
  action: DesktopAgentActionPayload
): Promise<boolean> => {
  const safeAction: NormalizedAgentActionPayload | undefined = normalizeAgentActionPayload(action);
  if (!safeAction) return false;
  const result = await invokeDesktop<boolean>('desktop_agent_action', { action: safeAction });
  return result === true;
};

const isSafeDesktopExternalUrl = (url: string): boolean => {
  try {
    const parsed = new URL(url);
    if (parsed.username || parsed.password) {
      return false;
    }
    if (parsed.protocol === 'http:' || parsed.protocol === 'https:') {
      return !!parsed.hostname;
    }
    if (parsed.protocol === 'mailto:') {
      const address = parsed.pathname.trim();
      return address.length > 0 && address.includes('@');
    }
    if (parsed.protocol === 'matrix:') {
      return parsed.hostname.length > 0 || parsed.pathname.replace(/^\//, '').length > 0;
    }
  } catch {
    return false;
  }

  return false;
};

export const openDesktopExternalUrl = async (url: string): Promise<boolean> => {
  const normalizedUrl = normalizeActionField(url, MAX_DESKTOP_ACTION_URL_LENGTH);
  if (!normalizedUrl || !isSynaraDesktop()) {
    return false;
  }
  if (!isSafeDesktopExternalUrl(normalizedUrl)) {
    recordDesktopInvokeFailure('desktop_open_external_url', 'rejected unsafe URL');
    return false;
  }

  const result = await invokeDesktopWithAvailability<boolean>('desktop_open_external_url', {
    url: normalizedUrl,
  });
  if (!result.available) {
    recordDesktopInvokeFailure('desktop_open_external_url', 'bridge unavailable');
    return false;
  }
  if (result.value !== true) {
    recordDesktopInvokeFailure('desktop_open_external_url', 'returned false');
    return false;
  }
  return true;
};

// Tauri IPC currently serializes byte payloads as number[]; chunked transfers avoid
// a single giant buffer but still allocate per chunk in the renderer.
const saveDesktopFileInline = async (
  blob: Blob,
  safeFilename: string
): Promise<string | undefined> => {
  const bytes = Array.from(new Uint8Array(await blob.arrayBuffer()));
  if (bytes.length === 0) return undefined;

  return invokeDesktop<string>('desktop_save_file', {
    payload: {
      filename: safeFilename,
      bytes,
    },
  });
};

const saveDesktopFileStreamed = async (
  blob: Blob,
  safeFilename: string
): Promise<string | undefined> => {
  const totalSize = blob.size;
  const begin = await invokeDesktop<DesktopSaveFileBeginResult>('desktop_save_file_begin', {
    filename: safeFilename,
    totalSize,
  });
  if (!begin?.sessionId) return undefined;

  let offset = 0;
  try {
    while (offset < totalSize) {
      const chunkEnd = Math.min(offset + DESKTOP_FILE_IPC_CHUNK_SIZE, totalSize);
      const chunk = blob.slice(offset, chunkEnd);
      const bytes = Array.from(new Uint8Array(await chunk.arrayBuffer()));
      const accepted = await invokeDesktop<boolean>('desktop_save_file_chunk', {
        sessionId: begin.sessionId,
        offset,
        bytes,
      });
      if (accepted !== true) {
        throw new Error('Desktop save chunk was rejected');
      }
      offset = chunkEnd;
    }

    return invokeDesktop<string>('desktop_save_file_end', {
      sessionId: begin.sessionId,
    });
  } catch {
    await invokeDesktop('desktop_save_file_abort', { sessionId: begin.sessionId });
    return undefined;
  }
};

export const saveDesktopFile = async (
  blob: Blob,
  filename: string
): Promise<string | undefined> => {
  if (!isSynaraDesktop()) return undefined;
  const safeFilename = normalizeActionField(filename || 'download', 240) || 'download';
  if (blob.size === 0) return undefined;

  if (shouldStreamDesktopFileIpc(blob.size)) {
    return saveDesktopFileStreamed(blob, safeFilename);
  }

  return saveDesktopFileInline(blob, safeFilename);
};

const inferFileMime = (filename: string): string => {
  const extension = filename.split('.').pop()?.toLowerCase();
  if (!extension) return '';
  return FILE_MIME_BY_EXTENSION[extension] ?? '';
};

const readDesktopDroppedFileStreamed = async (
  file: DesktopDroppedFilePayload
): Promise<File | undefined> => {
  const transferId = file.transferId;
  const size = file.size;
  if (!transferId || typeof size !== 'number' || size <= 0) return undefined;

  const chunks: BlobPart[] = [];
  let offset = 0;
  while (offset < size) {
    const length = Math.min(DESKTOP_FILE_IPC_CHUNK_SIZE, size - offset);
    const chunk = await invokeDesktop<number[]>('desktop_read_dropped_file_chunk', {
      transferId,
      offset,
      length,
    });
    if (!chunk || chunk.length === 0) {
      await invokeDesktop('desktop_read_dropped_file_end', { transferId });
      return undefined;
    }
    chunks.push(new Uint8Array(chunk));
    offset += chunk.length;
  }

  if (offset !== size) {
    await invokeDesktop('desktop_read_dropped_file_end', { transferId });
    return undefined;
  }

  await invokeDesktop('desktop_read_dropped_file_end', { transferId });
  return new File(chunks, file.name, {
    type: inferFileMime(file.name),
  });
};

export const readDesktopDroppedFiles = async (paths: string[]): Promise<File[]> => {
  if (!isSynaraDesktop() || paths.length === 0) return [];
  const droppedFiles = await invokeDesktop<DesktopDroppedFilePayload[]>(
    'desktop_read_dropped_files',
    { paths }
  );
  if (!droppedFiles) return [];

  const files: File[] = [];
  for (const file of droppedFiles) {
    if (Array.isArray(file.bytes) && file.bytes.length > 0) {
      files.push(
        new File([new Uint8Array(file.bytes)], file.name, {
          type: inferFileMime(file.name),
        })
      );
      continue;
    }

    const streamed = await readDesktopDroppedFileStreamed(file);
    if (streamed) {
      files.push(streamed);
    }
  }

  return files;
};

const isDesktopImageResourceSize = (value: unknown): value is DesktopImageResourceSize => {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.width === 'number' &&
    Number.isInteger(candidate.width) &&
    candidate.width > 0 &&
    typeof candidate.height === 'number' &&
    Number.isInteger(candidate.height) &&
    candidate.height > 0
  );
};

const normalizeDesktopByteArray = (value: unknown): Uint8Array | undefined => {
  if (value instanceof Uint8Array) return value;
  if (!Array.isArray(value)) return undefined;
  if (!value.every((byte) => Number.isInteger(byte) && byte >= 0 && byte <= 255)) {
    return undefined;
  }
  return new Uint8Array(value);
};

const createPngBlobFromRgba = (
  rgba: Uint8Array,
  size: DesktopImageResourceSize
): Promise<Blob | undefined> =>
  new Promise((resolve) => {
    const expectedLength = size.width * size.height * 4;
    if (rgba.length !== expectedLength) {
      resolve(undefined);
      return;
    }

    const canvas = document.createElement('canvas');
    canvas.width = size.width;
    canvas.height = size.height;

    const context = canvas.getContext('2d');
    if (!context) {
      resolve(undefined);
      return;
    }

    const imageData = context.createImageData(size.width, size.height);
    imageData.data.set(rgba);
    context.putImageData(imageData, 0, 0);
    canvas.toBlob((blob) => resolve(blob ?? undefined), 'image/png');
  });

export const readDesktopClipboardImage = async (): Promise<File | undefined> => {
  if (!isSynaraDesktop()) return undefined;
  const invoke = getDesktopInvoke();
  if (!invoke) return undefined;

  let rid: number | undefined;
  try {
    const resourceId = await invoke<unknown>('plugin:clipboard-manager|read_image');
    if (typeof resourceId !== 'number') return undefined;
    rid = resourceId;

    const size = await invoke<unknown>('plugin:image|size', { rid });
    if (!isDesktopImageResourceSize(size)) return undefined;

    const rgba = normalizeDesktopByteArray(await invoke<unknown>('plugin:image|rgba', { rid }));
    if (!rgba) return undefined;

    const blob = await createPngBlobFromRgba(rgba, size);
    if (!blob) return undefined;

    return new File([blob], 'clipboard-image.png', { type: 'image/png' });
  } catch (error) {
    recordDesktopDiagnostic(
      `desktop clipboard image import failed: ${formatDesktopInvokeError(error)}`
    );
    return undefined;
  } finally {
    if (rid !== undefined) {
      try {
        await invoke('plugin:resources|close', { rid });
      } catch {
        // Resource cleanup is best-effort; a failed close should not block paste.
      }
    }
  }
};

export const readDesktopClipboardText = async (): Promise<string | undefined> => {
  if (!isSynaraDesktop()) return undefined;
  const invoke = getDesktopInvoke();
  if (!invoke) return undefined;

  try {
    const text = await invoke<unknown>('plugin:clipboard-manager|read_text');
    if (typeof text !== 'string' || text.length === 0) return undefined;
    return text;
  } catch (error) {
    recordDesktopDiagnostic(
      `desktop clipboard text import failed: ${formatDesktopInvokeError(error)}`
    );
    return undefined;
  }
};

export const setDesktopShortcuts = async (
  shortcuts: DesktopShortcutConfig
): Promise<DesktopShortcutApplyResult> => {
  if (getBridge()?.supportsGlobalShortcuts === false) {
    return normalizeDesktopShortcutApplyResult(undefined);
  }

  try {
    const invokeResult = await invokeDesktopWithAvailability<unknown>('desktop_set_shortcuts', {
      shortcuts,
    });
    if (!invokeResult.available) {
      return normalizeDesktopShortcutApplyResult(undefined);
    }
    const normalized = normalizeDesktopShortcutApplyResult(invokeResult.value);
    if (!normalized.success) {
      recordDesktopInvokeFailure('desktop_set_shortcuts', normalized.message);
    }
    return normalized;
  } catch {
    return normalizeDesktopShortcutApplyResult(undefined);
  }
};

const invokeDesktopTrayState = async (state: DesktopTrayState): Promise<boolean> => {
  try {
    const invokeResult = await invokeDesktopWithAvailability<unknown>('desktop_update_tray_state', {
      state: {
        unreadCount: clampCount(state.unreadCount),
        highlightCount: clampCount(state.highlightCount),
        laterCount: clampCount(state.laterCount),
        notificationInboxCount: clampCount(state.notificationInboxCount),
        doNotDisturb: state.doNotDisturb === true,
      },
    });
    if (!invokeResult.available) {
      return false;
    }
    if (invokeResult.value === true) {
      return true;
    }
    recordDesktopInvokeFailure(
      'desktop_update_tray_state',
      invokeResult.value === false ? 'returned false' : 'returned no result'
    );
    return false;
  } catch {
    return false;
  }
};

const debouncedSetDesktopTrayState = createDebouncedTrayStateUpdater(invokeDesktopTrayState);

export const setDesktopTrayState = async (state: DesktopTrayState): Promise<boolean> => {
  if (!isDesktopBridgeAvailable() || getBridge()?.supportsTrayState !== true) {
    return false;
  }
  return debouncedSetDesktopTrayState(state);
};

export const flushPendingDesktopTrayStateUpdate = (): Promise<boolean | undefined> =>
  debouncedSetDesktopTrayState.flush();

export const getDesktopIntegrationStatus = async (): Promise<DesktopIntegrationStatus> => {
  if (!getBridge()?.supportsIntegrationStatus) {
    return normalizeDesktopIntegrationStatus(undefined);
  }

  try {
    const result = await invokeDesktop<unknown>('desktop_get_integration_status');
    return normalizeDesktopIntegrationStatus(result);
  } catch {
    return normalizeDesktopIntegrationStatus(undefined);
  }
};

export const getDesktopPerformanceCapabilities =
  async (): Promise<DesktopPerformanceCapabilities> => {
    try {
      const result = await invokeDesktop<DesktopPerformanceCapabilities>(
        'desktop_get_performance_capabilities'
      );
      return result ?? { platform: 'web' };
    } catch {
      return { platform: 'web' };
    }
  };

const normalizeDesktopPermission = (permission: unknown): DesktopNotificationPermission => {
  if (
    permission === 'granted' ||
    permission === 'denied' ||
    permission === 'prompt' ||
    permission === 'prompt-with-rationale'
  ) {
    return permission;
  }
  return 'denied';
};

export const getDesktopNotificationPermission =
  async (): Promise<DesktopNotificationPermission> => {
    const result = await invokeDesktop<string>('desktop_get_notification_permission');
    return normalizeDesktopPermission(result);
  };

export const requestDesktopNotificationPermission =
  async (): Promise<DesktopNotificationPermission> => {
    const result = await invokeDesktop<string>('desktop_request_notification_permission');
    return normalizeDesktopPermission(result);
  };

export const showDesktopNotification = async (
  notification: DesktopNotificationPayload
): Promise<boolean> => {
  const payload: DesktopNotificationPayload = {
    title: normalizeValue(notification.title),
    body: notification.body === undefined ? undefined : normalizeValue(notification.body),
    route: sanitizeDesktopNotificationRoute(notification.route),
  };
  if (notification.actions) {
    payload.actions = notification.actions.map((action) => ({
      id: normalizeValue(action.id),
      label: normalizeValue(action.label),
    }));
    payload.actionContext = notification.actionContext
      ? {
          kind: normalizeValue(notification.actionContext.kind),
          roomId:
            notification.actionContext.roomId === undefined
              ? undefined
              : normalizeValue(notification.actionContext.roomId),
          eventId:
            notification.actionContext.eventId === undefined
              ? undefined
              : normalizeValue(notification.actionContext.eventId),
        }
      : undefined;
  }

  const result = await invokeDesktop<boolean>('desktop_notify', {
    notification: payload,
  });
  return result === true;
};

export const getDesktopNotificationCount = (
  unreadCounts: Iterable<{ total?: number; highlight?: number }>,
  laterActiveCount: number
): number => getBadgeCount(unreadCounts, laterActiveCount);

export const DESKTOP_TRAY_DND_TOGGLE_EVENT = 'synara-tray-dnd-toggle';

export const subscribeDesktopTrayDndToggle = (handler: () => void): (() => void) => {
  const listener = () => handler();
  window.addEventListener(DESKTOP_TRAY_DND_TOGGLE_EVENT, listener);
  return () => window.removeEventListener(DESKTOP_TRAY_DND_TOGGLE_EVENT, listener);
};
