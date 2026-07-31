import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import {
  editMessageWithNativeOwner,
  type NativeEditMessageInput,
} from './nativeEditMessageOwner';

export async function editMessageWithNativeDesktopOwner(
  input: NativeEditMessageInput
): Promise<'native' | 'legacy'> {
  return editMessageWithNativeOwner(input, isSynaraDesktop(), (command, args) =>
    invokeDesktopWithAvailability(command, args)
  );
}
