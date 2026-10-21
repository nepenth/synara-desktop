import {
  MAX_AGENT_ACTION_TEXT_LENGTH,
  normalizeAgentActionPayload,
  planAgentActionExecution,
  type AgentActionPayload,
  type NormalizedAgentActionPayload,
} from '../agents/agentActions';
import { copyToClipboard } from '../utils/dom';
import {
  isSynaraDesktop,
  listen,
  sendDesktopAgentAction,
  type DesktopAgentActionEventPayload,
  type DesktopAgentActionOutcome,
  type DesktopAgentActionPayload,
  type DesktopUnlisten,
} from '../utils/desktop';
import { openPlatformExternalUrl } from './links';

export const PLATFORM_AGENT_ACTION_EVENT = 'synara://agent-action';

export type PlatformAgentActionPayload = DesktopAgentActionPayload;

const RECENT_AGENT_ACTION_IDS = new Set<string>();
const MAX_RECENT_AGENT_ACTION_IDS = 32;

const rememberAgentActionId = (id: string): boolean => {
  if (RECENT_AGENT_ACTION_IDS.has(id)) return false;
  RECENT_AGENT_ACTION_IDS.add(id);
  if (RECENT_AGENT_ACTION_IDS.size > MAX_RECENT_AGENT_ACTION_IDS) {
    const oldest = RECENT_AGENT_ACTION_IDS.values().next().value;
    if (oldest) RECENT_AGENT_ACTION_IDS.delete(oldest);
  }
  return true;
};

let agentActionInvocationCounter = 0;

/**
 * Card action ids repeat (`action-0` on every card) and a click can repeat,
 * but the event listener drops ids it has already run. Each send therefore
 * carries a per-invocation id: a duplicated event of one click still runs
 * once, while a second click or another card's action runs again.
 */
export const withAgentActionInvocationId = (
  action: PlatformAgentActionPayload,
  nonce: string
): PlatformAgentActionPayload => {
  const id = typeof action.id === 'string' ? action.id.trim() : '';
  // An action without an id stays invalid; the nonce must not make it valid.
  if (!id) return action;
  const suffix = `#${nonce}`;
  return { ...action, id: `${id.slice(0, MAX_AGENT_ACTION_TEXT_LENGTH - suffix.length)}${suffix}` };
};

const nextAgentActionNonce = (): string => {
  agentActionInvocationCounter += 1;
  return `${Date.now().toString(36)}-${agentActionInvocationCounter.toString(36)}`;
};

/**
 * The shell reports what it did: ran the action itself, handed it to the
 * listener below (which runs it with the same plan), or could not run it.
 * An action that plan cannot run (an `open` without a URL) is reported as
 * unsupported up front, so the caller's fallback runs instead of a silent
 * no-op.
 */
export const sendPlatformAgentAction = async (
  action: PlatformAgentActionPayload
): Promise<DesktopAgentActionOutcome> => {
  const normalized = normalizeAgentActionPayload(action);
  if (normalized && planAgentActionExecution(normalized).type === 'unsupported') {
    return 'unsupported';
  }
  return sendDesktopAgentAction(withAgentActionInvocationId(action, nextAgentActionNonce()));
};

const openAgentActionUrl = async (url: string): Promise<boolean> => {
  const opened = await openPlatformExternalUrl(url);
  if (opened) return true;
  if (isSynaraDesktop() || typeof window === 'undefined') return false;
  const popup = window.open(url, '_blank', 'noopener,noreferrer');
  return popup !== null;
};

export const executePlatformAgentAction = async (
  action: NormalizedAgentActionPayload
): Promise<boolean> => {
  const plan = planAgentActionExecution(action);
  switch (plan.type) {
    case 'open-url':
      return openAgentActionUrl(plan.url);
    case 'copy-text':
      copyToClipboard(plan.text);
      return true;
    default:
      return false;
  }
};

export const parseIncomingPlatformAgentAction = (
  raw: unknown
): NormalizedAgentActionPayload | undefined => {
  if (!raw || typeof raw !== 'object') return undefined;
  const envelope = raw as DesktopAgentActionEventPayload;
  if (!envelope.action || typeof envelope.action !== 'object') return undefined;
  return normalizeAgentActionPayload(envelope.action as AgentActionPayload);
};

export const handleIncomingPlatformAgentAction = async (raw: unknown): Promise<boolean> => {
  const action = parseIncomingPlatformAgentAction(raw);
  if (!action) return false;
  if (!rememberAgentActionId(action.id)) return false;
  return executePlatformAgentAction(action);
};

export const registerPlatformAgentActionListener = async (): Promise<
  DesktopUnlisten | undefined
> => {
  if (!isSynaraDesktop()) return undefined;

  return listen<DesktopAgentActionEventPayload>('synara://agent-action', (event) => {
    void handleIncomingPlatformAgentAction(event.payload);
  });
};
