import type { DesktopInvokeResult } from '../utils/desktop';

export type NativeRoomCreateRequest = {
  name?: string;
  topic?: string;
  roomVersion?: string;
  roomAliasName?: string;
  isDirect?: boolean;
  invite?: string[];
  visibility?: 'private' | 'public';
  preset?: 'private_chat' | 'public_chat' | 'trusted_private_chat';
  creationContent?: {
    type?: string;
    federate?: boolean;
    additionalCreators?: string[];
  };
  encryption?: boolean;
  joinRule?: 'invite' | 'knock' | 'restricted' | 'knock_restricted' | 'public';
  knock?: boolean;
  parentRoomId?: string;
  powerLevelContentOverride?: {
    eventsDefault?: number;
    events?: Record<string, number>;
  };
};

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
};

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

const unavailableMessage = 'Native Matrix room create is unavailable.';

async function requireLoggedIn(invoke: NativeInvoke): Promise<void> {
  const session = await invoke('matrix_session_snapshot');
  if (!session.available) {
    throw new Error(unavailableMessage);
  }
  const snapshot = session.value as NativeSessionSnapshot | undefined;
  if (snapshot?.status !== 'logged_in') {
    throw new Error(unavailableMessage);
  }
}

/**
 * Sole room/space create owner for the desktop product. All unavailable
 * states fail closed; there is no JS SDK create-room fallback.
 */
export async function createRoomWithNativeOwner(
  request: NativeRoomCreateRequest,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<string> {
  if (!desktopAvailable) {
    throw new Error(unavailableMessage);
  }

  await requireLoggedIn(invoke);
  const result = await invoke('matrix_room_create', { request });
  if (!result.available || typeof result.value !== 'string' || !result.value.trim()) {
    throw new Error(unavailableMessage);
  }
  return result.value;
}
