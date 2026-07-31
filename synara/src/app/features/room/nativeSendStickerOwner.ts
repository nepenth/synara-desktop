import type { DesktopInvokeResult } from '../../utils/desktop';
import { isNativeMatrixLoggedIn, type NativeInvoke } from './nativeSendAttachmentOwner';

export type NativeSendStickerInfo = {
  width?: number;
  height?: number;
  mimetype?: string;
  size?: number;
};

export type NativeSendStickerInput = {
  roomId: string;
  body: string;
  mxc: string;
  info?: NativeSendStickerInfo;
  replyTo?: string;
  /** Thread root event id (`m.thread`). Paired with `replyTo` for in-thread stickers. */
  threadRoot?: string;
};

export type NativeSendStickerResult = {
  roomId: string;
  eventId: string;
  status: 'sent';
};

/**
 * Sole composer sticker (`m.sticker`) owner when a native session is live.
 * Never falls through to `mx.sendEvent(EventType.Sticker)`.
 */
export async function sendStickerWithNativeOwner(
  input: NativeSendStickerInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<'native' | 'legacy'> {
  if (!(await isNativeMatrixLoggedIn(desktopAvailable, invoke))) {
    return 'legacy';
  }

  const send = await invoke('matrix_send_sticker', {
    roomId: input.roomId,
    body: input.body,
    mxc: input.mxc,
    width: input.info?.width,
    height: input.info?.height,
    mimetype: input.info?.mimetype,
    size: input.info?.size,
    replyTo: input.replyTo,
    threadRoot: input.threadRoot,
  });
  if (!send.available) {
    throw new Error('Native Matrix sticker send is unavailable.');
  }
  const result = send.value as NativeSendStickerResult | undefined;
  if (result?.status !== 'sent') {
    throw new Error('Native Matrix sticker send is unavailable.');
  }
  return 'native';
}
