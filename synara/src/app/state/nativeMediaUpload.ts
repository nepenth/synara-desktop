import { invokeDesktopWithAvailability, isSynaraDesktop } from '../utils/desktop';
import { uploadMediaWithNativeOwner } from './nativeMediaUploadOwner';

const invoke = (command: string, args?: Record<string, unknown>) =>
  invokeDesktopWithAvailability(command, args);

/** V-SEND.R-PACK-UPLOAD: desktop fail-closed pack/media upload via matrix_upload_media. */
export const uploadMediaNative = (
  mimeType: string,
  bytes: number[]
): Promise<{ mxc: string } | 'legacy'> =>
  uploadMediaWithNativeOwner(mimeType, bytes, isSynaraDesktop(), invoke);
