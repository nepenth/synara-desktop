import type { DesktopInvokeResult } from '../../utils/desktop';

const MAX_CALL_WIDGET_MEDIA_URI_BYTES = 2 * 1024;
const MAX_CALL_WIDGET_MEDIA_DOWNLOAD_BYTES = 32 * 1024 * 1024;

export type NativeCallWidgetMediaInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

export type NativeCallWidgetMediaConfig = {
  'm.upload.size': number;
};

export type NativeCallWidgetMediaDownload = {
  file: Uint8Array;
};

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
};

const unavailable = (operation: string): Error =>
  new Error(`Native Matrix call widget ${operation} is unavailable.`);

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

const isLoggedInNativeSession = async (
  desktopAvailable: boolean,
  invoke: NativeCallWidgetMediaInvoke
): Promise<void> => {
  if (!desktopAvailable) throw unavailable('media');

  let result: DesktopInvokeResult<unknown>;
  try {
    result = await invoke('matrix_session_snapshot');
  } catch {
    throw unavailable('session');
  }

  if (!result.available) throw unavailable('session');
  const snapshot = result.value as NativeSessionSnapshot | undefined;
  if (snapshot?.status !== 'logged_in') throw unavailable('session');
};

const isValidContentUri = (contentUri: unknown): contentUri is string => {
  if (
    typeof contentUri !== 'string' ||
    contentUri.length === 0 ||
    contentUri.length > MAX_CALL_WIDGET_MEDIA_URI_BYTES ||
    contentUri.trim() !== contentUri ||
    !contentUri.startsWith('mxc://') ||
    contentUri.includes('?') ||
    contentUri.includes('#')
  ) {
    return false;
  }

  const separator = contentUri.indexOf('/', 'mxc://'.length);
  if (separator <= 'mxc://'.length || contentUri.indexOf('/', separator + 1) !== -1) {
    return false;
  }

  const serverName = contentUri.slice('mxc://'.length, separator);
  const mediaId = contentUri.slice(separator + 1);
  if (!/^[A-Za-z0-9_-]+$/.test(mediaId)) return false;
  if (!/^(?:[A-Za-z0-9.-]+(?::[0-9]{1,5})?|\[[0-9A-Fa-f:]+\](?::[0-9]{1,5})?)$/.test(serverName)) {
    return false;
  }

  try {
    const parsed = new URL(contentUri);
    return (
      parsed.protocol === 'mxc:' &&
      parsed.username === '' &&
      parsed.password === '' &&
      parsed.search === '' &&
      parsed.hash === '' &&
      parsed.pathname === `/${mediaId}`
    );
  } catch {
    return false;
  }
};

const invokeNative = async (
  operation: string,
  invoke: NativeCallWidgetMediaInvoke,
  command: string,
  args?: Record<string, unknown>
): Promise<unknown> => {
  let result: DesktopInvokeResult<unknown>;
  try {
    result = await invoke(command, args);
  } catch {
    throw unavailable(operation);
  }
  if (!result.available) throw unavailable(operation);
  return result.value;
};

export async function getMediaConfigWithNativeOwner(
  desktopAvailable: boolean,
  invoke: NativeCallWidgetMediaInvoke
): Promise<NativeCallWidgetMediaConfig> {
  await isLoggedInNativeSession(desktopAvailable, invoke);
  const value = await invokeNative('media config', invoke, 'matrix_call_media_config');
  if (!isRecord(value) || Object.keys(value).length !== 1) {
    throw unavailable('media config');
  }

  const uploadSize = value['m.upload.size'];
  if (
    typeof uploadSize !== 'number' ||
    !Number.isFinite(uploadSize) ||
    !Number.isSafeInteger(uploadSize) ||
    uploadSize < 0
  ) {
    throw unavailable('media config');
  }
  return { 'm.upload.size': uploadSize };
}

export async function downloadFileWithNativeOwner(
  contentUri: string,
  desktopAvailable: boolean,
  invoke: NativeCallWidgetMediaInvoke
): Promise<NativeCallWidgetMediaDownload> {
  if (!isValidContentUri(contentUri)) throw unavailable('media download');
  await isLoggedInNativeSession(desktopAvailable, invoke);
  const value = await invokeNative('media download', invoke, 'matrix_media_download', {
    contentUri,
  });
  if (!isRecord(value) || Object.keys(value).length !== 1 || !Array.isArray(value.bytes)) {
    throw unavailable('media download');
  }

  const bytes = value.bytes;
  if (
    bytes.length > MAX_CALL_WIDGET_MEDIA_DOWNLOAD_BYTES ||
    !bytes.every((byte) => Number.isInteger(byte) && byte >= 0 && byte <= 255)
  ) {
    throw unavailable('media download');
  }
  return { file: Uint8Array.from(bytes) };
}

export { isValidContentUri as isValidCallWidgetMediaContentUri };
