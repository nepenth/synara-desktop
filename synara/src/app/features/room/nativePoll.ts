import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import {
  respondPollWithNativeOwner,
  sendPollCommandWithNativeOwner,
  sendPollWithNativeOwner,
  type NativePollRespondInput,
  type NativeSendPollInput,
} from './nativePollOwner';

export async function sendPollWithNativeDesktopOwner(
  input: NativeSendPollInput
): Promise<'native' | 'legacy'> {
  return sendPollWithNativeOwner(input, isSynaraDesktop(), (command, args) =>
    invokeDesktopWithAvailability(command, args)
  );
}

export async function sendPollCommandWithNativeDesktopOwner(
  input: NativeSendPollInput,
  onNativeSent: () => void | Promise<void>
): Promise<'native' | 'legacy'> {
  return sendPollCommandWithNativeOwner(input, onNativeSent, isSynaraDesktop(), (command, args) =>
    invokeDesktopWithAvailability(command, args)
  );
}

export async function respondPollWithNativeDesktopOwner(
  input: NativePollRespondInput
): Promise<'native' | 'legacy'> {
  return respondPollWithNativeOwner(input, isSynaraDesktop(), (command, args) =>
    invokeDesktopWithAvailability(command, args)
  );
}
