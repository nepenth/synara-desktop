import { isSynaraDesktop } from '../../utils/desktop';
import type { GifResult } from '../../utils/gifProvider';
import { nativeAttachmentInvoke } from './nativeSendAttachment';
import { sendGifWithNativeOwner } from './nativeSendGifOwner';

export const sendComposerGifWithNativeOwner = (
  roomId: string,
  gif: GifResult,
  replyTo?: string,
  threadRoot?: string
): Promise<'native'> =>
  sendGifWithNativeOwner(
    { roomId, gif, replyTo, threadRoot },
    isSynaraDesktop(),
    nativeAttachmentInvoke
  );
