import { invokeDesktopWithAvailability, type DesktopInvokeResult } from '../utils/desktop';
import {
  clearNativeRoomStateProjections,
  publishNativeRoomPowerLevelsProjection,
} from '../features/matrix-dto/nativeRoomStateProjection';

export type NativeRoomPowerLevelsContent = Record<string, unknown>;

export type NativeRoomPowerLevelsSnapshot = {
  status: 'ok';
  roomId: string;
  eventType: 'm.room.power_levels';
  stateKey: '';
  sessionGeneration: number;
  content: NativeRoomPowerLevelsContent;
};

export type NativeRoomPowerLevelsInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
  sessionGeneration?: number;
};

const unavailableMessage = 'Native Matrix room power levels are unavailable.';

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

const isJsonValue = (value: unknown): boolean => {
  if (value === null || typeof value === 'string' || typeof value === 'boolean') return true;
  if (typeof value === 'number') return Number.isFinite(value);
  if (Array.isArray(value)) return value.every(isJsonValue);
  return isRecord(value) && Object.values(value).every(isJsonValue);
};

const isSafePowerLevel = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value);

const isSafeGeneration = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;

const isRoomId = (value: unknown): value is string =>
  typeof value === 'string' && /^![^:\s]+:[^\s]+$/.test(value);

const isPowerLevelMap = (value: unknown): boolean =>
  isRecord(value) && Object.values(value).every(isSafePowerLevel);

const isPowerLevelsContent = (value: unknown): value is NativeRoomPowerLevelsContent => {
  if (!isRecord(value) || !isJsonValue(value)) return false;

  for (const field of [
    'ban',
    'events_default',
    'historical',
    'invite',
    'kick',
    'redact',
    'state_default',
    'users_default',
  ]) {
    if (field in value && !isSafePowerLevel(value[field])) return false;
  }
  for (const field of ['events', 'notifications', 'users']) {
    if (field in value && !isPowerLevelMap(value[field])) return false;
  }
  return true;
};

const invokeSafely = async (
  command: string,
  args: Record<string, unknown> | undefined,
  invoke: NativeRoomPowerLevelsInvoke
): Promise<DesktopInvokeResult<unknown>> => {
  try {
    return await invoke(command, args);
  } catch {
    throw new Error(unavailableMessage);
  }
};

const requireLoggedIn = async (invoke: NativeRoomPowerLevelsInvoke): Promise<number> => {
  try {
    const result = await invokeSafely('matrix_session_snapshot', undefined, invoke);
    if (!result.available || !isRecord(result.value)) throw new Error(unavailableMessage);
    const snapshot = result.value as NativeSessionSnapshot;
    if (snapshot.status !== 'logged_in' || !isSafeGeneration(snapshot.sessionGeneration)) {
      throw new Error(unavailableMessage);
    }
    return snapshot.sessionGeneration;
  } catch {
    clearNativeRoomStateProjections();
    throw new Error(unavailableMessage);
  }
};

export async function readRoomPowerLevelsWithNativeOwner(
  roomId: string,
  nativeSession: boolean,
  invoke: NativeRoomPowerLevelsInvoke = defaultNativeRoomPowerLevelsInvoke
): Promise<NativeRoomPowerLevelsSnapshot | undefined> {
  if (!nativeSession) return undefined;
  if (!isRoomId(roomId)) throw new Error(unavailableMessage);

  const sessionGeneration = await requireLoggedIn(invoke);
  const result = await invokeSafely('matrix_room_power_levels_snapshot', { roomId }, invoke);
  if (!result.available || !isRecord(result.value)) throw new Error(unavailableMessage);

  const value = result.value;
  if (
    value.status !== 'ok' ||
    value.roomId !== roomId ||
    value.eventType !== 'm.room.power_levels' ||
    value.stateKey !== '' ||
    value.sessionGeneration !== sessionGeneration ||
    !isSafeGeneration(value.sessionGeneration) ||
    !isPowerLevelsContent(value.content)
  ) {
    throw new Error(unavailableMessage);
  }

  const snapshot = value as NativeRoomPowerLevelsSnapshot;
  publishNativeRoomPowerLevelsProjection(
    snapshot.roomId,
    snapshot.sessionGeneration,
    snapshot.content
  );
  return snapshot;
}

export const defaultNativeRoomPowerLevelsInvoke: NativeRoomPowerLevelsInvoke = (command, args) =>
  invokeDesktopWithAvailability(command, args);
