import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import type { ImagePack } from '../../plugins/custom-emoji/ImagePack';
import {
  getGlobalImagePacksWithNativeOwner,
  getRoomImagePacksWithNativeOwner,
  getUserImagePackWithNativeOwner,
} from './nativeImagePackOwner';

const invoke = (command: string, args?: Record<string, unknown>) =>
  invokeDesktopWithAvailability(command, args);

export const getUserImagePackNative = (): Promise<ImagePack | undefined | 'legacy'> =>
  getUserImagePackWithNativeOwner(isSynaraDesktop(), invoke);

export const getRoomImagePacksNative = (roomId: string): Promise<ImagePack[] | 'legacy'> =>
  getRoomImagePacksWithNativeOwner(roomId, isSynaraDesktop(), invoke);

export const getGlobalImagePacksNative = (): Promise<ImagePack[] | 'legacy'> =>
  getGlobalImagePacksWithNativeOwner(isSynaraDesktop(), invoke);
