import {
  defaultDesktopPlatformSettings,
  getPlatformSettings,
  type DesktopPlatformSettings,
} from '../state/settings';

export type ClientDiagnosticDomain = 'performance' | 'session' | 'room';

export type ClientDiagnosticIdentifiers = {
  roomId?: string;
  eventId?: string;
};

export type DesktopDiagnosticsConfig = Pick<
  DesktopPlatformSettings,
  | 'desktopDiagnosticsEnabled'
  | 'desktopDiagnosticsPerformance'
  | 'desktopDiagnosticsSession'
  | 'desktopDiagnosticsRoomState'
  | 'desktopDiagnosticsOverlay'
>;

export type ClientDiagnosticPayload = {
  category: ClientDiagnosticDomain;
  event: string;
  fields: Record<string, unknown>;
};

const MAX_SAFE_FIELDS = 20;
const MAX_IDENTIFIER_TOKENS = 256;
const APP_STARTED_AT_MS = Date.now();
const APP_RUN_ID = `run-${APP_STARTED_AT_MS.toString(36)}-${Math.random()
  .toString(36)
  .slice(2, 10)}`;

const CONFIG_KEYS: Array<keyof DesktopDiagnosticsConfig> = [
  'desktopDiagnosticsEnabled',
  'desktopDiagnosticsPerformance',
  'desktopDiagnosticsSession',
  'desktopDiagnosticsRoomState',
  'desktopDiagnosticsOverlay',
];

const SAFE_NUMBER_FIELDS = new Set([
  'sequence',
  'traceSequence',
  'uptimeMs',
  'durationMs',
  'requestDurationMs',
  'nativeWriteDurationMs',
  'elapsedMs',
  'ageMs',
  'expiresInMs',
  'retryCount',
  'attempt',
  'generation',
  'revision',
  'eventCount',
  'linkedEventCount',
  'renderedRowCount',
  'rowCount',
  'rowIndex',
  'previousRowIndex',
  'offsetTop',
  'scrollTop',
  'previousScrollTop',
  'scrollDelta',
  'scrollHeight',
  'previousScrollHeight',
  'heightDelta',
  'viewportHeight',
  'bottomGap',
  'previousBottomGap',
  'totalSize',
  'totalSizeDelta',
  'anchorCorrection',
  'maxScrollDelta',
  'maxVelocity',
  'stableFrames',
  'waiterCount',
  'queueDepth',
  'coalescedCount',
  'fps',
  'longTaskCount',
  'lastLongTaskMs',
  'maxLongTaskMs',
  'memoryMb',
]);

const SAFE_BOOLEAN_FIELDS = new Set([
  'available',
  'success',
  'hasSession',
  'hasRefreshToken',
  'hasExpiry',
  'expired',
  'freshLogin',
  'identityCleared',
  'fallbackPresent',
  'nativeStoreAvailable',
  'nativeStoreConfigured',
  'nativeStoreError',
  'nativeRemovalError',
  'bridgeAvailable',
  'canPersistSession',
  'hasUnreadTarget',
  'hasUnreadSignal',
  'hasExpiryMetadata',
  'fallbackSdkStores',
  'identityStoresCleared',
  'continuityConfirmationPending',
  'unreadInInitialWindow',
  'readFrontierAtLiveTail',
  'hasSavedViewport',
  'savedViewportAtBottom',
  'savedAnchorPresent',
  'anchorInWindow',
  'restoredSavedViewport',
  'liveTailRecorded',
  'loadedAtEnd',
  'liveEndPinned',
  'atBottom',
  'userScrolling',
  'programmaticScroll',
  'structuralUpdateQueued',
  'fallbackUsed',
  'matrixStoreClearSuccess',
  'timedOut',
  'confirmed',
  'fromLiveTimeline',
  'privateReceipt',
  'publicReceipt',
  'hasConcreteHead',
  'preservedSummary',
  'activityChanged',
  'latestChanged',
  'enabled',
  'boundedContextsEnabled',
  'stableAnchoringEnabled',
  'documentVisible',
  'documentFocused',
  'online',
]);

const SAFE_LABEL_FIELDS = new Set([
  'appRunId',
  'roomToken',
  'eventToken',
  'traceId',
  'openMode',
  'source',
  'target',
  'status',
  'outcome',
  'phase',
  'direction',
  'errorType',
  'reason',
  'eventType',
  'msgtype',
  'mode',
  'queueState',
  'feature',
  'backend',
  'persistence',
  'continuity',
  'syncState',
  'previousSyncState',
  'inputKind',
  'writer',
  'navigationPhase',
  'readFrontierSource',
]);

const SAFE_RANGE_FIELDS = new Set(['range', 'virtualRange', 'previousVirtualRange']);

type IdentifierKind = 'room' | 'event';
const identifierTokens: Record<IdentifierKind, Map<string, string>> = {
  room: new Map(),
  event: new Map(),
};
const identifierCounters: Record<IdentifierKind, number> = { room: 0, event: 0 };
let diagnosticSequence = 0;

const readConfig = (): DesktopDiagnosticsConfig => {
  try {
    const settings = getPlatformSettings();
    return {
      desktopDiagnosticsEnabled: settings.desktopDiagnosticsEnabled === true,
      desktopDiagnosticsPerformance: settings.desktopDiagnosticsPerformance === true,
      desktopDiagnosticsSession: settings.desktopDiagnosticsSession === true,
      desktopDiagnosticsRoomState: settings.desktopDiagnosticsRoomState === true,
      desktopDiagnosticsOverlay: settings.desktopDiagnosticsOverlay === true,
    };
  } catch {
    return {
      desktopDiagnosticsEnabled: defaultDesktopPlatformSettings.desktopDiagnosticsEnabled,
      desktopDiagnosticsPerformance: defaultDesktopPlatformSettings.desktopDiagnosticsPerformance,
      desktopDiagnosticsSession: defaultDesktopPlatformSettings.desktopDiagnosticsSession,
      desktopDiagnosticsRoomState: defaultDesktopPlatformSettings.desktopDiagnosticsRoomState,
      desktopDiagnosticsOverlay: defaultDesktopPlatformSettings.desktopDiagnosticsOverlay,
    };
  }
};

let configSnapshot = readConfig();
const configListeners = new Set<() => void>();

const configEquals = (left: DesktopDiagnosticsConfig, right: DesktopDiagnosticsConfig): boolean =>
  CONFIG_KEYS.every((key) => left[key] === right[key]);

export const getDesktopDiagnosticsConfig = (): DesktopDiagnosticsConfig => configSnapshot;

export const refreshDesktopDiagnosticsConfig = (
  platformSettings?: DesktopPlatformSettings
): DesktopDiagnosticsConfig => {
  const next = platformSettings
    ? {
        desktopDiagnosticsEnabled: platformSettings.desktopDiagnosticsEnabled === true,
        desktopDiagnosticsPerformance: platformSettings.desktopDiagnosticsPerformance === true,
        desktopDiagnosticsSession: platformSettings.desktopDiagnosticsSession === true,
        desktopDiagnosticsRoomState: platformSettings.desktopDiagnosticsRoomState === true,
        desktopDiagnosticsOverlay: platformSettings.desktopDiagnosticsOverlay === true,
      }
    : readConfig();
  if (!configEquals(configSnapshot, next)) {
    configSnapshot = next;
    configListeners.forEach((listener) => listener());
  }
  return configSnapshot;
};

export const updateDesktopDiagnosticsConfig = (
  updates: Partial<DesktopDiagnosticsConfig>
): DesktopDiagnosticsConfig => {
  const next = { ...configSnapshot, ...updates };
  if (!configEquals(configSnapshot, next)) {
    configSnapshot = next;
    configListeners.forEach((listener) => listener());
  }
  return configSnapshot;
};

export const subscribeDesktopDiagnosticsConfig = (listener: () => void): (() => void) => {
  configListeners.add(listener);
  return () => configListeners.delete(listener);
};

export const isClientDiagnosticEnabled = (domain: ClientDiagnosticDomain): boolean => {
  if (!configSnapshot.desktopDiagnosticsEnabled) return false;
  if (domain === 'performance') return configSnapshot.desktopDiagnosticsPerformance;
  if (domain === 'session') return configSnapshot.desktopDiagnosticsSession;
  return configSnapshot.desktopDiagnosticsRoomState;
};

const safeLabel = (value: string): string | undefined => {
  const normalized = value.trim();
  if (
    normalized.length === 0 ||
    normalized.length > 64 ||
    /^[@!$#~]/.test(normalized) ||
    normalized.includes('://') ||
    !/^[a-zA-Z0-9_.:/-]+$/.test(normalized)
  ) {
    return undefined;
  }
  return normalized;
};

const tokenizeIdentifier = (kind: IdentifierKind, identifier: string): string => {
  const tokens = identifierTokens[kind];
  const existing = tokens.get(identifier);
  if (existing) return existing;
  if (tokens.size >= MAX_IDENTIFIER_TOKENS) {
    const oldest = tokens.keys().next().value;
    if (oldest) tokens.delete(oldest);
  }
  identifierCounters[kind] += 1;
  const token = `${kind}-${identifierCounters[kind]}`;
  tokens.set(identifier, token);
  return token;
};

const sanitizeRange = (value: unknown): Record<string, number> | undefined => {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return undefined;
  const candidate = value as Record<string, unknown>;
  const range = Object.fromEntries(
    ['start', 'end', 'startIndex', 'endIndex']
      .filter((key) => typeof candidate[key] === 'number' && Number.isFinite(candidate[key]))
      .map((key) => [key, Math.round((candidate[key] as number) * 100) / 100])
  );
  return Object.keys(range).length > 0 ? range : undefined;
};

const sanitizeFields = (fields: Record<string, unknown>): Record<string, unknown> => {
  const safeFields: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(fields)) {
    if (Object.keys(safeFields).length >= MAX_SAFE_FIELDS) break;
    if (SAFE_NUMBER_FIELDS.has(key) && typeof value === 'number' && Number.isFinite(value)) {
      safeFields[key] = Math.round(value * 100) / 100;
    } else if (SAFE_BOOLEAN_FIELDS.has(key) && typeof value === 'boolean') {
      safeFields[key] = value;
    } else if (SAFE_LABEL_FIELDS.has(key) && typeof value === 'string') {
      const label = safeLabel(value);
      if (label) safeFields[key] = label;
    } else if (SAFE_RANGE_FIELDS.has(key)) {
      const range = sanitizeRange(value);
      if (range) safeFields[key] = range;
    }
  }
  return safeFields;
};

const appUptimeMs = (): number => {
  if (typeof performance !== 'undefined' && Number.isFinite(performance.now())) {
    return Math.round(performance.now());
  }
  return Math.max(0, Date.now() - APP_STARTED_AT_MS);
};

export const buildClientDiagnosticPayload = (
  category: ClientDiagnosticDomain,
  event: string,
  fields: Record<string, unknown> = {},
  identifiers: ClientDiagnosticIdentifiers = {}
): ClientDiagnosticPayload | undefined => {
  const safeEvent = safeLabel(event);
  if (!safeEvent) return undefined;

  diagnosticSequence += 1;
  const safeFields = sanitizeFields(fields);
  const envelopeFields: Record<string, unknown> = {
    ...safeFields,
    appRunId: APP_RUN_ID,
    sequence: diagnosticSequence,
    uptimeMs: appUptimeMs(),
  };
  if (identifiers.roomId) {
    envelopeFields.roomToken = tokenizeIdentifier('room', identifiers.roomId);
  }
  if (identifiers.eventId) {
    envelopeFields.eventToken = tokenizeIdentifier('event', identifiers.eventId);
  }

  return { category, event: safeEvent, fields: envelopeFields };
};

const invokeNativeDiagnostic = (payload: ClientDiagnosticPayload): void => {
  if (typeof window === 'undefined') return;
  const invoke = window.__SYNARA_DESKTOP__?.invoke ?? window.__TAURI_INTERNALS__?.invoke;
  if (!invoke) return;
  try {
    void invoke('desktop_record_diagnostic', payload).catch(() => undefined);
  } catch {
    // Diagnostics must never affect the operation being observed.
  }
};

export const recordClientDiagnostic = (
  category: ClientDiagnosticDomain,
  event: string,
  fields: Record<string, unknown> = {},
  identifiers: ClientDiagnosticIdentifiers = {}
): void => {
  if (!isClientDiagnosticEnabled(category)) return;
  const payload = buildClientDiagnosticPayload(category, event, fields, identifiers);
  if (payload) invokeNativeDiagnostic(payload);
};

export const clearClientDiagnosticTokens = (): void => {
  identifierTokens.room.clear();
  identifierTokens.event.clear();
  identifierCounters.room = 0;
  identifierCounters.event = 0;
};

export const resetClientDiagnosticsForTests = (): void => {
  diagnosticSequence = 0;
  clearClientDiagnosticTokens();
  configSnapshot = readConfig();
};
