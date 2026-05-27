import { safeRemoteContentUrl } from '../utils/remoteContent';

export const AGENT_ACTION_KINDS = [
  'agent',
  'copy',
  'continue',
  'export',
  'prompt',
  'regenerate',
  'run',
  'approve',
  'reject',
  'open',
  'open_url',
] as const;

export type AgentActionKind = typeof AGENT_ACTION_KINDS[number];

export type AgentActionPayload = {
  id: string;
  title: string;
  kind?: string;
  prompt?: string;
  url?: string;
  markdown?: string;
};

export type NormalizedAgentActionPayload = {
  id: string;
  title: string;
  kind?: AgentActionKind;
  prompt?: string;
  url?: string;
  markdown?: string;
};

export const MAX_AGENT_ACTION_TEXT_LENGTH = 1_024;
export const MAX_AGENT_ACTION_MARKDOWN_LENGTH = 16_384;
export const MAX_AGENT_ACTION_URL_LENGTH = 2_048;

const AGENT_ACTION_KIND_SET = new Set<string>(AGENT_ACTION_KINDS);

const normalizeString = (value: unknown, maxLength: number): string | undefined => {
  if (typeof value !== 'string') return undefined;
  const normalized = value.trim();
  if (!normalized || normalized.length > maxLength) return undefined;
  return normalized;
};

const normalizeKind = (value: unknown): AgentActionKind | undefined => {
  const kind = normalizeString(value, MAX_AGENT_ACTION_TEXT_LENGTH)?.toLowerCase();
  if (!kind) return undefined;
  if (!AGENT_ACTION_KIND_SET.has(kind)) return undefined;
  return kind as AgentActionKind;
};

const normalizeUrl = (value: unknown): string | undefined => {
  const url = normalizeString(value, MAX_AGENT_ACTION_URL_LENGTH);
  if (!url) return undefined;
  return safeRemoteContentUrl(url);
};

export const normalizeAgentActionPayload = (
  action: AgentActionPayload
): NormalizedAgentActionPayload | undefined => {
  const id = normalizeString(action.id, MAX_AGENT_ACTION_TEXT_LENGTH);
  const title = normalizeString(action.title, MAX_AGENT_ACTION_TEXT_LENGTH);
  if (!id || !title) return undefined;

  const kind = action.kind === undefined ? undefined : normalizeKind(action.kind);
  if (action.kind !== undefined && !kind) return undefined;

  const url = action.url === undefined ? undefined : normalizeUrl(action.url);
  if (action.url !== undefined && !url) return undefined;

  const prompt = normalizeString(action.prompt, MAX_AGENT_ACTION_TEXT_LENGTH);
  const markdown = normalizeString(action.markdown, MAX_AGENT_ACTION_MARKDOWN_LENGTH);

  if (!url && !prompt && !markdown) return undefined;

  return {
    id,
    title,
    kind,
    prompt,
    url,
    markdown,
  };
};
