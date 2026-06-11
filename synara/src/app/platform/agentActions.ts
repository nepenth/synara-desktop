import {
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

export const sendPlatformAgentAction = sendDesktopAgentAction;

const openAgentActionUrl = async (url: string): Promise<boolean> => {
  const opened = await openPlatformExternalUrl(url);
  if (opened) return true;
  if (typeof window === 'undefined') return false;
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
