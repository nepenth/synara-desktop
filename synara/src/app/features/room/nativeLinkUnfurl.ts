import { find } from 'linkifyjs';
import type { DesktopInvokeResult } from '../../utils/desktop';
import { hasForbiddenWireFields, isObject } from '../matrix-dto/parseUtil';
import { projectNativeFormattedBody } from './nativeTimelinePresentationProjection';

export const COMPOSER_UNFURL_DEBOUNCE_MS = 400;

const ATTACHMENT_MESSAGE_TYPES = new Set(['image', 'file', 'audio', 'video']);
const HANDLE_PATTERN = /^[0-9a-f]{64}$/i;
const ROOM_ID_PATTERN = /^![^:\s]+:[^\s]+$/;

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

export type NativeMediaPreview = {
  status: 'ok';
  roomId: string;
  sessionGeneration: number;
  url: string;
  title?: string;
  description?: string;
  siteName?: string;
  thumbnailHandleId?: string;
};

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
  sessionGeneration?: number;
};

const unavailableMessage = 'Native Matrix URL preview is unavailable.';

const inFlight = new Map<string, Promise<NativeMediaPreview | null>>();

const isSafeGeneration = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value > 0;

const isRoomId = (value: unknown): value is string =>
  typeof value === 'string' && value.length <= 512 && ROOM_ID_PATTERN.test(value);

const hasExactKeys = (
  value: Record<string, unknown>,
  keys: readonly string[],
  requiredKeys: readonly string[] = keys
): boolean => {
  const allowed = new Set(keys);
  return (
    Object.keys(value).every((key) => allowed.has(key)) && requiredKeys.every((key) => key in value)
  );
};

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

const requirePreviewSession = async (invoke: NativeInvoke): Promise<number | undefined> => {
  const result = await invokeSafely('matrix_session_snapshot', undefined, invoke);
  if (!result.available || !isObject(result.value) || hasForbiddenWireFields(result.value)) {
    return undefined;
  }
  const snapshot = result.value as NativeSessionSnapshot & Record<string, unknown>;
  if (snapshot.status !== 'logged_in') return undefined;
  if (
    !hasExactKeys(snapshot, [
      'status',
      'user_id',
      'device_id',
      'homeserver_url',
      'sessionGeneration',
    ]) ||
    !isSafeGeneration(snapshot.sessionGeneration)
  ) {
    return undefined;
  }
  return snapshot.sessionGeneration;
};

export const parseMediaPreviewSnapshot = (
  value: unknown,
  roomId: string,
  sessionGeneration: number,
  url: string
): NativeMediaPreview | null => {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  if ('imageMxc' in value || 'og:image' in value || 'image_mxc' in value) {
    throw new Error(unavailableMessage);
  }
  if (typeof value.status !== 'string') throw new Error(unavailableMessage);
  if (value.status === 'skipped' || value.status === 'unavailable') return null;
  const required = ['status', 'roomId', 'sessionGeneration', 'url'] as const;
  const optional = ['title', 'description', 'siteName', 'thumbnailHandleId'] as const;
  if (
    !hasExactKeys(value, [...required, ...optional], required) ||
    value.status !== 'ok' ||
    value.roomId !== roomId ||
    value.sessionGeneration !== sessionGeneration ||
    value.url !== url ||
    !isSafeGeneration(value.sessionGeneration)
  ) {
    throw new Error(unavailableMessage);
  }
  for (const key of optional) {
    if (key in value && (typeof value[key] !== 'string' || value[key].length === 0)) {
      throw new Error(unavailableMessage);
    }
  }
  if ('thumbnailHandleId' in value && !HANDLE_PATTERN.test(String(value.thumbnailHandleId))) {
    throw new Error(unavailableMessage);
  }
  const dumped = JSON.stringify(value);
  if (dumped.includes('mxc://') || dumped.includes('access_token')) {
    throw new Error(unavailableMessage);
  }
  return value as NativeMediaPreview;
};

export async function fetchMediaPreviewWithNativeOwner(input: {
  roomId: string;
  url: string;
  ts?: number;
  encryptionStatus?: string;
  desktopAvailable: boolean;
  invoke: NativeInvoke;
}): Promise<NativeMediaPreview | null> {
  if (
    !input.desktopAvailable ||
    !isRoomId(input.roomId) ||
    !isPreviewCandidateUrl(input.url) ||
    input.encryptionStatus !== 'not_encrypted'
  ) {
    return null;
  }
  const sessionGeneration = await requirePreviewSession(input.invoke);
  if (sessionGeneration === undefined) return null;
  const cacheKey = `${sessionGeneration}\0${input.roomId}\0${input.url}\0${input.ts ?? ''}`;
  const pending = inFlight.get(cacheKey);
  if (pending) return pending;
  const task = (async () => {
    const args: Record<string, unknown> = {
      roomId: input.roomId,
      sessionGeneration,
      url: input.url,
    };
    if (input.ts !== undefined) args.ts = input.ts;
    const result = await invokeSafely('matrix_media_preview', args, input.invoke);
    if (!result.available) return null;
    try {
      return parseMediaPreviewSnapshot(result.value, input.roomId, sessionGeneration, input.url);
    } catch {
      return null;
    }
  })();
  inFlight.set(cacheKey, task);
  try {
    return await task;
  } finally {
    inFlight.delete(cacheKey);
  }
}
