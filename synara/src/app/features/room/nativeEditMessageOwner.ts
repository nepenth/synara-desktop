import type { DesktopInvokeResult } from '../../utils/desktop';

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
};

export type NativeEditMessageInput = {
  roomId: string;
  /** The original event id being replaced (`m.replace` target). */
  eventId: string;
  body: string;
  msgType?: string;
  formattedBody?: string;
  mentionUserIds?: string[];
  mentionRoom?: boolean;
  txnId?: string;
};

export type NativeEditMessageResult = {
  roomId: string;
  eventId: string;
  localTxnId: string;
  status: 'sent';
};

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

/**
 * Sole native message-edit owner when a native Matrix session is live.
 *
 * V-SEND.R-EDIT: when a native session is live this is the only path that sends
 * the `m.replace` edit. It never falls through to `mx.sendMessage` — a native
 * command failure throws (fail-closed). The legacy `mx.sendMessage` edit path is
 * only used when no native session is live (web / logged-out).
 */
export async function editMessageWithNativeOwner(
  input: NativeEditMessageInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<'native' | 'legacy'> {
  if (!desktopAvailable) return 'legacy';

  const session = await invoke('matrix_session_snapshot');
  if (!session.available) return 'legacy';
  const snapshot = session.value as NativeSessionSnapshot | undefined;
  if (snapshot?.status !== 'logged_in') return 'legacy';

  const edit = await invoke('matrix_edit_message', {
    roomId: input.roomId,
    eventId: input.eventId,
    body: input.body,
    msgType: input.msgType,
    formattedBody: input.formattedBody,
    mentionUserIds: input.mentionUserIds,
    mentionRoom: input.mentionRoom,
    txnId: input.txnId,
  });
  if (!edit.available) {
    throw new Error('Native Matrix message edit is unavailable.');
  }
  const result = edit.value as NativeEditMessageResult | undefined;
  if (result?.status !== 'sent') {
    throw new Error('Native Matrix message edit is unavailable.');
  }
  return 'native';
}
