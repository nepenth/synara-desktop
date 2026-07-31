import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import {
  sendStickerWithNativeOwner,
  type NativeSendStickerInput,
} from './nativeSendStickerOwner';

export const sendComposerStickerWithNativeOwner = (
  input: NativeSendStickerInput
): Promise<'native' | 'legacy'> =>
  sendStickerWithNativeOwner(input, isSynaraDesktop(), (command, args) =>
    invokeDesktopWithAvailability(command, args)
  );
