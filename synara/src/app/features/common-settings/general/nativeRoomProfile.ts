import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../../utils/desktop';
import {
  getRoomDirectoryVisibilityWithNativeOwner,
  setRoomAvatarWithNativeOwner,
  setRoomDirectoryVisibilityWithNativeOwner,
  setRoomNameWithNativeOwner,
  setRoomTopicWithNativeOwner,
  type NativeRoomDirectoryVisibility,
  type NativeRoomDirectoryVisibilityResult,
  type NativeRoomDirectoryVisibilityWriteResult,
} from './nativeRoomProfileOwner';

const invoke = (command: string, args?: Record<string, unknown>) =>
  invokeDesktopWithAvailability(command, args);

export const setRoomNameNative = (roomId: string, name: string): Promise<'native' | 'legacy'> =>
  setRoomNameWithNativeOwner(roomId, name, isSynaraDesktop(), invoke);

export const setRoomTopicNative = (roomId: string, topic: string): Promise<'native' | 'legacy'> =>
  setRoomTopicWithNativeOwner(roomId, topic, isSynaraDesktop(), invoke);

export const setRoomAvatarNative = (roomId: string, mxc: string): Promise<'native' | 'legacy'> =>
  setRoomAvatarWithNativeOwner(roomId, mxc, isSynaraDesktop(), invoke);

export const getRoomDirectoryVisibilityNative = (
  roomId: string
): Promise<NativeRoomDirectoryVisibilityResult> =>
  getRoomDirectoryVisibilityWithNativeOwner(roomId, isSynaraDesktop(), invoke);

export const setRoomDirectoryVisibilityNative = (
  roomId: string,
  visibility: NativeRoomDirectoryVisibility
): Promise<NativeRoomDirectoryVisibilityWriteResult> =>
  setRoomDirectoryVisibilityWithNativeOwner(roomId, visibility, isSynaraDesktop(), invoke);
