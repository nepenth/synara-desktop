export const NATIVE_ATTACHMENT_MAX_BYTES = 32 * 1024 * 1024;

export function effectiveNativeAttachmentLimit(serverLimit: number | undefined): number {
  if (typeof serverLimit !== 'number' || Number.isFinite(serverLimit) === false) {
    return NATIVE_ATTACHMENT_MAX_BYTES;
  }
  return Math.min(serverLimit, NATIVE_ATTACHMENT_MAX_BYTES);
}
