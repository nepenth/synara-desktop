import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { sendTextWithNativeOwner, type NativeSendTextInput } from './nativeSendTextOwner';

export const sendPlainTextWithNativeOwner = (
  input: NativeSendTextInput
): Promise<'native' | 'legacy'> =>
  sendTextWithNativeOwner(input, isSynaraDesktop(), (command, args) =>
    invokeDesktopWithAvailability(command, args)
  );
