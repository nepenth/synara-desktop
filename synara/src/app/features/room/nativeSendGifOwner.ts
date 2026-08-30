import type { GifResult } from '../../utils/gifProvider';
import { fetchGifForUpload } from '../../utils/gifProvider';
import {
  isNativeMatrixLoggedIn,
  sendAttachmentWithNativeOwner,
  type NativeInvoke,
} from './nativeSendAttachmentOwner';

export type NativeSendGifInput = {
  roomId: string;
  gif: GifResult;
  transactionId?: string;
  replyTo?: string;
  /** Thread root event id; forces native attachment thread relation. */
  threadRoot?: string;
};

/**
 * Sole composer GIF upload/send owner when a native session is live.
 * Reuses `matrix_send_attachment` (image/gif bytes); never falls through to
 * a legacy JS upload/send path.
 */
export async function sendGifWithNativeOwner(
  input: NativeSendGifInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke,
  fetchGif: typeof fetchGifForUpload = fetchGifForUpload
): Promise<'native'> {
  if (!(await isNativeMatrixLoggedIn(desktopAvailable, invoke))) {
    throw new Error('Native Matrix GIF send is unavailable.');
  }

  const { blob, fileName } = await fetchGif(input.gif);
  const buffer = await blob.arrayBuffer();
  const bytes = Array.from(new Uint8Array(buffer));
  const owner = await sendAttachmentWithNativeOwner(
    {
      roomId: input.roomId,
      transactionId: input.transactionId ?? `synara-gif-${crypto.randomUUID()}`,
      file: {
        filename: fileName || 'gif.gif',
        mimeType: 'image/gif',
        bytes,
      },
      replyTo: input.replyTo,
      threadRoot: input.threadRoot,
    },
    desktopAvailable,
    invoke
  );
  if (owner !== 'native') {
    throw new Error('Native Matrix GIF send is unavailable.');
  }
  return 'native';
}
