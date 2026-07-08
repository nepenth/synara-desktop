const MAX_AGENT_APPROVAL_BODY_CHARS = 100_000;
const MAX_AGENT_APPROVAL_COMMAND_CHARS = 180;
const MAX_AGENT_APPROVAL_COMMAND_BODY_CHARS = 8_000;

export const AGENT_APPROVAL_REACTION_APPROVE_ONCE = '✅';
export const AGENT_APPROVAL_REACTION_APPROVE_ALWAYS = '♾️';
export const AGENT_APPROVAL_REACTION_DENY = '❌';
export const AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ONCE = 'agent-approval.approve-once';
export const AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ALWAYS = 'agent-approval.approve-always';
export const AGENT_APPROVAL_NOTIFICATION_ACTION_DENY = 'agent-approval.deny';

export type AgentApprovalNotificationActionId =
  | typeof AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ONCE
  | typeof AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ALWAYS
  | typeof AGENT_APPROVAL_NOTIFICATION_ACTION_DENY;

export const AGENT_APPROVAL_NOTIFICATION_ACTIONS: {
  id: AgentApprovalNotificationActionId;
  label: string;
}[] = [
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

export type AgentApprovalPrompt = {
  title: string;
  body: string;
  command?: string;
  commandPreview?: string;
};

const COMMAND_FENCE_RE = /```(?:[a-z0-9_-]+)?\s*\n([\s\S]*?)```/i;
const CODE_BLOCK_LABEL_RE =
  /\bCode\s+(?:Copy\s*)?([\s\S]*?)(?=\n+Reason:|\n+Reply\s+[!/](?:approve|deny)\b|$)/i;
const HTML_TAG_RE = /<[^>]+>/g;
const APPROVAL_HEADINGS = [
  'approval required: dangerous command',
  'dangerous command requires approval',
];

const normalizeWhitespace = (value: string): string => value.replace(/\s+/g, ' ').trim();

const truncate = (value: string, maxChars: number): string =>
  value.length > maxChars ? `${value.slice(0, maxChars - 1)}...` : value;

const cleanCommand = (value: string): string | undefined => {
  const command = value
    .split('\n')
    .map((line) => line.trimEnd())
    .filter((line, index) => index > 0 || line.trim().toLowerCase() !== 'copy')
    .join('\n')
    .trim();

  return command ? truncate(command, MAX_AGENT_APPROVAL_COMMAND_BODY_CHARS) : undefined;
};

const extractCommand = (body: string): string | undefined => {
  const fenced = body.match(COMMAND_FENCE_RE)?.[1];
  const rawCommand = fenced ?? body.match(CODE_BLOCK_LABEL_RE)?.[1];
  if (!rawCommand) return undefined;
  return cleanCommand(rawCommand);
};

const extractCommandPreview = (body: string): string | undefined => {
  const command = extractCommand(body);
  if (!command) return undefined;

  const firstUsefulLine = command
    .split('\n')
    .map((line) => line.trim())
    .find(Boolean);

  return firstUsefulLine
    ? truncate(normalizeWhitespace(firstUsefulLine), MAX_AGENT_APPROVAL_COMMAND_CHARS)
    : undefined;
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

  const commandPreview = extractCommandPreview(body);
  const command = extractCommand(body);
  const reason = body.match(/\bReason:\s*([^\n]+)/i)?.[1];
  const reasonBody = reason ? truncate(normalizeWhitespace(reason), 220) : undefined;

  return {
    title: 'Approval Required: Dangerous Command',
    body: reasonBody ?? 'A Hermes Agent command is waiting for approval.',
    command,
    commandPreview,
  };
};

export const detectAgentApprovalPrompt = (
  content: Record<string, unknown>
): AgentApprovalPrompt | undefined => {
  return getApprovalBodyCandidates(content).map(detectAgentApprovalPromptBody).find(Boolean);
};
