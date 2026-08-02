import { invokeDesktopWithAvailability, type DesktopInvokeResult } from '../../../utils/desktop';
import type { MemberPowerTag } from '../../../../types/matrix/room';

export type RoomPowerLevelsContent = Record<string, unknown>;
export type PowerLevelTagsContent = Record<string, MemberPowerTag>;

export type NativePowerLevelWriteResult<TContent> = {
  status: 'ok';
  roomId: string;
  eventType: 'm.room.power_levels' | 'in.synara.room.power_level_tags';
  stateKey: '';
  sessionGeneration: number;
  content: TContent;
};

export type NativeRoomPowerLevelsInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
  sessionGeneration?: number;
};

const unavailableMessage = 'Native Matrix room power-level writes are unavailable.';
const maxTextLength = 4 * 1024;

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

const isBoundedString = (value: unknown, required: boolean): value is string =>
  typeof value === 'string' &&
  value.length <= maxTextLength &&
  (required ? value.trim().length > 0 : true);

const isPowerLevelMap = (value: unknown): boolean =>
  isRecord(value) && Object.values(value).every(isSafePowerLevel);

const isRoomPowerLevelsContent = (value: unknown): value is RoomPowerLevelsContent => {
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

const isPowerLevelTagIcon = (value: unknown): boolean => {
  if (!isRecord(value)) return false;
  if ('key' in value && !isBoundedString(value.key, false)) return false;
  if (!('info' in value)) return true;
  if (!isRecord(value.info)) return false;
  for (const [key, fieldValue] of Object.entries(value.info)) {
    if (key === 'w' || key === 'h' || key === 'size') {
      if (!isSafePowerLevel(fieldValue) || fieldValue < 0) return false;
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

const isPowerLevelTagsContent = (value: unknown): value is PowerLevelTagsContent => {
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

const canonicalJson = (value: unknown): unknown => {
  if (Array.isArray(value)) return value.map(canonicalJson);
  if (!isRecord(value)) return value;
  return Object.fromEntries(
    Object.keys(value)
      .sort()
      .map((key) => [key, canonicalJson(value[key])])
  );
};

const sameJson = (left: unknown, right: unknown): boolean =>
  JSON.stringify(canonicalJson(left)) === JSON.stringify(canonicalJson(right));

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
  const result = await invokeSafely('matrix_session_snapshot', undefined, invoke);
  if (!result.available) throw new Error(unavailableMessage);
  const snapshot = result.value as NativeSessionSnapshot | undefined;
  if (snapshot?.status !== 'logged_in' || !isSafeGeneration(snapshot.sessionGeneration)) {
    throw new Error(unavailableMessage);
  }
  return snapshot.sessionGeneration;
};

const writePowerLevelState = async <TContent>(
  roomId: string,
  content: TContent,
  eventType: NativePowerLevelWriteResult<TContent>['eventType'],
  command: string,
  desktopAvailable: boolean,
  invoke: NativeRoomPowerLevelsInvoke,
  validate: (value: unknown) => value is TContent
): Promise<NativePowerLevelWriteResult<TContent>> => {
  if (!desktopAvailable || !roomId.trim() || roomId.trim() !== roomId || !validate(content)) {
    throw new Error(unavailableMessage);
  }

  const sessionGeneration = await requireLoggedIn(invoke);
  const result = await invokeSafely(command, { roomId, content }, invoke);
  if (!result.available || !isRecord(result.value)) throw new Error(unavailableMessage);

  const value = result.value;
  if (
    value.status !== 'ok' ||
    value.roomId !== roomId ||
    value.eventType !== eventType ||
    value.stateKey !== '' ||
    value.sessionGeneration !== sessionGeneration ||
    !isSafeGeneration(value.sessionGeneration) ||
    !sameJson(value.content, content)
  ) {
    throw new Error(unavailableMessage);
  }

  return value as NativePowerLevelWriteResult<TContent>;
};

/** Sole native desktop owner for complete room power-level replacements. */
export function setRoomPowerLevelsWithNativeOwner(
  roomId: string,
  content: RoomPowerLevelsContent,
  desktopAvailable: boolean,
  invoke: NativeRoomPowerLevelsInvoke
): Promise<NativePowerLevelWriteResult<RoomPowerLevelsContent>> {
  return writePowerLevelState(
    roomId,
    content,
    'm.room.power_levels',
    'matrix_room_set_power_levels',
    desktopAvailable,
    invoke,
    isRoomPowerLevelsContent
  );
}

/** Sole native desktop owner for complete custom power-level tag replacements. */
export function setRoomPowerLevelTagsWithNativeOwner(
  roomId: string,
  content: PowerLevelTagsContent,
  desktopAvailable: boolean,
  invoke: NativeRoomPowerLevelsInvoke
): Promise<NativePowerLevelWriteResult<PowerLevelTagsContent>> {
  return writePowerLevelState(
    roomId,
    content,
    'in.synara.room.power_level_tags',
    'matrix_room_set_power_level_tags',
    desktopAvailable,
    invoke,
    isPowerLevelTagsContent
  );
}

export const defaultNativeRoomPowerLevelsInvoke: NativeRoomPowerLevelsInvoke = (command, args) =>
  invokeDesktopWithAvailability(command, args);
