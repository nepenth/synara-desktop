import { recordDesktopDiagnostic } from './desktopDiagnostics';

export type FoundationDiagnosticDomain = 'timeline' | 'read' | 'activity';

type FoundationDiagnosticInput = {
  roomId?: string;
  eventId?: string;
  fields?: Record<string, unknown>;
};

type RedactedTokenKind = 'room' | 'event';

const MAX_IDENTIFIER_TOKENS = 128;
const MAX_DIAGNOSTIC_LENGTH = 220;
const MAX_SAFE_FIELDS = 10;

const identifierTokens: Record<RedactedTokenKind, Map<string, string>> = {
  room: new Map(),
  event: new Map(),
};
const identifierCounters: Record<RedactedTokenKind, number> = { room: 0, event: 0 };

const SAFE_NUMBER_FIELDS = new Set([
  'sequence',
  'elapsedMs',
  'eventCount',
  'linkedEventCount',
  'renderedRowCount',
  'rowIndex',
  'offsetTop',
  'item',
  'durationMs',
  'waiterCount',
  'revision',
]);
const SAFE_BOOLEAN_FIELDS = new Set([
  'hasUnreadTarget',
  'unreadInInitialWindow',
  'loadedAtEnd',
  'liveEndPinned',
  'atBottom',
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
]);
const SAFE_LABEL_FIELDS = new Set([
  'traceId',
  'openMode',
  'direction',
  'errorType',
  'reason',
  'eventType',
  'msgtype',
  'mode',
  'queueState',
  'feature',
]);
const SAFE_RANGE_FIELDS = new Set(['range', 'virtualRange']);

const safeLabel = (value: string): string => {
  if (
    value.length === 0 ||
    value.length > 48 ||
    /^[@!$#~]/.test(value) ||
    value.includes('://') ||
    !/^[a-zA-Z0-9_.:/-]+$/.test(value)
  ) {
    return '[redacted]';
  }
  return value;
};

const tokenizeIdentifier = (kind: RedactedTokenKind, identifier: string): string => {
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
  if (!value || typeof value !== 'object') return undefined;
  const candidate = value as Record<string, unknown>;
  const range = Object.fromEntries(
    ['start', 'end', 'startIndex', 'endIndex']
      .filter((key) => typeof candidate[key] === 'number' && Number.isFinite(candidate[key]))
      .map((key) => [key, Math.round(candidate[key] as number)])
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
      safeFields[key] = safeLabel(value);
    } else if (SAFE_RANGE_FIELDS.has(key)) {
      const range = sanitizeRange(value);
      if (range) safeFields[key] = range;
    }
  }
  return safeFields;
};

export const recordFoundationDiagnostic = (
  domain: FoundationDiagnosticDomain,
  event: string,
  { roomId, eventId, fields = {} }: FoundationDiagnosticInput = {}
): void => {
  try {
    const envelope: Record<string, unknown> = {
      version: 1,
      domain,
      event: safeLabel(event),
    };
    if (roomId) envelope.room = tokenizeIdentifier('room', roomId);
    if (eventId) envelope.eventToken = tokenizeIdentifier('event', eventId);

    const safeFields = sanitizeFields(fields);
    const acceptedFields: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(safeFields)) {
      const candidate = JSON.stringify({
        ...envelope,
        fields: { ...acceptedFields, [key]: value },
      });
      if (`[synara:foundation] ${candidate}`.length > MAX_DIAGNOSTIC_LENGTH) break;
      acceptedFields[key] = value;
    }
    if (Object.keys(acceptedFields).length > 0) envelope.fields = acceptedFields;

    recordDesktopDiagnostic(`[synara:foundation] ${JSON.stringify(envelope)}`);
  } catch {
    // Diagnostics must never affect timeline, receipt, or room-list behavior.
  }
};

export const clearFoundationDiagnosticTokens = (): void => {
  identifierTokens.room.clear();
  identifierTokens.event.clear();
  identifierCounters.room = 0;
  identifierCounters.event = 0;
};
