import {
  DESKTOP_FILE_IPC_CHUNK_SIZE,
  DESKTOP_FILE_IPC_INLINE_THRESHOLD,
  readDesktopClipboardImage,
  readDesktopClipboardText,
  readDesktopDroppedFiles,
  saveDesktopFile,
  shouldStreamDesktopFileIpc,
  type DesktopDroppedFiles,
  type DesktopNativeFileDropPayload,
} from '../utils/desktop';

export type PlatformNativeFileDropPayload = DesktopNativeFileDropPayload;
export type PlatformDroppedFiles = DesktopDroppedFiles;

export {
  DESKTOP_FILE_IPC_CHUNK_SIZE,
  DESKTOP_FILE_IPC_INLINE_THRESHOLD,
  shouldStreamDesktopFileIpc,
};

export const savePlatformFile = saveDesktopFile;
export const readPlatformDroppedFiles = readDesktopDroppedFiles;
export const readPlatformClipboardImage = readDesktopClipboardImage;
export const readPlatformClipboardText = readDesktopClipboardText;
