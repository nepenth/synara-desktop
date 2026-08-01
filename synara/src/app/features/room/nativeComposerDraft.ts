import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import {
  clearReplyDraftWithNativeComposerOwner,
  getReplyDraftWithNativeComposerOwner,
  setReplyDraftWithNativeComposerOwner,
  type NativeComposerReplyDraftReadback,
  type NativeComposerReplyDraftRoomInput,
  type NativeComposerSetReplyDraftInput,
} from './nativeComposerDraftOwner';

const invoke: Parameters<typeof setReplyDraftWithNativeComposerOwner>[2] = (command, args) =>
  invokeDesktopWithAvailability(command, args);

export const setNativeComposerReplyDraft = (
  input: NativeComposerSetReplyDraftInput
): Promise<NativeComposerReplyDraftReadback | 'unavailable'> =>
  setReplyDraftWithNativeComposerOwner(input, isSynaraDesktop(), invoke);

export const clearNativeComposerReplyDraft = (
  input: NativeComposerReplyDraftRoomInput
): Promise<NativeComposerReplyDraftReadback | 'unavailable'> =>
  clearReplyDraftWithNativeComposerOwner(input, isSynaraDesktop(), invoke);

export const getNativeComposerReplyDraft = (
  input: NativeComposerReplyDraftRoomInput
): Promise<NativeComposerReplyDraftReadback | 'unavailable'> =>
  getReplyDraftWithNativeComposerOwner(input, isSynaraDesktop(), invoke);

export { mapNativeReplyDraftToJs } from './nativeComposerDraftOwner';
