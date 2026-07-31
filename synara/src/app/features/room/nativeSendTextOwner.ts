import type { DesktopInvokeResult } from '../../utils/desktop';

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
};

export type NativeSendTextInput = {
  roomId: string;
  body: string;
  msgType?: string;
  formattedBody?: string;
  mentionUserIds?: string[];
  mentionRoom?: boolean;
  replyTo?: string;
  txnId?: string;
};

export type NativeSendTextResult = {
  roomId: string;
  eventId: string;
  localTxnId: string;
  status: 'sent';
};

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

export async function sendTextWithNativeOwner(
  input: NativeSendTextInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<'native' | 'legacy'> {
  if (!desktopAvailable) return 'legacy';

  const session = await invoke('matrix_session_snapshot');
  if (!session.available) return 'legacy';
  const snapshot = session.value as NativeSessionSnapshot | undefined;
  if (snapshot?.status !== 'logged_in') return 'legacy';

  const send = await invoke('matrix_send_text', input);
  if (!send.available) {
    throw new Error('Native Matrix text send is unavailable.');
  }
  const result = send.value as NativeSendTextResult | undefined;
  if (result?.status !== 'sent') throw new Error('Native Matrix text send is unavailable.');
  return 'native';
}
