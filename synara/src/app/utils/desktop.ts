import { getBadgeCount } from '../notifications/badgeSummary';
import { getHomeRoomPath } from '../pages/pathUtils';
import {
  normalizeAgentActionPayload,
  type AgentActionPayload,
  type NormalizedAgentActionPayload,
} from '../agents/agentActions';

type TauriInternals = {
  invoke?: <T = unknown>(command: string, args?: Record<string, unknown>) => Promise<T>;
  transformCallback?: <T>(callback: (response: T) => void, once?: boolean) => number;
};

type TauriEventPluginInternals = {
  unregisterListener?: (event: string, eventId: number) => void;
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

export type DesktopShortcutApplyState = 'active' | 'permission-needed' | 'unsupported' | 'failed';

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

export type DesktopPerformanceCapabilities = {
  platform: string;
  appVersion?: string;
  buildRevision?: string;
  buildBranch?: string;
  buildLabel?: string;
  fps?: number;
  memoryUsageBytes?: number;
};

export type DesktopNotificationPermission =
  | 'granted'
  | 'denied'
  | 'prompt'
  | 'prompt-with-rationale';

export type DesktopNotificationPayload = {
  title: string;
  body?: string;
  route?: string;
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

export const shouldStreamDesktopFileIpc = (byteLength: number): boolean =>
  byteLength > DESKTOP_FILE_IPC_INLINE_THRESHOLD;

declare global {
  interface Window {
    __SYNARA_DESKTOP__?: SynaraDesktopBridge;
    __TAURI_INTERNALS__?: TauriInternals;
    __TAURI_EVENT_PLUGIN_INTERNALS__?: TauriEventPluginInternals;
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

export const buildDesktopNotificationRoomRoute = (
  roomId: string,
  eventId?: string
): string => getHomeRoomPath(roomId, eventId);

const toShortcutApplyState = (value: unknown): DesktopShortcutApplyState | undefined => {
  if (value === 'active') return 'active';
  if (value === 'permission-needed') return 'permission-needed';
  if (value === 'unsupported') return 'unsupported';
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

export const invokeDesktop = async <T = unknown>(
  command: string,
  args?: Record<string, unknown>
): Promise<T | undefined> => {
  const invoke = window.__SYNARA_DESKTOP__?.invoke ?? window.__TAURI_INTERNALS__?.invoke;
  if (!invoke) return undefined;
  return invoke<T>(command, args);
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

export const openDesktopExternalUrl = async (url: string): Promise<boolean> => {
  const normalizedUrl = normalizeActionField(url, MAX_DESKTOP_ACTION_URL_LENGTH);
  if (!normalizedUrl || !isSynaraDesktop()) return false;
  const result = await invokeDesktop<boolean>('desktop_open_external_url', { url: normalizedUrl });
  return result === true;
};

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
    if (!chunk || chunk.length === 0) break;
    chunks.push(new Uint8Array(chunk));
    offset += chunk.length;
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

export const setDesktopShortcuts = async (
  shortcuts: DesktopShortcutConfig
): Promise<DesktopShortcutApplyResult> => {
  if (getBridge()?.supportsGlobalShortcuts === false) {
    return normalizeDesktopShortcutApplyResult(undefined);
  }

  try {
    const result = await invokeDesktop<unknown>('desktop_set_shortcuts', { shortcuts });
    return normalizeDesktopShortcutApplyResult(result);
  } catch {
    return normalizeDesktopShortcutApplyResult(undefined);
  }
};

export const setDesktopTrayState = async (state: DesktopTrayState): Promise<boolean> => {
  if (getBridge()?.supportsTrayState !== true) return false;

  try {
    const result = await invokeDesktop<unknown>('desktop_update_tray_state', {
      state: {
        unreadCount: clampCount(state.unreadCount),
        highlightCount: clampCount(state.highlightCount),
        laterCount: clampCount(state.laterCount),
        notificationInboxCount: clampCount(state.notificationInboxCount),
        doNotDisturb: state.doNotDisturb === true,
      },
    });
    return result === true || result === undefined || result === null;
  } catch {
    return false;
  }
};

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
    const result = await invokeDesktop<DesktopPerformanceCapabilities>(
      'desktop_get_performance_capabilities'
    );
    return result ?? { platform: 'web' };
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
  const result = await invokeDesktop<boolean>('desktop_notify', {
    notification: {
      title: normalizeValue(notification.title),
      body: notification.body === undefined ? undefined : normalizeValue(notification.body),
      route: sanitizeDesktopNotificationRoute(notification.route),
    },
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
