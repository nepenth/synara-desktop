/**
 * Privacy-safe MSC4426 status / in-call parser and IPC.
 *
 * Own writes are emoji+text only. This is not `m.presence` and never
 * sends `state` or `set_call`.
 */

import { useEffect, useState } from 'react';
import {
  invokeDesktopWithAvailability,
  isSynaraDesktop,
  type DesktopInvokeResult,
} from '../../utils/desktop';
import { getSessionBootstrapResult } from '../../state/sessionBootstrap';

export const MAX_STATUS_EMOJI_BYTES = 32;
export const MAX_STATUS_TEXT_BYTES = 256;

export type NativeUserStatus = {
  emoji: string;
  text: string;
};

export type NativeInCall = {
  callJoinedTs?: number;
};

export type NativeUserStatusSnapshot = {
  sessionGeneration: number;
  userId: string;
  userStatus?: NativeUserStatus;
  inCall?: NativeInCall;
};

export type NativeUserStatusInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

export type NativeUserStatusDependencies = {
  desktopNativeSession: boolean;
  invoke: NativeUserStatusInvoke;
};

const SNAPSHOT_KEYS = ['sessionGeneration', 'userId', 'userStatus', 'inCall'];
const STATUS_KEYS = ['emoji', 'text'];
const IN_CALL_KEYS = ['callJoinedTs'];
const WRITE_ACK_KEYS = ['status'];
const PRESENCE_STATE_KEYS = ['state', 'statusMsg', 'currentlyActive'];

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

const onlyKnownKeys = (value: Record<string, unknown>, keys: string[]): boolean =>
  Object.keys(value).every((key) => keys.includes(key));

const hasPresenceStateKey = (value: Record<string, unknown>): boolean =>
  PRESENCE_STATE_KEYS.some((key) => key in value);

const isSafeGeneration = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;

const isUserId = (value: unknown): value is string =>
  typeof value === 'string' && value.length <= 255 && /^@[^:\s]+:[^\s]+$/.test(value);

const utf8ByteLength = (value: string): number => new TextEncoder().encode(value).length;

const isBoundedEmoji = (value: unknown): value is string =>
  typeof value === 'string' && utf8ByteLength(value) <= MAX_STATUS_EMOJI_BYTES;

const isBoundedText = (value: unknown): value is string =>
  typeof value === 'string' && utf8ByteLength(value) <= MAX_STATUS_TEXT_BYTES;

const isSafeTimestamp = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;

export function parseUserStatusField(value: unknown): NativeUserStatus | null {
  if (!isRecord(value) || !onlyKnownKeys(value, STATUS_KEYS) || hasPresenceStateKey(value)) {
    return null;
  }
  if (!isBoundedEmoji(value.emoji) || !isBoundedText(value.text)) return null;
  if (value.emoji.length === 0 && value.text.length === 0) return null;
  return { emoji: value.emoji, text: value.text };
}

export function parseInCallField(value: unknown): NativeInCall | null {
  if (!isRecord(value) || !onlyKnownKeys(value, IN_CALL_KEYS) || hasPresenceStateKey(value)) {
    return null;
  }
  if (value.callJoinedTs === undefined) return {};
  if (!isSafeTimestamp(value.callJoinedTs)) return null;
  return { callJoinedTs: value.callJoinedTs };
}

export function parseUserStatusSnapshot(
  value: unknown,
  requestedUserId?: string
): NativeUserStatusSnapshot | null {
  if (!isRecord(value) || !onlyKnownKeys(value, SNAPSHOT_KEYS) || hasPresenceStateKey(value)) {
    return null;
  }
  if (!isSafeGeneration(value.sessionGeneration) || !isUserId(value.userId)) return null;
  if (requestedUserId !== undefined && value.userId !== requestedUserId) return null;
  let userStatus: NativeUserStatus | undefined;
  if (value.userStatus !== undefined) {
    const parsed = parseUserStatusField(value.userStatus);
    if (!parsed) return null;
    userStatus = parsed;
  }
  let inCall: NativeInCall | undefined;
  if (value.inCall !== undefined) {
    const parsed = parseInCallField(value.inCall);
    if (!parsed) return null;
    inCall = parsed;
  }
  return {
    sessionGeneration: value.sessionGeneration,
    userId: value.userId,
    ...(userStatus ? { userStatus } : {}),
    ...(inCall ? { inCall } : {}),
  };
}

export function parseUserStatusWriteAck(value: unknown): boolean {
  return (
    isRecord(value) &&
    onlyKnownKeys(value, WRITE_ACK_KEYS) &&
    value.status === 'ok' &&
    !('emoji' in value) &&
    !('text' in value) &&
    !hasPresenceStateKey(value)
  );
}

function resolveDependencies(
  deps: Partial<NativeUserStatusDependencies>
): NativeUserStatusDependencies {
  return {
    desktopNativeSession:
      deps.desktopNativeSession ??
      (isSynaraDesktop() && getSessionBootstrapResult().source === 'native'),
    invoke: deps.invoke ?? ((command, args) => invokeDesktopWithAvailability(command, args)),
  };
}

export type UserStatusWriteFailure = 'unsupported' | 'invalid' | 'unavailable';

export class UserStatusWriteError extends Error {
  readonly kind: UserStatusWriteFailure;

  constructor(kind: UserStatusWriteFailure) {
    super(
      kind === 'unsupported'
        ? 'This homeserver does not support user status.'
        : kind === 'invalid'
          ? 'Status emoji or text is too long.'
          : 'Native user status is unavailable.'
    );
    this.kind = kind;
  }
}

function writeFailureKind(error: unknown): UserStatusWriteFailure {
  if (!error || typeof error !== 'object') return 'unavailable';
  const record = error as Record<string, unknown>;
  const diagnostic = typeof record.diagnosticId === 'string' ? record.diagnosticId : '';
  const message = typeof record.message === 'string' ? record.message : '';
  if (diagnostic === 'v-user-status-unsupported' || message.includes('does not support')) {
    return 'unsupported';
  }
  if (
    diagnostic === 'v-user-status-emoji-cap' ||
    diagnostic === 'v-user-status-text-cap' ||
    diagnostic.includes('invalid-payload')
  ) {
    return 'invalid';
  }
  return 'unavailable';
}

async function invokeWrite(
  resolved: NativeUserStatusDependencies,
  command: 'matrix_user_status_set' | 'matrix_user_status_clear',
  args?: Record<string, unknown>
): Promise<void> {
  try {
    const result = await resolved.invoke(command, args);
    if (!result.available || !parseUserStatusWriteAck(result.value)) {
      throw new UserStatusWriteError('unavailable');
    }
  } catch (error) {
    if (error instanceof UserStatusWriteError) throw error;
    throw new UserStatusWriteError(writeFailureKind(error));
  }
}

export async function snapshotUserStatusNative(
  userId: string,
  deps: Partial<NativeUserStatusDependencies> = {}
): Promise<NativeUserStatusSnapshot | null> {
  const resolved = resolveDependencies(deps);
  if (!resolved.desktopNativeSession || !isUserId(userId)) return null;
  try {
    const result = await resolved.invoke('matrix_user_status_snapshot', { userId });
    if (!result.available) return null;
    return parseUserStatusSnapshot(result.value, userId);
  } catch {
    return null;
  }
}

export async function setOwnUserStatusNative(
  emoji: string,
  text: string,
  deps: Partial<NativeUserStatusDependencies> = {}
): Promise<void> {
  if (utf8ByteLength(emoji) > MAX_STATUS_EMOJI_BYTES || utf8ByteLength(text) > MAX_STATUS_TEXT_BYTES) {
    throw new UserStatusWriteError('invalid');
  }
  const resolved = resolveDependencies(deps);
  if (!resolved.desktopNativeSession) {
    throw new UserStatusWriteError('unavailable');
  }
  await invokeWrite(resolved, 'matrix_user_status_set', { emoji, text });
}

export async function clearOwnUserStatusNative(
  deps: Partial<NativeUserStatusDependencies> = {}
): Promise<void> {
  const resolved = resolveDependencies(deps);
  if (!resolved.desktopNativeSession) {
    throw new UserStatusWriteError('unavailable');
  }
  await invokeWrite(resolved, 'matrix_user_status_clear');
}

/** Lazy peer/own snapshot. Missing fields stay absent; never presence. */
export function useNativeUserStatus(userId: string | undefined): NativeUserStatusSnapshot | undefined {
  const [snapshot, setSnapshot] = useState<NativeUserStatusSnapshot | undefined>();

  useEffect(() => {
    setSnapshot(undefined);
    if (!userId) return undefined;
    let active = true;
    void snapshotUserStatusNative(userId).then((next) => {
      if (active) setSnapshot(next ?? undefined);
    });
    return () => {
      active = false;
    };
  }, [userId]);

  return snapshot;
}
