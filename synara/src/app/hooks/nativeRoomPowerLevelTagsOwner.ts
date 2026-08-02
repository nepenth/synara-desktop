import { invokeDesktopWithAvailability, type DesktopInvokeResult } from '../utils/desktop';
import type { MemberPowerTag } from '../../types/matrix/room';

export type NativeRoomPowerLevelTagsContent = Record<string, MemberPowerTag>;

export type NativeRoomPowerLevelTagsSnapshot = {
  status: 'ok';
  roomId: string;
  eventType: 'in.synara.room.power_level_tags';
  stateKey: '';
  sessionGeneration: number;
  content: NativeRoomPowerLevelTagsContent;
};

export type NativeRoomPowerLevelTagsInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
  sessionGeneration?: number;
};

const unavailableMessage = 'Native Matrix room power-level tags are unavailable.';
const maxTextLength = 4 * 1024;

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

const isJsonValue = (value: unknown): boolean => {
  if (value === null || typeof value === 'string' || typeof value === 'boolean') return true;
  if (typeof value === 'number') return Number.isFinite(value);
  if (Array.isArray(value)) return value.every(isJsonValue);
  return isRecord(value) && Object.values(value).every(isJsonValue);
};

const isSafeInteger = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value);

const isSafeGeneration = (value: unknown): value is number => isSafeInteger(value) && value >= 0;

const isRoomId = (value: unknown): value is string =>
  typeof value === 'string' && /^![^:\s]+:[^\s]+$/.test(value);

const isBoundedString = (value: unknown, required: boolean): value is string =>
  typeof value === 'string' &&
  value.length <= maxTextLength &&
  (required ? value.trim().length > 0 : true);

const isPowerLevelTagIcon = (value: unknown): boolean => {
  if (!isRecord(value)) return false;
  if ('key' in value && !isBoundedString(value.key, false)) return false;
  if (!('info' in value)) return true;
  if (!isRecord(value.info)) return false;
  for (const [key, fieldValue] of Object.entries(value.info)) {
    if (key === 'w' || key === 'h' || key === 'size') {
      if (!isSafeInteger(fieldValue) || fieldValue < 0) return false;
    } else if (
      (key === 'mimetype' || key === 'xyz.amorgan.blurhash') &&
      !isBoundedString(fieldValue, false)
    ) {
      return false;
    } else if (key !== 'mimetype' && key !== 'xyz.amorgan.blurhash') {
      return false;
    }
  }
  return true;
};

const isPowerLevelTagsContent = (value: unknown): value is NativeRoomPowerLevelTagsContent => {
  if (!isRecord(value) || !isJsonValue(value)) return false;
  return Object.entries(value).every(([power, tag]) => {
    const numericPower = Number(power);
    if (!Number.isSafeInteger(numericPower) || String(numericPower) !== power) return false;
    if (!isRecord(tag) || !isBoundedString(tag.name, true)) return false;
    if ('color' in tag && !isBoundedString(tag.color, false)) return false;
    if ('icon' in tag && !isPowerLevelTagIcon(tag.icon)) return false;
    return Object.keys(tag).every((key) => key === 'name' || key === 'color' || key === 'icon');
  });
};

const invokeSafely = async (
  command: string,
  args: Record<string, unknown> | undefined,
  invoke: NativeRoomPowerLevelTagsInvoke
): Promise<DesktopInvokeResult<unknown>> => {
  try {
    return await invoke(command, args);
  } catch {
    throw new Error(unavailableMessage);
  }
};

const requireLoggedIn = async (invoke: NativeRoomPowerLevelTagsInvoke): Promise<number> => {
  const result = await invokeSafely('matrix_session_snapshot', undefined, invoke);
  if (!result.available || !isRecord(result.value)) throw new Error(unavailableMessage);
  const snapshot = result.value as NativeSessionSnapshot;
  if (snapshot.status !== 'logged_in' || !isSafeGeneration(snapshot.sessionGeneration)) {
    throw new Error(unavailableMessage);
  }
  return snapshot.sessionGeneration;
};

export async function readRoomPowerLevelTagsWithNativeOwner(
  roomId: string,
  nativeSession: boolean,
  invoke: NativeRoomPowerLevelTagsInvoke = defaultNativeRoomPowerLevelTagsInvoke
): Promise<NativeRoomPowerLevelTagsSnapshot | undefined> {
  if (!nativeSession) return undefined;
  if (!isRoomId(roomId)) throw new Error(unavailableMessage);

  const sessionGeneration = await requireLoggedIn(invoke);
  const result = await invokeSafely('matrix_room_power_level_tags_snapshot', { roomId }, invoke);
  if (!result.available || !isRecord(result.value)) throw new Error(unavailableMessage);

  const value = result.value;
  if (
    value.status !== 'ok' ||
    value.roomId !== roomId ||
    value.eventType !== 'in.synara.room.power_level_tags' ||
    value.stateKey !== '' ||
    value.sessionGeneration !== sessionGeneration ||
    !isSafeGeneration(value.sessionGeneration) ||
    !isPowerLevelTagsContent(value.content)
  ) {
    throw new Error(unavailableMessage);
  }

  return value as NativeRoomPowerLevelTagsSnapshot;
}

export const defaultNativeRoomPowerLevelTagsInvoke: NativeRoomPowerLevelTagsInvoke = (
  command,
  args
) => invokeDesktopWithAvailability(command, args);
