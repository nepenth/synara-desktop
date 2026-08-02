/**
 * V-ROOMS.R-DIRECTORY — sole desktop public-room directory owner.
 *
 * Native availability, session generation, request correlation, cancellation,
 * and DTO parsing are terminal boundaries. There is no JS Matrix fallback.
 */

import {
  parseDirectoryProtocols,
  parseDirectorySearchResponse,
  type DirectoryPage,
  type DirectoryProtocols,
  type DirectoryRoomType,
} from '../../../features/matrix-dto/roomDirectory';
import { hasForbiddenWireFields } from '../../../features/matrix-dto/parseUtil';
import { invokeDesktopWithAvailability, type DesktopInvokeResult } from '../../../utils/desktop';

export type NativeRoomDirectoryInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

export type NativeRoomDirectorySearchRequest = {
  serverName: string;
  term?: string;
  roomType?: DirectoryRoomType;
  thirdPartyInstanceId?: string;
  limit: number;
  since?: string;
};

export type NativeRoomDirectorySession = {
  sessionGeneration: number;
  userId: string;
  serverName: string;
};

type NativeSessionWire = {
  status: 'logged_out' | 'logged_in';
  sessionGeneration?: unknown;
  user_id?: unknown;
  device_id?: unknown;
  homeserver_url?: unknown;
};

const SESSION_WIRE_KEYS = [
  'status',
  'sessionGeneration',
  'user_id',
  'device_id',
  'homeserver_url',
] as const;

const hasExactSessionKeys = (value: Record<string, unknown>): boolean => {
  const allowed = new Set<string>(SESSION_WIRE_KEYS);
  if (!Object.keys(value).every((key) => allowed.has(key))) return false;
  if (value.status === 'logged_out') return Object.keys(value).length === 1;
  return SESSION_WIRE_KEYS.every((key) => key in value);
};

const unavailableMessage = 'Native Matrix room directory is unavailable.';
const defaultInvoke: NativeRoomDirectoryInvoke = (command, args) =>
  invokeDesktopWithAvailability(command, args);

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

const isSafeGeneration = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value > 0;

const serverFromUserId = (userId: string): string | undefined => {
  const separator = userId.indexOf(':');
  const server = separator >= 0 ? userId.slice(separator + 1).trim() : '';
  return server || undefined;
};

const unavailable = (): Error => new Error(unavailableMessage);

const invokeNative = async (
  command: string,
  args: Record<string, unknown>,
  invoke: NativeRoomDirectoryInvoke
): Promise<unknown> => {
  try {
    const result = await invoke(command, args);
    if (!result.available || result.value === undefined) throw unavailable();
    return result.value;
  } catch {
    throw unavailable();
  }
};

const parseLoggedInSession = (value: unknown): NativeRoomDirectorySession | undefined => {
  if (!isRecord(value) || hasForbiddenWireFields(value) || !hasExactSessionKeys(value))
    return undefined;
  const wire = value as NativeSessionWire;
  if (wire.status !== 'logged_in' || !isSafeGeneration(wire.sessionGeneration)) {
    return undefined;
  }
  const userId = wire.user_id;
  if (
    typeof userId !== 'string' ||
    !userId.trim() ||
    typeof wire.device_id !== 'string' ||
    !wire.device_id.trim() ||
    typeof wire.homeserver_url !== 'string' ||
    !wire.homeserver_url.trim()
  ) {
    return undefined;
  }
  const serverName = serverFromUserId(userId);
  if (!serverName) return undefined;
  return {
    sessionGeneration: wire.sessionGeneration,
    userId,
    serverName,
  };
};

export async function readNativeRoomDirectorySession(
  desktopAvailable: boolean,
  invoke: NativeRoomDirectoryInvoke = defaultInvoke
): Promise<NativeRoomDirectorySession | undefined> {
  if (!desktopAvailable) throw unavailable();
  let result: DesktopInvokeResult<unknown>;
  try {
    result = await invoke('matrix_session_snapshot');
  } catch {
    throw unavailable();
  }
  if (!result.available || result.value === undefined) throw unavailable();
  const value = result.value;
  if (!isRecord(value) || hasForbiddenWireFields(value) || !hasExactSessionKeys(value)) {
    throw unavailable();
  }
  if (value.status === 'logged_out') return undefined;
  const session = parseLoggedInSession(value);
  if (!session) throw unavailable();
  return session;
}

const validateSearchInput = (request: NativeRoomDirectorySearchRequest): void => {
  if (
    typeof request.serverName !== 'string' ||
    request.serverName.trim() !== request.serverName ||
    request.serverName.length === 0 ||
    request.serverName.length > 256 ||
    !Number.isSafeInteger(request.limit) ||
    request.limit < 1 ||
    request.limit > 100
  ) {
    throw unavailable();
  }
  for (const value of [request.term, request.thirdPartyInstanceId]) {
    if (
      value !== undefined &&
      (value.trim() !== value || value.length === 0 || value.length > 256)
    ) {
      throw unavailable();
    }
  }
  if (
    request.since !== undefined &&
    (request.since.trim() !== request.since ||
      request.since.length === 0 ||
      request.since.length > 512)
  ) {
    throw unavailable();
  }
  if (
    request.roomType !== undefined &&
    request.roomType !== 'room' &&
    request.roomType !== 'space'
  ) {
    throw unavailable();
  }
};

export type NativeRoomDirectoryOwner = {
  getProtocols: () => Promise<DirectoryProtocols>;
  search: (request: NativeRoomDirectorySearchRequest) => Promise<DirectoryPage>;
  cancel: () => Promise<void>;
  dispose: () => Promise<void>;
};

export function createNativeRoomDirectoryOwner(
  desktopAvailable: boolean,
  invoke: NativeRoomDirectoryInvoke = defaultInvoke
): NativeRoomDirectoryOwner {
  let nextRequestId = 0;
  let disposed = false;
  let active: { requestId: number; sessionGeneration?: number } | undefined;

  const cancelRequest = async (request: { requestId: number; sessionGeneration?: number }) => {
    if (!request.sessionGeneration) return;
    try {
      await invokeNative(
        'matrix_room_directory_cancel',
        {
          sessionGeneration: request.sessionGeneration,
          requestId: request.requestId,
        },
        invoke
      );
    } catch {
      // The owner is already terminal for the obsolete request. A missing
      // cancel command must not resurrect or render its eventual response.
    }
  };

  const cancel = async (): Promise<void> => {
    const request = active;
    active = undefined;
    if (request) await cancelRequest(request);
  };

  const search = async (request: NativeRoomDirectorySearchRequest): Promise<DirectoryPage> => {
    if (disposed) throw unavailable();
    validateSearchInput(request);
    const requestId = ++nextRequestId;
    const previous = active;
    active = { requestId };
    if (previous) void cancelRequest(previous);

    const session = await readNativeRoomDirectorySession(desktopAvailable, invoke);
    if (!session || active?.requestId !== requestId || disposed) throw unavailable();
    active.sessionGeneration = session.sessionGeneration;

    const responseValue = await invokeNative(
      'matrix_room_directory_search',
      {
        sessionGeneration: session.sessionGeneration,
        requestId,
        serverName: request.serverName,
        term: request.term,
        roomType: request.roomType,
        thirdPartyInstanceId: request.thirdPartyInstanceId,
        limit: request.limit,
        since: request.since,
      },
      invoke
    );
    const response = parseDirectorySearchResponse(responseValue);
    if (
      !response ||
      response.sessionGeneration !== session.sessionGeneration ||
      response.requestId !== requestId ||
      response.status !== 'ready' ||
      !response.page ||
      response.page.sessionGeneration !== session.sessionGeneration ||
      response.page.requestId !== requestId ||
      active?.requestId !== requestId ||
      disposed
    ) {
      throw unavailable();
    }
    active = undefined;
    return response.page;
  };

  const getProtocols = async (): Promise<DirectoryProtocols> => {
    if (disposed || !desktopAvailable) throw unavailable();
    const session = await readNativeRoomDirectorySession(desktopAvailable, invoke);
    if (!session || disposed) throw unavailable();
    const value = await invokeNative('matrix_room_directory_protocols', {}, invoke);
    const protocols = parseDirectoryProtocols(value);
    if (!protocols || protocols.sessionGeneration !== session.sessionGeneration)
      throw unavailable();
    return protocols;
  };

  const dispose = async (): Promise<void> => {
    if (disposed) return;
    disposed = true;
    await cancel();
  };

  return { getProtocols, search, cancel, dispose };
}
