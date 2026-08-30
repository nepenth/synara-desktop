import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import {
  isNativeMatrixLoggedIn,
  sendAttachmentPlanWithNativeOwner,
  type NativeSendAttachmentInput,
} from './nativeSendAttachmentOwner';

export const nativeComposerAttachmentReady = (): Promise<boolean> =>
  isNativeMatrixLoggedIn(isSynaraDesktop(), (command, args) =>
    invokeDesktopWithAvailability(command, args)
  );

export const sendComposerAttachmentPlanWithNativeOwner = (
  inputs: NativeSendAttachmentInput[],
  onSent: (index: number) => void | Promise<void>
): Promise<'native' | 'legacy'> =>
  sendAttachmentPlanWithNativeOwner(
    inputs,
    isSynaraDesktop(),
    (command, args) => invokeDesktopWithAvailability(command, args),
    onSent
  );

export async function fileToNativeAttachmentBytes(file: Blob): Promise<number[]> {
  const buffer = await file.arrayBuffer();
  return Array.from(new Uint8Array(buffer));
}
