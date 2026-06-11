import {
  DESKTOP_FILE_IPC_CHUNK_SIZE,
  DESKTOP_FILE_IPC_INLINE_THRESHOLD,
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
