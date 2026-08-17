import type { EncryptedAttachmentInfo } from 'browser-encrypt-attachment';
import { invokeDesktopWithAvailability } from '../utils/desktop';
import { decryptFile, downloadEncryptedMedia, downloadMedia, mxcUrlToHttp } from '../utils/matrix';

const TIMELINE_MEDIA_HANDLE_PREFIX = 'timeline-media-';

/** Prefer an opaque timeline handle over leftover `mxc://` or protocol URLs. */
function timelineMediaHandleFromUri(contentUri: string): string | null {
  const trimmed = contentUri.trim();
  if (trimmed.startsWith(TIMELINE_MEDIA_HANDLE_PREFIX)) {
    return trimmed;
  }
  const match = /^synara-media:\/\/[^/]*\/(.+)$/i.exec(trimmed);
  if (!match?.[1]) {
    return null;
  }
  try {
    return decodeURIComponent(match[1]);
  } catch {
    return match[1];
  }
}

type MatrixMediaClient = Parameters<typeof mxcUrlToHttp>[0];

export type MatrixMediaUrlOptions = {
  useAuthentication?: boolean;
  width?: number;
  height?: number;
  resizeMethod?: string;
  allowDirectLinks?: boolean;
  allowRedirects?: boolean;
};

export type MatrixMediaDownloadOptions = MatrixMediaUrlOptions & {
  mimeType: string;
  encryptedInfo?: EncryptedAttachmentInfo;
};

export function resolveMatrixMediaUrl(
  mx: MatrixMediaClient,
  mxcUrl: string,
  options: MatrixMediaUrlOptions = {}
): string {
  const mediaUrl = mxcUrlToHttp(
    mx,
    mxcUrl,
    options.useAuthentication,
    options.width,
    options.height,
    options.resizeMethod,
    options.allowDirectLinks,
    options.allowRedirects
  );
  if (!mediaUrl) throw new Error('Invalid Matrix media URL');
  return mediaUrl;
}

export function resolveOptionalMatrixMediaUrl(
  mx: MatrixMediaClient,
  mxcUrl: string | undefined,
  options: MatrixMediaUrlOptions = {}
): string | undefined {
  if (!mxcUrl) return undefined;

  try {
    return resolveMatrixMediaUrl(mx, mxcUrl, options);
  } catch {
    return undefined;
  }
}

export function resolveMatrixThumbnailUrl(
  mx: MatrixMediaClient,
  mxcUrl: string,
  size: number,
  options: Omit<MatrixMediaUrlOptions, 'width' | 'height' | 'resizeMethod'> = {}
): string | undefined {
  try {
    return resolveMatrixMediaUrl(mx, mxcUrl, {
      ...options,
      width: size,
      height: size,
      resizeMethod: 'crop',
    });
  } catch {
    return undefined;
  }
}

async function downloadNativeTimelineMedia(contentUri: string, mimeType: string): Promise<Blob> {
  const handle = timelineMediaHandleFromUri(contentUri);
  if (!handle) {
    throw new Error('Invalid native timeline media handle');
  }
  const result = await invokeDesktopWithAvailability<{ bytes?: unknown }>('matrix_media_download', {
    contentUri: handle,
  });
  if (!result.available || !result.value || !Array.isArray(result.value.bytes)) {
    throw new Error('Native timeline media download failed');
  }
  const bytes = Uint8Array.from(result.value.bytes.map((b) => (typeof b === 'number' ? b : 0)));
  return new Blob([bytes], { type: mimeType });
}

export async function downloadMatrixMedia(
  mx: MatrixMediaClient,
  mxcUrl: string,
  options: MatrixMediaDownloadOptions
): Promise<Blob> {
  // Live timeline media is a native handle. Do not JS-decrypt or fetch mxc.
  if (timelineMediaHandleFromUri(mxcUrl)) {
    return downloadNativeTimelineMedia(mxcUrl, options.mimeType);
  }
  const mediaUrl = resolveMatrixMediaUrl(mx, mxcUrl, options);
  if (options.encryptedInfo) {
    const encryptedInfo = options.encryptedInfo;
    return downloadEncryptedMedia(mediaUrl, (encryptedBuffer) =>
      decryptFile(encryptedBuffer, options.mimeType, encryptedInfo)
    );
  }
  return downloadMedia(mediaUrl);
}

export async function createMatrixMediaObjectUrl(
  mx: MatrixMediaClient,
  mxcUrl: string,
  options: MatrixMediaDownloadOptions
): Promise<string> {
  const fileContent = await downloadMatrixMedia(mx, mxcUrl, options);
  return URL.createObjectURL(fileContent);
}
