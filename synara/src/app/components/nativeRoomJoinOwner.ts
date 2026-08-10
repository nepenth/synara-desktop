import type { DesktopInvokeResult } from '../utils/desktop';

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
};

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

const unavailableMessage = 'Native Matrix room join is unavailable.';

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
 * Sole room/space join owner for the desktop product. A native session owns
 * the mutation and all unavailable states fail closed; there is no JS SDK
 * join fallback.
 */
export async function joinRoomWithNativeOwner(
  roomIdOrAlias: string,
  viaServers: string[] | undefined,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<void> {
  if (!desktopAvailable || !roomIdOrAlias.trim()) {
    throw new Error(unavailableMessage);
  }

  await requireLoggedIn(invoke);
  const args: Record<string, unknown> = { roomIdOrAlias };
  if (viaServers !== undefined) {
    args.viaServers = viaServers;
  }
  const result = await invoke('matrix_room_join', args);
  if (!result.available) {
    throw new Error(unavailableMessage);
  }
}
