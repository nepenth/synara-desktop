import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import type { GifResult } from '../../utils/gifProvider';
import { sendGifWithNativeOwner } from './nativeSendGifOwner';

export const sendComposerGifWithNativeOwner = (
  roomId: string,
  gif: GifResult,
  replyTo?: string,
  threadRoot?: string
): Promise<'native' | 'legacy'> =>
  sendGifWithNativeOwner(
    { roomId, gif, replyTo, threadRoot },
    isSynaraDesktop(),
    (command, args) => invokeDesktopWithAvailability(command, args)
  );
