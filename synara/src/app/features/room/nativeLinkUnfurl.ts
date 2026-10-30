import { find } from 'linkifyjs';
import type { DesktopInvokeResult } from '../../utils/desktop';
import { hasForbiddenWireFields, isObject } from '../matrix-dto/parseUtil';
import { projectNativeFormattedBody } from './nativeTimelinePresentationProjection';

export const COMPOSER_UNFURL_DEBOUNCE_MS = 400;

const ATTACHMENT_MESSAGE_TYPES = new Set(['image', 'file', 'audio', 'video']);
const HANDLE_PATTERN = /^[0-9a-f]{64}$/i;
const MAX_PREVIEW_TEXT = 2048;

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

/**
 * A link preview this device fetched (`desktop_link_preview`). The image is
 * served by the desktop shell on `synara-media:` at `link-preview/<id>`.
 */
export type NativeLinkPreview = {
  url: string;
  title: string;
  description?: string;
  siteName?: string;
  imageHandleId?: string;
};

const inFlight = new Map<string, Promise<NativeLinkPreview | null>>();

const invokeSafely = async (
  command: string,
  args: Record<string, unknown> | undefined,
  invoke: NativeInvoke
): Promise<DesktopInvokeResult<unknown>> => {
  try {
    return await invoke(command, args);
  } catch {
    return { available: false };
  }
};

export const isPreviewCandidateUrl = (value: string): boolean => {
  if (!value || value.length > 2048 || value.trim() !== value) return false;
  try {
    const parsed = new URL(value);
    if (parsed.username || parsed.password) return false;
    return parsed.protocol === 'http:' || parsed.protocol === 'https:';
  } catch {
    return false;
  }
};

const urlsInText = (text: string): string[] => {
  if (!text.trim()) return [];
  return find(text)
    .filter((match) => match.type === 'url' && isPreviewCandidateUrl(match.href))
    .map((match) => match.href);
};

export const trailingComposerPreviewUrl = (plainText: string): string | undefined => {
  const trimmed = plainText.trim();
  if (!trimmed) return undefined;
  const matches = find(trimmed).filter(
    (match) => match.type === 'url' && isPreviewCandidateUrl(match.href)
  );
  if (matches.length !== 1) return undefined;
  const lastToken = trimmed.split(/\s+/).pop();
  if (!lastToken) return undefined;
  const [match] = matches;
  const hrefNoSlash = match.href.replace(/\/$/, '');
  if (lastToken !== match.href && lastToken !== match.value && lastToken !== hrefNoSlash) {
    return undefined;
  }
  return match.href;
};

export const firstTimelinePreviewUrl = (
  body: string,
  formattedBody?: string
): string | undefined => {
  if (formattedBody) {
    const projection = projectNativeFormattedBody(formattedBody);
    const formatted = projection?.links.find(isPreviewCandidateUrl);
    if (formatted) return formatted;
  }
  return urlsInText(body)[0];
};

export const shouldSkipMessageUnfurl = (input: {
  messageType?: string;
  hasMedia?: boolean;
  skip?: boolean;
}): boolean =>
  Boolean(
    input.skip ||
    input.hasMedia ||
    (input.messageType && ATTACHMENT_MESSAGE_TYPES.has(input.messageType))
  );

/** The URL a message row previews: its first link, unless a skip rule applies. */
export const timelinePreviewUrl = (input: {
  enabled: boolean;
  body: string;
  formattedBody?: string;
  messageType?: string;
  hasMedia?: boolean;
  skip?: boolean;
}): string | undefined => {
  if (!input.enabled || shouldSkipMessageUnfurl(input)) return undefined;
  return firstTimelinePreviewUrl(input.body, input.formattedBody);
};

/** The desktop shell's answer for `url`, or `null` when it is not a valid preview of it. */
export const parseLinkPreview = (value: unknown, url: string): NativeLinkPreview | null => {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const optional = ['description', 'siteName', 'imageHandleId'] as const;
  const allowed = new Set<string>(['url', 'title', ...optional]);
  if (!Object.keys(value).every((key) => allowed.has(key)) || value.url !== url) return null;
  if (
    typeof value.title !== 'string' ||
    value.title.length === 0 ||
    value.title.length > MAX_PREVIEW_TEXT
  ) {
    return null;
  }
  for (const key of optional) {
    if (!(key in value)) continue;
    const field = value[key];
    if (typeof field !== 'string' || field.length === 0 || field.length > MAX_PREVIEW_TEXT) {
      return null;
    }
  }
  if ('imageHandleId' in value && !HANDLE_PATTERN.test(String(value.imageHandleId))) return null;
  return value as NativeLinkPreview;
};

/** Where the desktop shell serves a preview's image on `synara-media:`. */
export const linkPreviewImagePath = (handleId: string): string => `link-preview/${handleId}`;

/**
 * A preview of `url` fetched from this device: the desktop shell requests the
 * page itself and nothing goes through the homeserver, so encrypted rooms
 * preview too. `null` when the Link previews setting is off, outside the
 * desktop app, or when the page has no preview.
 */
export async function fetchDeviceLinkPreview(input: {
  url: string;
  enabled: boolean;
  desktopAvailable: boolean;
  invoke: NativeInvoke;
}): Promise<NativeLinkPreview | null> {
  if (!input.enabled || !input.desktopAvailable || !isPreviewCandidateUrl(input.url)) {
    return null;
  }
  const pending = inFlight.get(input.url);
  if (pending) return pending;
  const task = (async () => {
    const result = await invokeSafely('desktop_link_preview', { url: input.url }, input.invoke);
    if (!result.available) return null;
    return parseLinkPreview(result.value, input.url);
  })();
  inFlight.set(input.url, task);
  try {
    return await task;
  } finally {
    inFlight.delete(input.url);
  }
}
