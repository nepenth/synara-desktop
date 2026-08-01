import type { DesktopInvokeResult } from '../utils/desktop';

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
};

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

export type NativeUploadMediaResult = {
  mxc: string;
};

/**
 * V-SEND.R-PACK-UPLOAD (+ shared compact media upload): sole media-upload owner
 * when a native Matrix session is live. Reuses `matrix_upload_media` from
 * V-SEND.R-AVATAR-UPLOAD. Fail-closed — never falls through to
 * mx.uploadContent on a native session.
 */
export async function isNativeMediaUploadSession(
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<boolean> {
  if (!desktopAvailable) return false;
  const session = await invoke('matrix_session_snapshot');
  if (!session.available) return false;
  const snapshot = session.value as NativeSessionSnapshot | undefined;
  return snapshot?.status === 'logged_in';
}

export async function uploadMediaWithNativeOwner(
  mimeType: string,
  bytes: number[],
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<{ mxc: string } | 'legacy'> {
  if (!(await isNativeMediaUploadSession(desktopAvailable, invoke))) {
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
