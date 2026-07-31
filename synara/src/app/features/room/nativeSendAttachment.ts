import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import {
  isNativeMatrixLoggedIn,
  sendAttachmentsWithNativeOwner,
  type NativeSendAttachmentFile,
} from './nativeSendAttachmentOwner';

export const nativeComposerAttachmentReady = (): Promise<boolean> =>
  isNativeMatrixLoggedIn(isSynaraDesktop(), (command, args) =>
    invokeDesktopWithAvailability(command, args)
  );

export const sendComposerAttachmentsWithNativeOwner = (
  roomId: string,
  files: NativeSendAttachmentFile[],
  replyTo?: string,
  threadRoot?: string
): Promise<'native' | 'legacy'> =>
  sendAttachmentsWithNativeOwner(
    roomId,
    files,
    replyTo,
    threadRoot,
    isSynaraDesktop(),
    (command, args) => invokeDesktopWithAvailability(command, args)
  );

export async function fileToNativeAttachmentBytes(file: Blob): Promise<number[]> {
  const buffer = await file.arrayBuffer();
  return Array.from(new Uint8Array(buffer));
}
