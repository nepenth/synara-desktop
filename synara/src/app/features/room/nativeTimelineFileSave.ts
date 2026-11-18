import { saveMatrixMediaFile, type MatrixMediaSaveEvent } from '../../matrix/media';

const FALLBACK_FILE_NAME = 'download';

/** Filename shown on the chip and used for the save. */
export const nativeTimelineFileDownloadName = (filename?: string): string => {
  const trimmed = filename?.trim();
  return trimmed && trimmed.length > 0 ? trimmed : FALLBACK_FILE_NAME;
};

/**
 * Save a native-timeline file attachment in Rust.
 *
 * The synara-media protocol only serves magic-sniffed inline types (image,
 * audio, video, PDF). Generic files including `text/markdown` are not displayed
 * from that URL. Rust downloads and writes the file, then returns the name it
 * got in Downloads (`name (1).ext` when the name was taken).
 */
export const saveNativeTimelineFileAttachment = async (
  {
    handleId,
    filename,
  }: {
    handleId: string;
    filename?: string;
    mimeType?: string;
  },
  onProgress?: (event: MatrixMediaSaveEvent) => void
): Promise<string> => {
  const trimmedHandle = handleId.trim();
  if (!trimmedHandle) {
    throw new Error('File attachment is unavailable.');
  }
  return saveMatrixMediaFile(trimmedHandle, nativeTimelineFileDownloadName(filename), onProgress);
};
