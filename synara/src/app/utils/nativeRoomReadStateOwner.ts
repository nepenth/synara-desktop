import type { DesktopInvokeResult } from './desktop';

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
};

export type NativeRoomReadAction = 'mark_read' | 'mark_unread';

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>,
) => Promise<DesktopInvokeResult<unknown>>;

async function requireLoggedIn(invoke: NativeInvoke): Promise<void> {
  const session = await invoke('matrix_session_snapshot');
  if (!session.available) {
    throw new Error('Native Matrix room read state is unavailable.');
  }
  const snapshot = session.value as NativeSessionSnapshot | undefined;
  if (snapshot?.status !== 'logged_in') {
    throw new Error('Native Matrix room read state is unavailable.');
  }
}

/**
 * Sole desktop owner for room-level receipts and the unread flag. Context-menu
 * Mark as Read uses this so a room does not have to be open.
 */
export async function setRoomReadStateWithNativeOwner(
  roomId: string,
  action: NativeRoomReadAction,
  desktopAvailable: boolean,
  invoke: NativeInvoke,
): Promise<void> {
  if (!desktopAvailable) {
    throw new Error('Native Matrix room read state is unavailable.');
  }
  if (!roomId.trim() || (action !== 'mark_read' && action !== 'mark_unread')) {
    throw new Error('Native Matrix room read state is unavailable.');
  }

  await requireLoggedIn(invoke);
  const result = await invoke('matrix_room_set_read_state', { roomId, action });
  if (!result.available) {
    throw new Error('Native Matrix room read state is unavailable.');
  }
}
