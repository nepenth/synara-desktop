import { invokeDesktopWithAvailability, type DesktopInvokeResult } from '../utils/desktop';

export type NativeRoomCreatorsSnapshot = {
  status: 'ok';
  roomId: string;
  eventType: 'm.room.create';
  stateKey: '';
  sessionGeneration: number;
  creators: string[];
};

export type NativeRoomCreatorsInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
  sessionGeneration?: number;
};

const unavailableMessage = 'Native Matrix room creators are unavailable.';

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

const isSafeGeneration = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;

const isRoomId = (value: unknown): value is string =>
  typeof value === 'string' && /^![^:\s]+:[^\s]+$/.test(value);

const isUserId = (value: unknown): value is string =>
  typeof value === 'string' && /^@[^:\s]+:[^\s]+$/.test(value);

const invokeSafely = async (
  command: string,
  args: Record<string, unknown> | undefined,
  invoke: NativeRoomCreatorsInvoke
): Promise<DesktopInvokeResult<unknown>> => {
  try {
    return await invoke(command, args);
  } catch {
    throw new Error(unavailableMessage);
  }
};

const requireLoggedIn = async (invoke: NativeRoomCreatorsInvoke): Promise<number> => {
  const result = await invokeSafely('matrix_session_snapshot', undefined, invoke);
  if (!result.available || !isRecord(result.value)) throw new Error(unavailableMessage);
  const snapshot = result.value as NativeSessionSnapshot;
  if (snapshot.status !== 'logged_in' || !isSafeGeneration(snapshot.sessionGeneration)) {
    throw new Error(unavailableMessage);
  }
  return snapshot.sessionGeneration;
};

export async function readRoomCreatorsWithNativeOwner(
  roomId: string,
  nativeSession: boolean,
  invoke: NativeRoomCreatorsInvoke = defaultNativeRoomCreatorsInvoke
): Promise<NativeRoomCreatorsSnapshot | undefined> {
  if (!nativeSession) return undefined;
  if (!isRoomId(roomId)) throw new Error(unavailableMessage);

  const sessionGeneration = await requireLoggedIn(invoke);
  const result = await invokeSafely('matrix_room_creators_snapshot', { roomId }, invoke);
  if (!result.available || !isRecord(result.value)) throw new Error(unavailableMessage);

  const value = result.value;
  if (
    value.status !== 'ok' ||
    value.roomId !== roomId ||
    value.eventType !== 'm.room.create' ||
    value.stateKey !== '' ||
    value.sessionGeneration !== sessionGeneration ||
    !isSafeGeneration(value.sessionGeneration) ||
    !Array.isArray(value.creators) ||
    !value.creators.every(isUserId)
  ) {
    throw new Error(unavailableMessage);
  }

  return value as NativeRoomCreatorsSnapshot;
}

export const defaultNativeRoomCreatorsInvoke: NativeRoomCreatorsInvoke = (command, args) =>
  invokeDesktopWithAvailability(command, args);
