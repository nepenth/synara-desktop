import type { DesktopEvent, DesktopInvokeResult, DesktopUnlisten } from '../../../utils/desktop';
import { hasForbiddenWireFields, isObject } from '../../matrix-dto/parseUtil';

export type NativeRoomJoinRule =
  | 'public'
  | 'knock'
  | 'invite'
  | 'restricted'
  | 'knock_restricted'
  | 'private';

export type NativeRoomJoinRuleSnapshot = {
  status: 'ok';
  roomId: string;
  sessionGeneration: number;
  joinRule: NativeRoomJoinRule;
};

export type NativeRoomJoinRuleUpdate =
  | {
      status: 'ready';
      roomId: string;
      sessionGeneration: number;
      joinRule: NativeRoomJoinRule;
    }
  | {
      status: 'unavailable';
      roomId: string;
      sessionGeneration: number;
    };

type NativeSessionSnapshot = {
  status: 'logged_in';
  user_id: string;
  device_id: string;
  homeserver_url: string;
  sessionGeneration: number;
};

export type NativeRoomJoinRuleState =
  | { status: 'loading'; userId?: string }
  | {
      status: 'ready';
      userId: string;
      snapshot: NativeRoomJoinRuleSnapshot;
    }
  | { status: 'error'; userId?: string; error: Error };

export type NativeRoomJoinRuleInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

export type NativeRoomJoinRuleListen = <T>(
  event: string,
  handler: (event: DesktopEvent<T>) => void
) => Promise<DesktopUnlisten | undefined>;

export type NativeRoomJoinRuleDependencies = {
  desktopAvailable: boolean;
  invoke: NativeRoomJoinRuleInvoke;
  listen: NativeRoomJoinRuleListen;
};

export const ROOM_JOIN_RULE_UPDATED_EVENT = 'matrix-room-join-rule-updated';
const unavailableMessage = 'Native Matrix room join rule is unavailable.';
const roomIdPattern = /^![^:\s]+:[^\s]+$/;

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

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

const isSafeGeneration = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value > 0;

const isRoomId = (value: unknown): value is string =>
  typeof value === 'string' && value.length <= 512 && roomIdPattern.test(value);

const isUserId = (value: unknown): value is string =>
  typeof value === 'string' && value.length <= 255 && /^@[^:\s]+:[^\s]+$/.test(value);

const isJoinRule = (value: unknown): value is NativeRoomJoinRule =>
  value === 'public' ||
  value === 'knock' ||
  value === 'invite' ||
  value === 'restricted' ||
  value === 'knock_restricted' ||
  value === 'private';

const invokeSafely = async (
  dependencies: NativeRoomJoinRuleDependencies,
  command: string,
  args?: Record<string, unknown>
): Promise<DesktopInvokeResult<unknown>> => {
  try {
    return await dependencies.invoke(command, args);
  } catch {
    throw new Error(unavailableMessage);
  }
};

const parseSession = (value: unknown): NativeSessionSnapshot => {
  if (
    !isRecord(value) ||
    hasForbiddenWireFields(value) ||
    !hasExactKeys(value, [
      'status',
      'user_id',
      'device_id',
      'homeserver_url',
      'sessionGeneration',
    ]) ||
    value.status !== 'logged_in' ||
    !isUserId(value.user_id) ||
    typeof value.device_id !== 'string' ||
    value.device_id.length === 0 ||
    value.device_id.length > 255 ||
    /\s/.test(value.device_id) ||
    typeof value.homeserver_url !== 'string' ||
    value.homeserver_url.length === 0 ||
    value.homeserver_url.length > 2_048 ||
    /\s/.test(value.homeserver_url) ||
    !isSafeGeneration(value.sessionGeneration)
  ) {
    throw new Error(unavailableMessage);
  }
  return value as NativeSessionSnapshot;
};

export const parseRoomJoinRuleSnapshot = (
  value: unknown,
  roomId: string,
  sessionGeneration: number
): NativeRoomJoinRuleSnapshot => {
  if (
    !isObject(value) ||
    hasForbiddenWireFields(value) ||
    !hasExactKeys(value, ['status', 'roomId', 'sessionGeneration', 'joinRule']) ||
    value.status !== 'ok' ||
    value.roomId !== roomId ||
    value.sessionGeneration !== sessionGeneration ||
    !isSafeGeneration(value.sessionGeneration) ||
    !isJoinRule(value.joinRule)
  ) {
    throw new Error(unavailableMessage);
  }
  return value as NativeRoomJoinRuleSnapshot;
};

export const parseRoomJoinRuleUpdate = (value: unknown): NativeRoomJoinRuleUpdate | undefined => {
  if (
    !isRecord(value) ||
    hasForbiddenWireFields(value) ||
    typeof value.status !== 'string' ||
    !isRoomId(value.roomId) ||
    !isSafeGeneration(value.sessionGeneration)
  ) {
    return undefined;
  }
  if (value.status === 'ready') {
    if (
      !hasExactKeys(value, ['status', 'roomId', 'sessionGeneration', 'joinRule']) ||
      !isJoinRule(value.joinRule)
    ) {
      return undefined;
    }
    return value as NativeRoomJoinRuleUpdate;
  }
  if (
    value.status === 'unavailable' &&
    hasExactKeys(value, ['status', 'roomId', 'sessionGeneration'])
  ) {
    return value as NativeRoomJoinRuleUpdate;
  }
  return undefined;
};

const parseEvent = <T>(event: DesktopEvent<T>): unknown => event.payload;

/**
 * Testable native owner for one mounted RoomPublish gate. The listener is
 * installed before the session snapshot and the initial room read. There is
 * deliberately no JS room-state fallback when native state is unavailable.
 */
export const createNativeRoomJoinRuleOwner = async (
  roomId: string,
  dependencies: NativeRoomJoinRuleDependencies,
  onState: (state: NativeRoomJoinRuleState) => void
): Promise<() => void> => {
  let disposed = false;
  let unlisten: DesktopUnlisten | undefined;
  let expectedGeneration: number | undefined;
  let userId: string | undefined;

  const failClosed = () => {
    if (!disposed) onState({ status: 'error', userId, error: new Error(unavailableMessage) });
  };

  const dispose = () => {
    disposed = true;
    void unlisten?.();
    unlisten = undefined;
  };

  if (!dependencies.desktopAvailable || !isRoomId(roomId)) {
    failClosed();
    return dispose;
  }

  onState({ status: 'loading' });
  try {
    const listener = await dependencies.listen<NativeRoomJoinRuleUpdate>(
      ROOM_JOIN_RULE_UPDATED_EVENT,
      (event) => {
        if (disposed) return;
        const update = parseRoomJoinRuleUpdate(parseEvent(event));
        if (
          !update ||
          update.roomId !== roomId ||
          expectedGeneration === undefined ||
          update.sessionGeneration !== expectedGeneration
        ) {
          // Clear mounted state on malformed/stale updates; never retain a
          // previous-generation publishable rule after an unsafe signal.
          failClosed();
          return;
        }
        if (update.status === 'unavailable') {
          failClosed();
          return;
        }
        onState({
          status: 'ready',
          userId: userId as string,
          snapshot: {
            status: 'ok',
            roomId: update.roomId,
            sessionGeneration: update.sessionGeneration,
            joinRule: update.joinRule,
          },
        });
      }
    );
    if (!listener) throw new Error(unavailableMessage);
    unlisten = listener;
    if (disposed) return dispose;

    const sessionResult = await invokeSafely(dependencies, 'matrix_session_snapshot');
    if (!sessionResult.available) throw new Error(unavailableMessage);
    const session = parseSession(sessionResult.value);
    userId = session.user_id;
    expectedGeneration = session.sessionGeneration;

    const snapshotResult = await invokeSafely(dependencies, 'matrix_room_join_rule_snapshot', {
      roomId,
      sessionGeneration: expectedGeneration,
    });
    if (disposed || !snapshotResult.available) throw new Error(unavailableMessage);
    const snapshot = parseRoomJoinRuleSnapshot(snapshotResult.value, roomId, expectedGeneration);
    if (!disposed) onState({ status: 'ready', userId, snapshot });
  } catch {
    failClosed();
  }
  return dispose;
};
