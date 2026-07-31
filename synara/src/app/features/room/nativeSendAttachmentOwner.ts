import type { DesktopInvokeResult } from '../../utils/desktop';

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
};

export type NativeSendAttachmentFile = {
  filename: string;
  mimeType: string;
  bytes: number[];
};

export type NativeSendAttachmentInput = {
  roomId: string;
  file: NativeSendAttachmentFile;
  replyTo?: string;
  /** Thread root event id; forces EnforceThread::Threaded on the native owner. */
  threadRoot?: string;
};

export type NativeSendAttachmentResult = {
  roomId: string;
  eventId: string;
  localTxnId: string;
  status: 'sent';
};

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

export async function isNativeMatrixLoggedIn(
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<boolean> {
  if (!desktopAvailable) return false;
  const session = await invoke('matrix_session_snapshot');
  if (!session.available) return false;
  const snapshot = session.value as NativeSessionSnapshot | undefined;
  return snapshot?.status === 'logged_in';
}

/**
 * Sole composer attachment upload/send owner when a native session is live.
 * Never falls through to `mx.uploadContent` / `mx.sendMessage`.
 */
export async function sendAttachmentWithNativeOwner(
  input: NativeSendAttachmentInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<'native' | 'legacy'> {
  if (!(await isNativeMatrixLoggedIn(desktopAvailable, invoke))) {
    return 'legacy';
  }

  const send = await invoke('matrix_send_attachment', {
    roomId: input.roomId,
    filename: input.file.filename,
    mimeType: input.file.mimeType,
    bytes: input.file.bytes,
    replyTo: input.replyTo,
    threadRoot: input.threadRoot,
  });
  if (!send.available) {
    throw new Error('Native Matrix attachment send is unavailable.');
  }
  const result = send.value as NativeSendAttachmentResult | undefined;
  if (result?.status !== 'sent') {
    throw new Error('Native Matrix attachment send is unavailable.');
  }
  return 'native';
}

export async function sendAttachmentsWithNativeOwner(
  roomId: string,
  files: NativeSendAttachmentFile[],
  replyTo: string | undefined,
  threadRoot: string | undefined,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<'native' | 'legacy'> {
  if (!(await isNativeMatrixLoggedIn(desktopAvailable, invoke))) {
    return 'legacy';
  }
  for (const file of files) {
    const owner = await sendAttachmentWithNativeOwner(
      { roomId, file, replyTo, threadRoot },
      desktopAvailable,
      invoke
    );
    if (owner !== 'native') {
      throw new Error('Native Matrix attachment send is unavailable.');
    }
  }
  return 'native';
}
