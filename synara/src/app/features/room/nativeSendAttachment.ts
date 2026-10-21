import {
  invokeDesktopBodyWithAvailability,
  invokeDesktopWithAvailability,
  isSynaraDesktop,
} from '../../utils/desktop';
import {
  isNativeMatrixLoggedIn,
  sendAttachmentWithNativeOwner,
  type NativeInvoke,
  type NativeSendAttachmentInput,
} from './nativeSendAttachmentOwner';

/** Desktop invoke for the attachment owners: bytes go as a raw body. */
export const nativeAttachmentInvoke: NativeInvoke = (command, args) =>
  args instanceof Uint8Array
    ? invokeDesktopBodyWithAvailability(command, args)
    : invokeDesktopWithAvailability(command, args);

export const nativeComposerAttachmentReady = (): Promise<boolean> =>
  isNativeMatrixLoggedIn(isSynaraDesktop(), nativeAttachmentInvoke);

/**
 * Send one composer file. The composer converts and sends files one at a
 * time, so only the file being sent is held in memory as bytes.
 */
export const sendComposerAttachmentWithNativeOwner = (
  input: NativeSendAttachmentInput
): Promise<'native' | 'legacy'> =>
  sendAttachmentWithNativeOwner(input, isSynaraDesktop(), nativeAttachmentInvoke);

/** The file's bytes for the raw-body attachment stage (no number array). */
export async function fileToNativeAttachmentBytes(file: Blob): Promise<Uint8Array> {
  return new Uint8Array(await file.arrayBuffer());
}
