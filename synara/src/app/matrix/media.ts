import type { EncryptedAttachmentInfo } from '../../types/matrix/common';
import { convertDesktopFileSrc, invokeDesktopWithAvailability } from '../utils/desktop';
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

function resolvedMediaContentUri(contentUri: string): string {
  return timelineMediaHandleFromUri(contentUri) ?? contentUri.trim();
}

function assertDisplayableMedia(
  contentUri: string,
  encryptedInfo?: EncryptedAttachmentInfo
): string {
  const handle = timelineMediaHandleFromUri(contentUri);
  if (encryptedInfo && !handle) {
    throw new Error('Leftover encrypted media requires a native handle');
  }
  const resolved = resolvedMediaContentUri(contentUri);
  if (handle || resolved.startsWith('mxc://')) return resolved;
  throw new Error('Invalid Matrix media URL');
}

/** Opaque display URL. The webview loads bytes from Rust; JS never receives them. */
export function nativeMediaDisplayUrl(contentUri: string | undefined): string | undefined {
  if (!contentUri) return undefined;
  try {
    const resolved = assertDisplayableMedia(contentUri);
    return convertDesktopFileSrc(resolved, 'synara-media');
  } catch {
    return undefined;
  }
}

const mediaDiagnosticId = (error: unknown): string | undefined => {
  if (!error || typeof error !== 'object') return undefined;
  const record = error as Record<string, unknown>;
  const diagnosticId = record.diagnosticId ?? record.diagnostic_id;
  return typeof diagnosticId === 'string' ? diagnosticId : undefined;
};

export async function previewMatrixMediaText(
  contentUri: string
): Promise<{ kind: 'ready'; text: string } | { kind: 'tooLarge' }> {
  const resolved = resolvedMediaContentUri(contentUri);
  if (!resolved) throw new Error('Could not open file.');
  try {
    const result = await invokeDesktopWithAvailability<{ text?: unknown }>(
      'matrix_media_text_preview',
      { contentUri: resolved }
    );
    if (!result.available || typeof result.value?.text !== 'string') {
      throw new Error('Could not open file.');
    }
    return { kind: 'ready', text: result.value.text };
  } catch (error) {
    if (mediaDiagnosticId(error) === 'v-send.r-media-preview-too-large') {
      return { kind: 'tooLarge' };
    }
    throw error instanceof Error ? error : new Error('Could not open file.');
  }
}

/** Rust writes the file and returns its Downloads filename. No file bytes cross IPC. */
export async function saveMatrixMediaFile(contentUri: string, filename: string): Promise<string> {
  const resolved = resolvedMediaContentUri(contentUri);
  if (!resolved) throw new Error('File attachment is unavailable.');
  const result = await invokeDesktopWithAvailability<{ filename?: unknown }>('matrix_media_save', {
    contentUri: resolved,
    filename,
  });
  if (
    !result.available ||
    !result.value ||
    typeof result.value.filename !== 'string' ||
    result.value.filename.length === 0
  ) {
    throw new Error('Native media save failed');
  }
  return result.value.filename;
}

/** Leftover `mxc://` and opaque handles must not be used as `<img src>`. */
export function isNativeMediaContentUri(contentUri: string | undefined): boolean {
  if (!contentUri) return false;
  const trimmed = contentUri.trim();
  return (
    trimmed.startsWith('mxc://') ||
    trimmed.startsWith('synara-media://') ||
    trimmed.startsWith(TIMELINE_MEDIA_HANDLE_PREFIX)
  );
}

export async function createMatrixMediaObjectUrl(
  mx: MatrixMediaClient,
  mxcUrl: string,
  options: MatrixMediaDownloadOptions
): Promise<string> {
  void mx;
  const resolved = assertDisplayableMedia(mxcUrl, options.encryptedInfo);
  const displayUrl = convertDesktopFileSrc(resolved, 'synara-media');
  if (!displayUrl) throw new Error('Native media display is unavailable');
  return displayUrl;
}
