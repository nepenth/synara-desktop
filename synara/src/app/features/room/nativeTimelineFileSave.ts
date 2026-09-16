import FileSaver from 'file-saver';
import { downloadTimelineMediaBytes } from '../../matrix/media';
import { savePlatformFile, supportsPlatformNativeFileSave } from '../../platform';

const FALLBACK_FILE_NAME = 'download';
const FALLBACK_MIME_TYPE = 'application/octet-stream';

/** Filename shown on the chip and used for the save dialog. */
export const nativeTimelineFileDownloadName = (filename?: string): string => {
  const trimmed = filename?.trim();
  return trimmed && trimmed.length > 0 ? trimmed : FALLBACK_FILE_NAME;
};

/**
 * Save a native-timeline file attachment through the download+save owners.
 *
 * The synara-media protocol only serves magic-sniffed inline types (image,
 * audio, video, PDF). Generic files including `text/markdown` 415 there, so
 * `<a href={protocol} download>` is a no-op. This path works for every
 * Core `messageType: 'file'` row, markdown included.
 */
export const saveNativeTimelineFileAttachment = async ({
  handleId,
  filename,
  mimeType,
}: {
  handleId: string;
  filename?: string;
  mimeType?: string;
}): Promise<void> => {
  const trimmedHandle = handleId.trim();
  if (!trimmedHandle) {
    throw new Error('File attachment is unavailable.');
  }
  const blob = await downloadTimelineMediaBytes(
    trimmedHandle,
    mimeType?.trim() || FALLBACK_MIME_TYPE
  );
  const name = nativeTimelineFileDownloadName(filename);
  if (supportsPlatformNativeFileSave()) {
    await savePlatformFile(blob, name);
    return;
  }
  FileSaver.saveAs(blob, name);
};
