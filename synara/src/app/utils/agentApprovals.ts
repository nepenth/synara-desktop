const MAX_AGENT_APPROVAL_BODY_CHARS = 100_000;
const MAX_AGENT_APPROVAL_COMMAND_CHARS = 180;
const MAX_AGENT_APPROVAL_COMMAND_BODY_CHARS = 8_000;
/** Bounded original prompt body shown in approval cards for operator context. */
const MAX_AGENT_APPROVAL_SOURCE_CONTEXT_CHARS = 4_000;
const MAX_AGENT_APPROVAL_REPLY_INSTRUCTIONS_CHARS = 600;

export const AGENT_APPROVAL_REACTION_APPROVE_ONCE = '✅';
export const AGENT_APPROVAL_REACTION_APPROVE_ALWAYS = '♾️';
export const AGENT_APPROVAL_REACTION_DENY = '❌';
export const AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ONCE = 'agent-approval.approve-once';
export const AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ALWAYS = 'agent-approval.approve-always';
export const AGENT_APPROVAL_NOTIFICATION_ACTION_DENY = 'agent-approval.deny';
export const AGENT_APPROVAL_NOTIFICATION_ACTION_REVIEW = 'agent-approval.review';
export const AGENT_APPROVAL_NOTIFICATION_KIND = 'agent-approval';

/** Max age of an approval prompt event (or native action) that can be acted on from OS notifications. */
export const AGENT_APPROVAL_NATIVE_ACTION_TTL_MS = 5 * 60 * 1000;

export const AGENT_APPROVAL_NATIVE_ACTION_DEDUP_STORAGE_KEY =
  'synara.agent-approval.native-action-dedupe';

export const AGENT_APPROVAL_REACTION_KEYS = [
  AGENT_APPROVAL_REACTION_APPROVE_ONCE,
  AGENT_APPROVAL_REACTION_APPROVE_ALWAYS,
  AGENT_APPROVAL_REACTION_DENY,
] as const;

export type AgentApprovalNotificationActionId =
  | typeof AGENT_APPROVAL_NOTIFICATION_ACTION_REVIEW
  | typeof AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ONCE
  | typeof AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ALWAYS
  | typeof AGENT_APPROVAL_NOTIFICATION_ACTION_DENY;

export const AGENT_APPROVAL_NOTIFICATION_ACTIONS: {
  id: AgentApprovalNotificationActionId;
  label: string;
}[] = [
  {
    id: AGENT_APPROVAL_NOTIFICATION_ACTION_REVIEW,
    label: 'Review',
  },
  {
    id: AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ONCE,
    label: 'Approve once',
  },
  {
    id: AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ALWAYS,
    label: 'Approve always',
  },
  {
    id: AGENT_APPROVAL_NOTIFICATION_ACTION_DENY,
    label: 'Deny',
  },
];

/**
 * Actions exposed on native OS notifications. Approve-always is intentionally excluded:
 * permanent approval requires an explicit in-app confirmation path.
 */
export const AGENT_APPROVAL_NATIVE_NOTIFICATION_ACTIONS: {
  id: AgentApprovalNotificationActionId;
  label: string;
}[] = [
  AGENT_APPROVAL_NOTIFICATION_ACTIONS.find(
    (action) => action.id === AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ONCE
  )!,
  AGENT_APPROVAL_NOTIFICATION_ACTIONS.find(
    (action) => action.id === AGENT_APPROVAL_NOTIFICATION_ACTION_DENY
  )!,
  AGENT_APPROVAL_NOTIFICATION_ACTIONS.find(
    (action) => action.id === AGENT_APPROVAL_NOTIFICATION_ACTION_REVIEW
  )!,
];

export const getAgentApprovalReactionForNotificationAction = (
  actionId: string
): string | undefined => {
  if (actionId === AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ONCE) {
    return AGENT_APPROVAL_REACTION_APPROVE_ONCE;
  }
  if (actionId === AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ALWAYS) {
    return AGENT_APPROVAL_REACTION_APPROVE_ALWAYS;
  }
  if (actionId === AGENT_APPROVAL_NOTIFICATION_ACTION_DENY) {
    return AGENT_APPROVAL_REACTION_DENY;
  }
  return undefined;
};

export const isKnownAgentApprovalNotificationActionId = (
  actionId: string
): actionId is AgentApprovalNotificationActionId =>
  actionId === AGENT_APPROVAL_NOTIFICATION_ACTION_REVIEW ||
  actionId === AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ONCE ||
  actionId === AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ALWAYS ||
  actionId === AGENT_APPROVAL_NOTIFICATION_ACTION_DENY;

export type AgentApprovalNativeActionPlan =
  | {
      type: 'send-reaction';
      roomId: string;
      eventId: string;
      actionId: AgentApprovalNotificationActionId;
      reaction: string;
      dedupeKey: string;
    }
  | {
      type: 'open-room';
      roomId: string;
      eventId: string;
      reason: string;
    }
  | {
      type: 'reject';
      reason: string;
    };

export type AgentApprovalNativeActionContext = {
  kind?: string;
  roomId?: string;
  eventId?: string;
};

export type PlanAgentApprovalNativeActionInput = {
  actionId: string;
  context?: AgentApprovalNativeActionContext;
  nowMs?: number;
  /** Origin event timestamp in ms, if known. */
  eventTsMs?: number;
  /** When the local notification was created, if tracked. */
  notificationCreatedAtMs?: number;
  ttlMs?: number;
  /** True when this client already recorded a successful native action for this target. */
  alreadyActed?: boolean;
  /** True when the current user already has a local approval reaction on the event. */
  alreadyReactedLocally?: boolean;
  /** Result of running the approval detector against resolved event content. */
  isApprovalPrompt?: boolean;
  /** When true, the event was resolved (timeline or fetch). Required before send-reaction. */
  eventResolved?: boolean;
};

export const buildAgentApprovalNativeActionDedupeKey = (roomId: string, eventId: string): string =>
  `${roomId}\u0000${eventId}`;

const isNonEmptyId = (value: string | undefined): value is string =>
  typeof value === 'string' && value.trim().length > 0;

export const isAgentApprovalNativeActionExpired = ({
  nowMs,
  eventTsMs,
  notificationCreatedAtMs,
  ttlMs = AGENT_APPROVAL_NATIVE_ACTION_TTL_MS,
}: {
  nowMs: number;
  eventTsMs?: number;
  notificationCreatedAtMs?: number;
  ttlMs?: number;
}): boolean => {
  if (ttlMs < 0) return false;
  if (typeof eventTsMs === 'number' && Number.isFinite(eventTsMs)) {
    if (Math.max(0, nowMs - eventTsMs) > ttlMs) return true;
  }
  if (typeof notificationCreatedAtMs === 'number' && Number.isFinite(notificationCreatedAtMs)) {
    if (Math.max(0, nowMs - notificationCreatedAtMs) > ttlMs) return true;
  }
  return false;
};

/**
 * Pure decision planner for native OS notification approval actions.
 * Callers must revalidate the Matrix event (resolve + detector) before allowing send-reaction.
 */
export const planAgentApprovalNativeNotificationAction = (
  input: PlanAgentApprovalNativeActionInput
): AgentApprovalNativeActionPlan => {
  const actionId = input.actionId?.trim() ?? '';
  const kind = input.context?.kind?.trim().toLowerCase() ?? '';
  const roomId = input.context?.roomId?.trim() ?? '';
  const eventId = input.context?.eventId?.trim() ?? '';
  const nowMs = input.nowMs ?? Date.now();

  if (kind !== AGENT_APPROVAL_NOTIFICATION_KIND) {
    return { type: 'reject', reason: 'invalid-kind' };
  }
  if (!isNonEmptyId(roomId) || !isNonEmptyId(eventId)) {
    return { type: 'reject', reason: 'missing-room-or-event-id' };
  }
  if (!isKnownAgentApprovalNotificationActionId(actionId)) {
    return { type: 'reject', reason: 'unknown-action-id' };
  }

  // Review is navigation-only and remains safe and useful even after the
  // five-minute reaction window has expired.
  if (actionId === AGENT_APPROVAL_NOTIFICATION_ACTION_REVIEW) {
    return { type: 'open-room', roomId, eventId, reason: 'review-requested' };
  }

  if (
    isAgentApprovalNativeActionExpired({
      nowMs,
      eventTsMs: input.eventTsMs,
      notificationCreatedAtMs: input.notificationCreatedAtMs,
      ttlMs: input.ttlMs,
    })
  ) {
    return { type: 'reject', reason: 'expired-ttl' };
  }

  if (input.alreadyActed) {
    return { type: 'reject', reason: 'already-acted' };
  }
  if (input.alreadyReactedLocally) {
    return { type: 'reject', reason: 'already-reacted' };
  }

  // Permanent approval must not fire from a background OS notification action.
  if (actionId === AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ALWAYS) {
    return {
      type: 'open-room',
      roomId,
      eventId,
      reason: 'approve-always-requires-in-app-confirmation',
    };
  }

  if (input.eventResolved === false) {
    return { type: 'reject', reason: 'event-unresolved' };
  }
  if (input.isApprovalPrompt === false) {
    return { type: 'reject', reason: 'not-approval-prompt' };
  }

  const reaction = getAgentApprovalReactionForNotificationAction(actionId);
  if (!reaction) {
    return { type: 'reject', reason: 'unknown-action-id' };
  }

  // Before event validation completes, callers should not send. When validation fields are
  // omitted (unit tests of early gates), still return a provisional send plan only if the
  // caller explicitly marked the event resolved and prompt-valid, or left them undefined
  // after passing the early gates for a non-always action that still requires validation.
  if (input.eventResolved !== true || input.isApprovalPrompt !== true) {
    return { type: 'reject', reason: 'event-not-validated' };
  }

  return {
    type: 'send-reaction',
    roomId,
    eventId,
    actionId,
    reaction,
    dedupeKey: buildAgentApprovalNativeActionDedupeKey(roomId, eventId),
  };
};

export type AgentApprovalNativeActionDedupeStore = {
  has: (key: string) => boolean;
  add: (key: string) => void;
  remove: (key: string) => void;
};

const dedupeStorageKey = (accountScope: string): string =>
  `${AGENT_APPROVAL_NATIVE_ACTION_DEDUP_STORAGE_KEY}.${encodeURIComponent(accountScope)}`;

const readDedupeKeysFromStorage = (storage: Storage, accountScope: string): Set<string> => {
  try {
    const raw = storage.getItem(dedupeStorageKey(accountScope));
    if (!raw) return new Set();
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return new Set();
    return new Set(parsed.filter((value): value is string => typeof value === 'string'));
  } catch {
    return new Set();
  }
};

const writeDedupeKeysToStorage = (
  storage: Storage,
  accountScope: string,
  keys: Set<string>
): void => {
  try {
    // Bound growth: keep the most recently recorded actions only.
    const values = Array.from(keys).slice(-200);
    storage.setItem(dedupeStorageKey(accountScope), JSON.stringify(values));
  } catch {
    // Storage may be unavailable (private mode, quota); in-memory set still helps in-session.
  }
};

/**
 * Bounded storage-backed dedupe so a successful native approval is not
 * contradicted after the desktop process or renderer restarts.
 */
export const createAgentApprovalNativeActionDedupeStore = (
  storage?: Storage | null,
  accountScope = 'unknown-account',
  memory: Set<string> = new Set()
): AgentApprovalNativeActionDedupeStore => {
  if (storage) {
    for (const key of readDedupeKeysFromStorage(storage, accountScope)) {
      memory.add(key);
    }
  }

  return {
    has: (key) => memory.has(key),
    add: (key) => {
      memory.add(key);
      if (storage) writeDedupeKeysToStorage(storage, accountScope, memory);
    },
    remove: (key) => {
      memory.delete(key);
      if (storage) writeDedupeKeysToStorage(storage, accountScope, memory);
    },
  };
};

export const hasLocalAgentApprovalReactionFromSenders = (
  reactionSendersByKey: Iterable<[string, Iterable<string>]> | undefined,
  userId: string | undefined
): boolean => {
  if (!reactionSendersByKey || !userId) return false;
  for (const [key, senders] of reactionSendersByKey) {
    if (!(AGENT_APPROVAL_REACTION_KEYS as readonly string[]).includes(key)) continue;
    for (const sender of senders) {
      if (sender === userId) return true;
    }
  }
  return false;
};

export type AgentApprovalPrompt = {
  title: string;
  /** Prominent short reason summary (from `Reason:` when present). */
  body: string;
  command?: string;
  commandPreview?: string;
  /**
   * Bounded normalized original source body so operators can review the full
   * approval prompt (heading, command, reason, reply/reaction instructions).
   */
  sourceContext?: string;
  /** Reply / reaction instruction section when present in the source body. */
  replyInstructions?: string;
};

const COMMAND_FENCE_RE = /```(?:[a-z0-9_-]+)?\s*\n([\s\S]*?)```/i;
/**
 * Hermes-style Code/Copy labeled blocks. Capture runs until Reason/Reply so
 * multi-line commands (including heredocs) are preserved intact.
 */
const CODE_BLOCK_LABEL_RE =
  /\bCode\b(?:\s|\n)+(?:Copy\b(?:\s|\n)+)?([\s\S]*?)(?=\n+Reason:|\n+Reply\s+[!/](?:approve|deny)\b|$)/i;
const REPLY_INSTRUCTIONS_RE = /(Reply\s+[!/](?:approve|deny)\b[\s\S]*?)(?=\n{3,}|$)/i;
const HTML_TAG_RE = /<[^>]+>/g;
const APPROVAL_HEADINGS = [
  'approval required: dangerous command',
  'dangerous command requires approval',
];

const normalizeWhitespace = (value: string): string => value.replace(/\s+/g, ' ').trim();

const truncate = (value: string, maxChars: number): string =>
  value.length > maxChars ? `${value.slice(0, maxChars - 1)}...` : value;

const normalizeSourceBody = (value: string): string =>
  value
    .replace(/\r\n/g, '\n')
    .split('\n')
    .map((line) => line.trimEnd())
    .join('\n')
    .replace(/\n{3,}/g, '\n\n')
    .trim();

const cleanCommand = (value: string): string | undefined => {
  const lines = value
    .replace(/\r\n/g, '\n')
    .split('\n')
    .map((line) => line.trimEnd());

  // Drop leading chrome labels that sometimes land inside the capture group.
  while (lines.length > 0) {
    const head = lines[0]?.trim().toLowerCase() ?? '';
    if (head === '' || head === 'copy' || head === 'code') {
      lines.shift();
      continue;
    }
    break;
  }

  const command = lines.join('\n').trim();
  return command ? truncate(command, MAX_AGENT_APPROVAL_COMMAND_BODY_CHARS) : undefined;
};

/**
 * Extract the full multi-line command body. Prefer fenced blocks, then
 * Code/Copy labeled sections, then a fallback that takes lines between the
 * heading chrome and Reason/Reply markers so heredocs are not truncated to the
 * first line.
 */
const extractCommand = (body: string): string | undefined => {
  const fenced = body.match(COMMAND_FENCE_RE)?.[1];
  if (fenced) {
    const cleaned = cleanCommand(fenced);
    if (cleaned) return cleaned;
  }

  const labeled = body.match(CODE_BLOCK_LABEL_RE)?.[1];
  if (labeled) {
    const cleaned = cleanCommand(labeled);
    if (cleaned) return cleaned;
  }

  // Fallback: lines after Code/Copy chrome until Reason/Reply, without requiring
  // the capture group to stop early on blank lines inside heredocs.
  const lines = body.replace(/\r\n/g, '\n').split('\n');
  let start = -1;
  for (let index = 0; index < lines.length; index += 1) {
    const trimmed = lines[index]?.trim().toLowerCase() ?? '';
    if (trimmed === 'code' || trimmed === 'copy') {
      start = index + 1;
      continue;
    }
    if (start >= 0 && trimmed && trimmed !== 'code' && trimmed !== 'copy') {
      start = index;
      break;
    }
  }
  if (start < 0) return undefined;

  const commandLines: string[] = [];
  for (let index = start; index < lines.length; index += 1) {
    const line = lines[index] ?? '';
    const trimmed = line.trim();
    if (/^Reason:/i.test(trimmed) || /^Reply\s+[!/](?:approve|deny)\b/i.test(trimmed)) {
      break;
    }
    commandLines.push(line.trimEnd());
  }

  return cleanCommand(commandLines.join('\n'));
};

const extractCommandPreview = (command: string | undefined): string | undefined => {
  if (!command) return undefined;

  const firstUsefulLine = command
    .split('\n')
    .map((line) => line.trim())
    .find(Boolean);

  return firstUsefulLine
    ? truncate(normalizeWhitespace(firstUsefulLine), MAX_AGENT_APPROVAL_COMMAND_CHARS)
    : undefined;
};

const extractReplyInstructions = (body: string): string | undefined => {
  const section = body.match(REPLY_INSTRUCTIONS_RE)?.[1];
  if (!section) return undefined;
  const normalized = normalizeSourceBody(section);
  return normalized ? truncate(normalized, MAX_AGENT_APPROVAL_REPLY_INSTRUCTIONS_CHARS) : undefined;
};

const scoreApprovalPrompt = (prompt: AgentApprovalPrompt): number => {
  let score = 0;
  if (prompt.command) score += Math.min(prompt.command.length, 2_000);
  if (prompt.sourceContext) score += Math.min(prompt.sourceContext.length, 1_000);
  if (prompt.replyInstructions) score += 80;
  if (prompt.body && !/waiting for approval/i.test(prompt.body)) score += 40;
  if (prompt.commandPreview) score += 10;
  return score;
};

const decodeHtmlEntities = (value: string): string =>
  value
    .replace(/&nbsp;/gi, ' ')
    .replace(/&amp;/gi, '&')
    .replace(/&lt;/gi, '<')
    .replace(/&gt;/gi, '>')
    .replace(/&quot;/gi, '"')
    .replace(/&#39;/gi, "'");

const htmlToText = (value: string): string =>
  decodeHtmlEntities(
    value
      .replace(/<br\s*\/?>/gi, '\n')
      .replace(/<(?:p|div|li|pre|code|blockquote|h[1-6])(?:\s[^>]*)?>/gi, '\n')
      .replace(/<\/(?:p|div|li|pre|code|blockquote|h[1-6])>/gi, '\n')
      .replace(HTML_TAG_RE, '')
  );

const getApprovalBodyCandidates = (content: Record<string, unknown>): string[] => {
  const candidates: string[] = [];
  const body = typeof content.body === 'string' ? content.body : undefined;
  const formattedBody =
    typeof content.formatted_body === 'string' ? htmlToText(content.formatted_body) : undefined;

  if (body) candidates.push(body);
  if (formattedBody && formattedBody !== body) candidates.push(formattedBody);
  return candidates.filter((candidate) => candidate.length <= MAX_AGENT_APPROVAL_BODY_CHARS);
};

const detectAgentApprovalPromptBody = (body: string): AgentApprovalPrompt | undefined => {
  const normalized = normalizeWhitespace(body).toLowerCase();
  if (!APPROVAL_HEADINGS.some((heading) => normalized.includes(heading))) return undefined;

  const command = extractCommand(body);
  const commandPreview = extractCommandPreview(command);
  const reason = body.match(/\bReason:\s*([^\n]+)/i)?.[1];
  const reasonBody = reason ? truncate(normalizeWhitespace(reason), 220) : undefined;
  const sourceContext = truncate(
    normalizeSourceBody(body),
    MAX_AGENT_APPROVAL_SOURCE_CONTEXT_CHARS
  );
  const replyInstructions = extractReplyInstructions(body);

  return {
    title: 'Approval Required: Dangerous Command',
    body: reasonBody ?? 'A Hermes Agent command is waiting for approval.',
    command,
    commandPreview,
    sourceContext: sourceContext || undefined,
    replyInstructions,
  };
};

export const detectAgentApprovalPrompt = (
  content: Record<string, unknown>
): AgentApprovalPrompt | undefined => {
  // Prefer the richest matching candidate when both plain body and formatted_body
  // are present (formatted HTML sometimes strips or truncates command lines).
  const prompts = getApprovalBodyCandidates(content)
    .map(detectAgentApprovalPromptBody)
    .filter((prompt): prompt is AgentApprovalPrompt => Boolean(prompt));

  if (prompts.length === 0) return undefined;
  return prompts.reduce((best, candidate) =>
    scoreApprovalPrompt(candidate) > scoreApprovalPrompt(best) ? candidate : best
  );
};
