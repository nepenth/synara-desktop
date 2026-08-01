import type { DesktopInvokeResult } from '../../../utils/desktop';

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
};

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

export type NativeRoomProfileWriteResult = {
  status: 'ok';
};

/**
 * R-ROOM-PROFILE: sole room profile write owner when a native Matrix session
 * is live. Fail-closed — never falls through to mx.sendStateEvent for
 * m.room.name / m.room.topic / m.room.avatar.
 */
export async function isNativeRoomProfileWriteSession(
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<boolean> {
  if (!desktopAvailable) return false;
  const session = await invoke('matrix_session_snapshot');
  if (!session.available) return false;
  const snapshot = session.value as NativeSessionSnapshot | undefined;
  return snapshot?.status === 'logged_in';
}

async function assertOk(result: DesktopInvokeResult<unknown>, label: string): Promise<void> {
  if (!result.available) {
    throw new Error(`Native Matrix room ${label} is unavailable.`);
  }
  const body = result.value as NativeRoomProfileWriteResult | undefined;
  if (body?.status !== 'ok') {
    throw new Error(`Native Matrix room ${label} is unavailable.`);
  }
}

export async function setRoomNameWithNativeOwner(
  roomId: string,
  name: string,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<'native' | 'legacy'> {
  if (!(await isNativeRoomProfileWriteSession(desktopAvailable, invoke))) {
    return 'legacy';
  }
  const result = await invoke('matrix_set_room_name', { roomId, name });
  await assertOk(result, 'name update');
  return 'native';
}

export async function setRoomTopicWithNativeOwner(
  roomId: string,
  topic: string,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<'native' | 'legacy'> {
  if (!(await isNativeRoomProfileWriteSession(desktopAvailable, invoke))) {
    return 'legacy';
  }
  const result = await invoke('matrix_set_room_topic', { roomId, topic });
  await assertOk(result, 'topic update');
  return 'native';
}

export async function setRoomAvatarWithNativeOwner(
  roomId: string,
  mxc: string,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<'native' | 'legacy'> {
  if (!(await isNativeRoomProfileWriteSession(desktopAvailable, invoke))) {
    return 'legacy';
  }
  const result = await invoke('matrix_set_room_avatar', { roomId, mxc });
  await assertOk(result, 'avatar update');
  return 'native';
}
