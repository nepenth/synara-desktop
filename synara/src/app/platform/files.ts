import {
  DESKTOP_FILE_IPC_CHUNK_SIZE,
  DESKTOP_FILE_IPC_INLINE_THRESHOLD,
  readDesktopClipboardImage,
  readDesktopDroppedFiles,
  saveDesktopFile,
  shouldStreamDesktopFileIpc,
  type DesktopNativeFileDropPayload,
} from '../utils/desktop';

export type PlatformNativeFileDropPayload = DesktopNativeFileDropPayload;

export {
  DESKTOP_FILE_IPC_CHUNK_SIZE,
  DESKTOP_FILE_IPC_INLINE_THRESHOLD,
  shouldStreamDesktopFileIpc,
};

export const savePlatformFile = saveDesktopFile;
export const readPlatformDroppedFiles = readDesktopDroppedFiles;
export const readPlatformClipboardImage = readDesktopClipboardImage;
