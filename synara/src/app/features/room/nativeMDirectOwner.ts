import type { DesktopInvokeResult } from '../../utils/desktop';

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
};

export type NativeMDirectMutationResult = {
  roomId: string;
  status: 'updated';
};

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

async function requireLoggedIn(invoke: NativeInvoke): Promise<void> {
  const session = await invoke('matrix_session_snapshot');
  if (!session.available) {
    throw new Error('Native Matrix direct-room map is unavailable.');
  }
  const snapshot = session.value as NativeSessionSnapshot | undefined;
  if (snapshot?.status !== 'logged_in') {
    throw new Error('Native Matrix direct-room map is unavailable.');
  }
}

export async function addRoomToMDirectWithNativeOwner(
  roomId: string,
  userId: string,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<void> {
  if (!desktopAvailable) {
    throw new Error('Native Matrix direct-room map is unavailable.');
  }
  await requireLoggedIn(invoke);
  const result = await invoke('matrix_mdirect_add', { roomId, userId });
  if (!result.available) {
    throw new Error('Native Matrix direct-room map is unavailable.');
  }
  const value = result.value as NativeMDirectMutationResult | undefined;
  if (value?.status !== 'updated') {
    throw new Error('Native Matrix direct-room map is unavailable.');
  }
}

export async function removeRoomFromMDirectWithNativeOwner(
  roomId: string,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<void> {
  if (!desktopAvailable) {
    throw new Error('Native Matrix direct-room map is unavailable.');
  }
  await requireLoggedIn(invoke);
  const result = await invoke('matrix_mdirect_remove', { roomId });
  if (!result.available) {
    throw new Error('Native Matrix direct-room map is unavailable.');
  }
  const value = result.value as NativeMDirectMutationResult | undefined;
  if (value?.status !== 'updated') {
    throw new Error('Native Matrix direct-room map is unavailable.');
  }
}
