import type { DesktopInvokeResult } from '../utils/desktop';

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
};

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

const unavailableMessage = 'Native Matrix room moderation is unavailable.';

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

async function runModerationCommand(
  command: string,
  args: Record<string, unknown>,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<void> {
  if (!desktopAvailable) {
    throw new Error(unavailableMessage);
  }

  await requireLoggedIn(invoke);
  const result = await invoke(command, args);
  if (!result.available) {
    throw new Error(unavailableMessage);
  }
}

/** Sole desktop owner for room invite mutations. There is no JS SDK fallback. */
export async function inviteUserWithNativeOwner(
  roomId: string,
  userId: string,
  reason: string | undefined,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<void> {
  if (!roomId.trim() || !userId.trim()) {
    throw new Error(unavailableMessage);
  }
  const args: Record<string, unknown> = { roomId, userId };
  if (reason !== undefined) {
    args.reason = reason;
  }
  await runModerationCommand('matrix_room_invite', args, desktopAvailable, invoke);
}

/** Sole desktop owner for room kick mutations. There is no JS SDK fallback. */
export async function kickUserWithNativeOwner(
  roomId: string,
  userId: string,
  reason: string | undefined,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<void> {
  if (!roomId.trim() || !userId.trim()) {
    throw new Error(unavailableMessage);
  }
  const args: Record<string, unknown> = { roomId, userId };
  if (reason !== undefined) {
    args.reason = reason;
  }
  await runModerationCommand('matrix_room_kick', args, desktopAvailable, invoke);
}

/** Sole desktop owner for room ban mutations. There is no JS SDK fallback. */
export async function banUserWithNativeOwner(
  roomId: string,
  userId: string,
  reason: string | undefined,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<void> {
  if (!roomId.trim() || !userId.trim()) {
    throw new Error(unavailableMessage);
  }
  const args: Record<string, unknown> = { roomId, userId };
  if (reason !== undefined) {
    args.reason = reason;
  }
  await runModerationCommand('matrix_room_ban', args, desktopAvailable, invoke);
}

/** Sole desktop owner for room unban mutations. There is no JS SDK fallback. */
export async function unbanUserWithNativeOwner(
  roomId: string,
  userId: string,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<void> {
  if (!roomId.trim() || !userId.trim()) {
    throw new Error(unavailableMessage);
  }
  await runModerationCommand('matrix_room_unban', { roomId, userId }, desktopAvailable, invoke);
}

/** Sole desktop owner for per-user power-level mutations. There is no JS SDK fallback. */
export async function setPowerLevelWithNativeOwner(
  roomId: string,
  userId: string,
  powerLevel: number,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<void> {
  if (!roomId.trim() || !userId.trim() || !Number.isSafeInteger(powerLevel)) {
    throw new Error(unavailableMessage);
  }
  await runModerationCommand(
    'matrix_room_set_power_level',
    { roomId, userId, powerLevel },
    desktopAvailable,
    invoke
  );
}
