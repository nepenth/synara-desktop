import type { DesktopInvokeResult } from '../../utils/desktop';

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
};

export type NativeSendPollInput = {
  roomId: string;
  question: string;
  answers: string[];
  maxSelections: number;
};

export type NativePollRespondInput = {
  roomId: string;
  pollEventId: string;
  answerIds: string[];
};

export type NativePollSendResult = {
  roomId: string;
  eventId: string;
  status: 'sent';
};

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>,
) => Promise<DesktopInvokeResult<unknown>>;

async function isNativeMatrixLoggedIn(
  desktopAvailable: boolean,
  invoke: NativeInvoke,
): Promise<boolean> {
  if (!desktopAvailable) return false;
  const session = await invoke('matrix_session_snapshot');
  if (!session.available) return false;
  const snapshot = session.value as NativeSessionSnapshot | undefined;
  return snapshot?.status === 'logged_in';
}

/**
 * Sole poll-start owner when a native session is live.
 * Never falls through to `mx.sendEvent` for poll.start.
 */
export async function sendPollWithNativeOwner(
  input: NativeSendPollInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke,
): Promise<'native' | 'legacy'> {
  if (!(await isNativeMatrixLoggedIn(desktopAvailable, invoke))) {
    return 'legacy';
  }

  const send = await invoke('matrix_send_poll', {
    roomId: input.roomId,
    question: input.question,
    answers: input.answers,
    maxSelections: input.maxSelections,
  });
  if (!send.available) {
    throw new Error('Native Matrix poll send is unavailable.');
  }
  const result = send.value as NativePollSendResult | undefined;
  if (result?.status !== 'sent') {
    throw new Error('Native Matrix poll send is unavailable.');
  }
  return 'native';
}

/**
 * Sole poll-response (vote) owner when a native session is live.
 * Never falls through to `mx.sendEvent` for poll.response.
 */
export async function respondPollWithNativeOwner(
  input: NativePollRespondInput,
  desktopAvailable: boolean,
  invoke: NativeInvoke,
): Promise<'native' | 'legacy'> {
  if (!(await isNativeMatrixLoggedIn(desktopAvailable, invoke))) {
    return 'legacy';
  }

  const send = await invoke('matrix_poll_respond', {
    roomId: input.roomId,
    pollEventId: input.pollEventId,
    answerIds: input.answerIds,
  });
  if (!send.available) {
    throw new Error('Native Matrix poll response is unavailable.');
  }
  const result = send.value as NativePollSendResult | undefined;
  if (result?.status !== 'sent') {
    throw new Error('Native Matrix poll response is unavailable.');
  }
  return 'native';
}
