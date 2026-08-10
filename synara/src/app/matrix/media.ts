import type { EncryptedAttachmentInfo } from 'browser-encrypt-attachment';
import { decryptFile, downloadEncryptedMedia, downloadMedia, mxcUrlToHttp } from '../utils/matrix';

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

export async function downloadMatrixMedia(
  mx: MatrixMediaClient,
  mxcUrl: string,
  options: MatrixMediaDownloadOptions
): Promise<Blob> {
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
