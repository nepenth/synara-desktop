import { bytesToSize } from './common';

export const NATIVE_ATTACHMENT_MAX_BYTES = 32 * 1024 * 1024;

export function effectiveNativeAttachmentLimit(serverLimit: number | undefined): number {
  if (typeof serverLimit !== 'number' || Number.isFinite(serverLimit) === false) {
    return NATIVE_ATTACHMENT_MAX_BYTES;
  }
  return Math.min(serverLimit, NATIVE_ATTACHMENT_MAX_BYTES);
}

/**
 * The homeserver upload limit from the media config. The native reader
 * (`parseMediaConfig`) names it `maxUploadSizeBytes`; the Matrix wire key
 * `m.upload.size` is still read for configs from elsewhere.
 */
export function mediaConfigUploadLimit(config: {
  maxUploadSizeBytes?: unknown;
  'm.upload.size'?: unknown;
}): number | undefined {
  const value = config.maxUploadSizeBytes ?? config['m.upload.size'];
  return typeof value === 'number' && Number.isFinite(value) && value > 0 ? value : undefined;
}

/** Card and composer copy for a file over the upload limit. */
export function attachmentTooLargeMessage(name: string, size: number, limit: number): string {
  return `"${name}" is ${bytesToSize(size)}, over the ${bytesToSize(
    limit
  )} upload limit. Remove it to send.`;
}
