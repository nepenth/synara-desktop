import { invokeDesktopWithAvailability, isSynaraDesktop } from '../utils/desktop';
import type { DesktopInvokeResult } from '../utils/desktop';
import { isNativeMatrixSession } from '../features/verification/nativeVerification';

export type NativeStateEventInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

const unavailableMessage = 'Native Matrix state event send is unavailable.';

const invokeSafely = async (
  command: string,
  args: Record<string, unknown> | undefined,
  invoke: NativeStateEventInvoke
): Promise<DesktopInvokeResult<unknown>> => {
  try {
    return await invoke(command, args);
  } catch {
    throw new Error(unavailableMessage);
  }
};

export const desktopUsesNativeStateEventOwner = (): boolean =>
  isSynaraDesktop() && isNativeMatrixSession();

/**
 * Native-session leftover state writes. Fail-closed on desktop — never JS
 * plaintext for encryptable types in StateEncrypted rooms.
 */
export async function sendStateEventWithNativeOwner(
  roomId: string,
  eventType: string,
  content: Record<string, unknown>,
  stateKey = '',
  invoke: NativeStateEventInvoke = (command, args) =>
    invokeDesktopWithAvailability(command, args),
  nativeAvailable = desktopUsesNativeStateEventOwner()
): Promise<void> {
  if (!nativeAvailable) {
    throw new Error(unavailableMessage);
  }
  const result = await invokeSafely(
    'matrix_send_state_event',
    { roomId, eventType, stateKey, content },
    invoke
  );
  if (!result.available) {
    throw new Error(unavailableMessage);
  }
}

export async function enableRoomEncryptedStateWithNativeOwner(
  roomId: string,
  encryptStateEvents: boolean,
  invoke: NativeStateEventInvoke = (command, args) =>
    invokeDesktopWithAvailability(command, args),
  nativeAvailable = desktopUsesNativeStateEventOwner()
): Promise<void> {
  if (!nativeAvailable) {
    throw new Error(unavailableMessage);
  }
  const result = await invokeSafely(
    'matrix_enable_room_encrypted_state',
    { roomId, encryptStateEvents },
    invoke
  );
  if (!result.available) {
    throw new Error(unavailableMessage);
  }
}

/** Native session leftover writes; JS only when this is not a native desktop session. */
export async function sendLeftoverStateEvent(
  roomId: string,
  eventType: string,
  content: Record<string, unknown>,
  stateKey: string | undefined,
  jsSend: () => Promise<unknown>,
  nativeAvailable = desktopUsesNativeStateEventOwner(),
  invoke: NativeStateEventInvoke = (command, args) =>
    invokeDesktopWithAvailability(command, args)
): Promise<void> {
  if (nativeAvailable) {
    await sendStateEventWithNativeOwner(
      roomId,
      eventType,
      content,
      stateKey ?? '',
      invoke,
      true
    );
    return;
  }
  await jsSend();
}
