import type { DesktopInvokeResult } from '../../../utils/desktop';

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
};

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

export type NativeProfileWriteResult = {
  status: 'ok';
};

export type NativeUploadMediaResult = {
  mxc: string;
};

export type NativeOwnProfile = {
  userId: string;
  displayName?: string;
  avatarUrl?: string;
};

/**
 * V-SEND.R-AVATAR-UPLOAD: sole user-profile write owner when a native Matrix
 * session is live. Fail-closed — never falls through to mx.setDisplayName /
 * mx.setAvatarUrl / mx.uploadContent.
 */
export async function isNativeProfileWriteSession(
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<boolean> {
  if (!desktopAvailable) return false;
  const session = await invoke('matrix_session_snapshot');
  if (!session.available) return false;
  const snapshot = session.value as NativeSessionSnapshot | undefined;
  return snapshot?.status === 'logged_in';
}

export async function setOwnDisplayNameWithNativeOwner(
  displayName: string,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<'native' | 'legacy'> {
  if (!(await isNativeProfileWriteSession(desktopAvailable, invoke))) {
    return 'legacy';
  }
  const result = await invoke('matrix_set_own_display_name', { displayName });
  if (!result.available) {
    throw new Error('Native Matrix display name update is unavailable.');
  }
  const body = result.value as NativeProfileWriteResult | undefined;
  if (body?.status !== 'ok') {
    throw new Error('Native Matrix display name update is unavailable.');
  }
  return 'native';
}

export async function setOwnAvatarWithNativeOwner(
  mxc: string,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<'native' | 'legacy'> {
  if (!(await isNativeProfileWriteSession(desktopAvailable, invoke))) {
    return 'legacy';
  }
  const result = await invoke('matrix_set_own_avatar', { mxc });
  if (!result.available) {
    throw new Error('Native Matrix avatar update is unavailable.');
  }
  const body = result.value as NativeProfileWriteResult | undefined;
  if (body?.status !== 'ok') {
    throw new Error('Native Matrix avatar update is unavailable.');
  }
  return 'native';
}

const isSafeMxc = (value: string | undefined): value is string =>
  typeof value === 'string' && value.startsWith('mxc://') && value.split('/').length >= 4;

export async function getOwnProfileWithNativeOwner(
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeOwnProfile | 'legacy'> {
  if (!(await isNativeProfileWriteSession(desktopAvailable, invoke))) {
    return 'legacy';
  }
  const result = await invoke('matrix_get_own_profile');
  if (!result.available || !result.value || typeof result.value !== 'object') {
    throw new Error('Native Matrix profile is unavailable.');
  }
  const body = result.value as Record<string, unknown>;
  const userId = typeof body.userId === 'string' ? body.userId : '';
  if (!userId.startsWith('@')) {
    throw new Error('Native Matrix profile is unavailable.');
  }
  const displayName = typeof body.displayName === 'string' ? body.displayName : undefined;
  const avatarUrl = isSafeMxc(typeof body.avatarUrl === 'string' ? body.avatarUrl : undefined)
    ? (body.avatarUrl as string)
    : undefined;
  return { userId, displayName, avatarUrl };
}

export async function uploadMediaWithNativeOwner(
  mimeType: string,
  bytes: number[],
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<{ mxc: string } | 'legacy'> {
  if (!(await isNativeProfileWriteSession(desktopAvailable, invoke))) {
    return 'legacy';
  }
  const result = await invoke('matrix_upload_media', { mimeType, bytes });
  if (!result.available) {
    throw new Error('Native Matrix media upload is unavailable.');
  }
  const body = result.value as NativeUploadMediaResult | undefined;
  if (!body?.mxc) {
    throw new Error('Native Matrix media upload is unavailable.');
  }
  return { mxc: body.mxc };
}
