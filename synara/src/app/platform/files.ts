import {
  readDesktopDroppedFiles,
  saveDesktopFile,
  type DesktopNativeFileDropPayload,
} from '../utils/desktop';

export type PlatformNativeFileDropPayload = DesktopNativeFileDropPayload;

export const savePlatformFile = saveDesktopFile;
export const readPlatformDroppedFiles = readDesktopDroppedFiles;
