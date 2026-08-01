import type { DesktopInvokeResult } from '../../utils/desktop';

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
};

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

async function requireLoggedIn(invoke: NativeInvoke): Promise<void> {
  const session = await invoke('matrix_session_snapshot');
  if (!session.available) {
    throw new Error('Native Matrix room leave is unavailable.');
  }
  const snapshot = session.value as NativeSessionSnapshot | undefined;
  if (snapshot?.status !== 'logged_in') {
    throw new Error('Native Matrix room leave is unavailable.');
  }
}

/**
 * Sole room/space leave owner for the desktop product. A native session owns
 * the mutation and all unavailable states fail closed; there is no JS SDK
 * leave fallback.
 */
export async function leaveRoomWithNativeOwner(
  roomId: string,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<void> {
  if (!desktopAvailable) {
    throw new Error('Native Matrix room leave is unavailable.');
  }
  if (!roomId.trim()) {
    throw new Error('Native Matrix room leave is unavailable.');
  }

  await requireLoggedIn(invoke);
  const result = await invoke('matrix_room_leave', { roomId });
  if (!result.available) {
    throw new Error('Native Matrix room leave is unavailable.');
  }
}
