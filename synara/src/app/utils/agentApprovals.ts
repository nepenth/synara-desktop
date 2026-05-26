const MAX_AGENT_APPROVAL_BODY_CHARS = 100_000;
const MAX_AGENT_APPROVAL_COMMAND_CHARS = 180;

export type AgentApprovalPrompt = {
  title: string;
  body: string;
  commandPreview?: string;
};

const COMMAND_FENCE_RE = /```(?:[a-z0-9_-]+)?\s*\n([\s\S]*?)```/i;
const CODE_BLOCK_LABEL_RE = /\bCode\s+(?:Copy\s*)?([\s\S]*?)(?=\n+Reason:|\n+Reply\s+\/approve|$)/i;
const HTML_TAG_RE = /<[^>]+>/g;
const APPROVAL_HEADINGS = [
  'approval required: dangerous command',
  'dangerous command requires approval',
];

const normalizeWhitespace = (value: string): string => value.replace(/\s+/g, ' ').trim();

const truncate = (value: string, maxChars: number): string =>
  value.length > maxChars ? `${value.slice(0, maxChars - 1)}...` : value;

const extractCommandPreview = (body: string): string | undefined => {
  const fenced = body.match(COMMAND_FENCE_RE)?.[1];
  const rawCommand = fenced ?? body.match(CODE_BLOCK_LABEL_RE)?.[1];
  if (!rawCommand) return undefined;

  const firstUsefulLine = rawCommand
    .split('\n')
    .map((line) => line.trim())
    .find((line) => line && !line.toLowerCase().startsWith('copy'));

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
  const reason = body.match(/\bReason:\s*([^\n]+)/i)?.[1];
  const reasonBody = reason ? truncate(normalizeWhitespace(reason), 220) : undefined;

  return {
    title: 'Approval Required: Dangerous Command',
    body: reasonBody ?? 'A Hermes Agent command is waiting for approval.',
    commandPreview,
  };
};

export const detectAgentApprovalPrompt = (
  content: Record<string, unknown>
): AgentApprovalPrompt | undefined => {
  return getApprovalBodyCandidates(content).map(detectAgentApprovalPromptBody).find(Boolean);
};
