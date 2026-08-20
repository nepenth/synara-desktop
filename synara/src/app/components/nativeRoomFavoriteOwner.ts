import type { DesktopInvokeResult } from '../utils/desktop';

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
    throw new Error('Native Matrix room favorite is unavailable.');
  }
  const snapshot = session.value as NativeSessionSnapshot | undefined;
  if (snapshot?.status !== 'logged_in') {
    throw new Error('Native Matrix room favorite is unavailable.');
  }
}

/**
 * Sole `m.favourite` write owner for the desktop product. A native session
 * persists the Matrix tag; there is no localStorage fallback.
 */
export async function setRoomFavoriteWithNativeOwner(
  roomId: string,
  favorite: boolean,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<void> {
  if (!desktopAvailable) {
    throw new Error('Native Matrix room favorite is unavailable.');
  }
  if (!roomId.trim()) {
    throw new Error('Native Matrix room favorite is unavailable.');
  }

  await requireLoggedIn(invoke);
  const result = await invoke('matrix_room_set_favorite', { roomId, favorite });
  if (!result.available) {
    throw new Error('Native Matrix room favorite is unavailable.');
  }
}
