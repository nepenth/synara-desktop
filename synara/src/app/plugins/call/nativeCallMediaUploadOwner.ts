import {
  isNativeMediaUploadSession,
  uploadMediaBytesWithNativeOwner,
  type NativeInvoke,
} from '../../state/nativeMediaUploadOwner';

type LegacyUploadResult = {
  content_uri: string;
};

type NativeUploadBytes = {
  mimeType: string;
  bytes: number[];
};

const getNativeUploadBytes = async (
  file: XMLHttpRequestBodyInit
): Promise<NativeUploadBytes | undefined> => {
  if (typeof Blob !== 'undefined' && file instanceof Blob) {
    const buffer = await file.arrayBuffer();
    return {
      mimeType: file.type || 'application/octet-stream',
      bytes: Array.from(new Uint8Array(buffer)),
    };
  }

  if (file instanceof ArrayBuffer) {
    return {
      mimeType: 'application/octet-stream',
      bytes: Array.from(new Uint8Array(file)),
    };
  }

  if (ArrayBuffer.isView(file)) {
    return {
      mimeType: 'application/octet-stream',
      bytes: Array.from(new Uint8Array(file.buffer, file.byteOffset, file.byteLength)),
    };
  }

  return undefined;
};

/**
 * V-SEND.R-CALL-UPLOAD: native media is the sole owner for a logged-in
 * desktop Matrix session. The legacy callback is only reachable for web or
 * logged-out sessions; native command failures stay terminal.
 */
export async function uploadCallWidgetFileWithNativeOwner(
  file: XMLHttpRequestBodyInit,
  desktopAvailable: boolean,
  invoke: NativeInvoke,
  legacyUpload: () => Promise<LegacyUploadResult>
): Promise<{ contentUri: string }> {
  if (!(await isNativeMediaUploadSession(desktopAvailable, invoke))) {
    const uploadResult = await legacyUpload();
    return { contentUri: uploadResult.content_uri };
  }

  const nativeUpload = await getNativeUploadBytes(file);
  if (!nativeUpload) {
    throw new Error('Native Matrix call media upload is unavailable.');
  }

  const uploadResult = await uploadMediaBytesWithNativeOwner(
    nativeUpload.mimeType,
    nativeUpload.bytes,
    invoke
  );

  return { contentUri: uploadResult.mxc };
}
