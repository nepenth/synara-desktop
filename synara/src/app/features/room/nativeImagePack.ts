import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import type { ImagePack } from '../../plugins/custom-emoji/ImagePack';
import type { PackContent, EmoteRoomsContent } from '../../plugins/custom-emoji/types';
import {
  getGlobalImagePacksWithNativeOwner,
  getRoomImagePacksWithNativeOwner,
  getUserImagePackWithNativeOwner,
  setGlobalImagePacksWithNativeOwner,
  setUserImagePackWithNativeOwner,
} from './nativeImagePackOwner';

const invoke = (command: string, args?: Record<string, unknown>) =>
  invokeDesktopWithAvailability(command, args);

export const getUserImagePackNative = (): Promise<ImagePack | undefined | 'legacy'> =>
  getUserImagePackWithNativeOwner(isSynaraDesktop(), invoke);

export const getRoomImagePacksNative = (roomId: string): Promise<ImagePack[] | 'legacy'> =>
  getRoomImagePacksWithNativeOwner(roomId, isSynaraDesktop(), invoke);

export const getGlobalImagePacksNative = (): Promise<ImagePack[] | 'legacy'> =>
  getGlobalImagePacksWithNativeOwner(isSynaraDesktop(), invoke);

export const setUserImagePackNative = (content: PackContent): Promise<'native' | 'legacy'> =>
  setUserImagePackWithNativeOwner(content, isSynaraDesktop(), invoke);

export const setGlobalImagePacksNative = (
  content: EmoteRoomsContent
): Promise<'native' | 'legacy'> =>
  setGlobalImagePacksWithNativeOwner(content, isSynaraDesktop(), invoke);
