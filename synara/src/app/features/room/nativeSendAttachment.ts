import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import {
  isNativeMatrixLoggedIn,
  sendAttachmentWithNativeOwner,
  type NativeSendAttachmentInput,
} from './nativeSendAttachmentOwner';

export const nativeComposerAttachmentReady = (): Promise<boolean> =>
  isNativeMatrixLoggedIn(isSynaraDesktop(), (command, args) =>
    invokeDesktopWithAvailability(command, args)
  );

/**
 * Send one composer file. The composer converts and sends files one at a
 * time, so only the file being sent is held in memory as bytes.
 */
export const sendComposerAttachmentWithNativeOwner = (
  input: NativeSendAttachmentInput
): Promise<'native' | 'legacy'> =>
  sendAttachmentWithNativeOwner(input, isSynaraDesktop(), (command, args) =>
    invokeDesktopWithAvailability(command, args)
  );

export async function fileToNativeAttachmentBytes(file: Blob): Promise<number[]> {
  const buffer = await file.arrayBuffer();
  return Array.from(new Uint8Array(buffer));
}
