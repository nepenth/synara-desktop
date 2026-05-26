import { safeRemoteContentUrl } from './remoteContent';

export type HermesArtifact = {
  title: string;
  type?: string;
  url?: string;
  summary?: string;
};

export type HermesCodeBlock = {
  id: string;
  title?: string;
  language?: string;
  code: string;
};

export type HermesAgentAction = {
  id: string;
  title: string;
  kind?: string;
  url?: string;
  prompt?: string;
};

export type HermesAgentPayload = {
  title: string;
  status?: string;
  summary?: string;
  actions: HermesAgentAction[];
  artifacts: HermesArtifact[];
  logs: HermesCodeBlock[];
  code: HermesCodeBlock[];
  diffs: HermesCodeBlock[];
};

export const DEFAULT_HERMES_KEYS = ['org.hermes.agent', 'io.hermes.agent', 'in.synara.agent'];
const MAX_PARSE_BODY_LENGTH = 200_000;
const MAX_BLOCKS_PER_SECTION = 20;
const MAX_CODE_CHARS = 50_000;
const MAX_TITLE_CHARS = 200;
const MAX_SUMMARY_CHARS = 5_000;
const MAX_ACTIONS = 12;

const asObject = (value: unknown): Record<string, unknown> | undefined =>
  value && typeof value === 'object' ? (value as Record<string, unknown>) : undefined;

const asString = (value: unknown): string | undefined =>
  typeof value === 'string' && value.trim() ? value : undefined;

const truncate = (value: string | undefined, maxChars: number): string | undefined =>
  value ? value.slice(0, maxChars) : undefined;

const toBlock = (section: string, index: number, value: unknown): HermesCodeBlock | undefined => {
  if (typeof value === 'string' && value.trim()) {
    return { id: `${section}-${index}`, code: value.slice(0, MAX_CODE_CHARS) };
  }
  const obj = asObject(value);
  const code = asString(obj?.code) ?? asString(obj?.text) ?? asString(obj?.body);
  if (!obj || !code) return undefined;
  return {
    id: asString(obj.id) ?? `${section}-${index}`,
    title: truncate(asString(obj.title) ?? asString(obj.name), MAX_TITLE_CHARS),
    language: truncate(asString(obj.language) ?? asString(obj.lang), MAX_TITLE_CHARS),
    code: code.slice(0, MAX_CODE_CHARS),
  };
};

const toArtifact = (value: unknown): HermesArtifact | undefined => {
  const obj = asObject(value);
  if (!obj) return undefined;
  const title = asString(obj.title) ?? asString(obj.name) ?? asString(obj.filename);
  if (!title) return undefined;
  const rawUrl = asString(obj.url);
  const safeUrl = rawUrl ? safeRemoteContentUrl(rawUrl) : undefined;
  if (rawUrl && !safeUrl) return undefined;

  return {
    title: title.slice(0, MAX_TITLE_CHARS),
    type: truncate(asString(obj.type) ?? asString(obj.mimeType), MAX_TITLE_CHARS),
    url: safeUrl,
    summary: truncate(asString(obj.summary) ?? asString(obj.description), MAX_SUMMARY_CHARS),
  };
};

const toAction = (value: unknown, index: number): HermesAgentAction | undefined => {
  const obj = asObject(value);
  if (!obj) return undefined;
  const title = truncate(asString(obj.title) ?? asString(obj.label) ?? asString(obj.name), 80);
  if (!title) return undefined;
  const url = asString(obj.url);
  const safeUrl = url ? safeRemoteContentUrl(url) : undefined;
  if (url && !safeUrl) return undefined;
  return {
    id: asString(obj.id) ?? `action-${index}`,
    title,
    kind: truncate(asString(obj.kind) ?? asString(obj.type), MAX_TITLE_CHARS),
    url: safeUrl,
    prompt: truncate(asString(obj.prompt) ?? asString(obj.command), MAX_SUMMARY_CHARS),
  };
};

const toArray = (value: unknown): unknown[] =>
  Array.isArray(value) ? value.slice(0, MAX_BLOCKS_PER_SECTION) : [];

const normalizePayload = (value: unknown): HermesAgentPayload | undefined => {
  const obj = asObject(value);
  if (!obj) return undefined;
  const title =
    truncate(asString(obj.title) ?? asString(obj.name), MAX_TITLE_CHARS) ?? 'Agent output';
  const summary = truncate(asString(obj.summary) ?? asString(obj.description), MAX_SUMMARY_CHARS);
  const status = truncate(asString(obj.status) ?? asString(obj.state), MAX_TITLE_CHARS);
  const actions = (Array.isArray(obj.actions) ? obj.actions.slice(0, MAX_ACTIONS) : [])
    .map(toAction)
    .filter((item): item is HermesAgentAction => !!item);
  const artifacts = toArray(obj.artifacts)
    .map(toArtifact)
    .filter((item): item is HermesArtifact => !!item);
  const logs = toArray(obj.logs)
    .map((item, index) => toBlock('logs', index, item))
    .filter((item): item is HermesCodeBlock => !!item);
  const code = toArray(obj.code)
    .map((item, index) => toBlock('code', index, item))
    .filter((item): item is HermesCodeBlock => !!item);
  const diffs = toArray(obj.diffs)
    .map((item, index) => toBlock('diffs', index, item))
    .filter((item): item is HermesCodeBlock => !!item);

  if (
    !summary &&
    actions.length === 0 &&
    artifacts.length === 0 &&
    logs.length === 0 &&
    code.length === 0 &&
    diffs.length === 0
  ) {
    return undefined;
  }

  return { title, status, summary, actions, artifacts, logs, code, diffs };
};

export const parseHermesAgentPayload = (
  content: Record<string, unknown>,
  contentKeys: string[] = DEFAULT_HERMES_KEYS
): HermesAgentPayload | undefined => {
  const keys = contentKeys.length > 0 ? contentKeys : DEFAULT_HERMES_KEYS;
  const directPayload = keys.map((key) => normalizePayload(content[key])).find(Boolean);
  if (directPayload) return directPayload;

  const body = asString(content.body);
  if (!body || body.length > MAX_PARSE_BODY_LENGTH) return undefined;
  try {
    const parsed = JSON.parse(body);
    const root = asObject(parsed);
    if (!root) return undefined;
    const nested = keys.map((key) => normalizePayload(root[key])).find(Boolean);
    if (nested) return nested;
    if (root.hermes === true) return normalizePayload(root.payload ?? root.agent);
    return undefined;
  } catch {
    return undefined;
  }
};

const sectionToMarkdown = (title: string, blocks: HermesCodeBlock[]): string[] => {
  if (blocks.length === 0) return [];
  return [
    `## ${title}`,
    ...blocks.map((block) =>
      [block.title ? `### ${block.title}` : '', `\`\`\`${block.language ?? ''}`, block.code, '```']
        .filter(Boolean)
        .join('\n')
    ),
  ].filter(Boolean);
};

export const hermesPayloadToMarkdown = (payload: HermesAgentPayload): string => {
  const lines = [
    `# ${payload.title}`,
    payload.status ? `Status: ${payload.status}` : '',
    payload.summary ?? '',
    payload.actions.length > 0 ? '## Actions' : '',
    ...payload.actions.map((action) => {
      const details = [action.kind, action.prompt].filter(Boolean).join(' - ');
      const label = details ? `${action.title} (${details})` : action.title;
      return action.url ? `- [${label}](${action.url})` : `- ${label}`;
    }),
    payload.artifacts.length > 0 ? '## Artifacts' : '',
    ...payload.artifacts.map((artifact) => {
      const details = [artifact.type, artifact.summary].filter(Boolean).join(' - ');
      const label = details ? `${artifact.title} (${details})` : artifact.title;
      return artifact.url ? `- [${label}](${artifact.url})` : `- ${label}`;
    }),
    ...sectionToMarkdown('Logs', payload.logs),
    ...sectionToMarkdown('Code', payload.code),
    ...sectionToMarkdown('Diffs', payload.diffs),
  ].filter(Boolean);

  return `${lines.join('\n\n')}\n`;
};
