import type { EncryptedAttachmentInfo } from '../../types/matrix/common';
import { invokeDesktopWithAvailability } from '../utils/desktop';
import { mxcUrlToHttp } from '../utils/matrix';

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

async function downloadNativeMedia(contentUri: string, mimeType: string): Promise<Blob> {
  const resolved = timelineMediaHandleFromUri(contentUri) ?? contentUri.trim();
  const result = await invokeDesktopWithAvailability<{ bytes?: unknown }>('matrix_media_download', {
    contentUri: resolved,
  });
  if (!result.available || !result.value || !Array.isArray(result.value.bytes)) {
    throw new Error('Native media download failed');
  }
  const bytes = Uint8Array.from(result.value.bytes.map((b) => (typeof b === 'number' ? b : 0)));
  return new Blob([bytes], { type: mimeType });
}

export async function downloadMatrixMedia(
  mx: MatrixMediaClient,
  mxcUrl: string,
  options: MatrixMediaDownloadOptions
): Promise<Blob> {
  void mx;
  const handle = timelineMediaHandleFromUri(mxcUrl);
  if (options.encryptedInfo && !handle) {
    throw new Error('Leftover encrypted media requires a native handle');
  }
  const trimmed = mxcUrl.trim();
  if (handle || trimmed.startsWith('mxc://')) {
    return downloadNativeMedia(mxcUrl, options.mimeType);
  }
  throw new Error('Invalid Matrix media URL');
}

export async function createMatrixMediaObjectUrl(
  mx: MatrixMediaClient,
  mxcUrl: string,
  options: MatrixMediaDownloadOptions
): Promise<string> {
  const fileContent = await downloadMatrixMedia(mx, mxcUrl, options);
  return URL.createObjectURL(fileContent);
}
