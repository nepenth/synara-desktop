import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../../utils/desktop';
import {
  getOwnProfileWithNativeOwner,
  setOwnAvatarWithNativeOwner,
  setOwnDisplayNameWithNativeOwner,
  uploadMediaWithNativeOwner,
  type NativeOwnProfile,
} from './nativeProfileOwner';

const invoke = (command: string, args?: Record<string, unknown>) =>
  invokeDesktopWithAvailability(command, args);

export const OWN_PROFILE_CHANGED_EVENT = 'synara-own-profile-changed';

export const notifyOwnProfileChanged = (): void => {
  if (typeof window === 'undefined') return;
  window.dispatchEvent(new Event(OWN_PROFILE_CHANGED_EVENT));
};

export const setOwnDisplayNameNative = (displayName: string): Promise<'native' | 'legacy'> =>
  setOwnDisplayNameWithNativeOwner(displayName, isSynaraDesktop(), invoke);

export const setOwnAvatarNative = (mxc: string): Promise<'native' | 'legacy'> =>
  setOwnAvatarWithNativeOwner(mxc, isSynaraDesktop(), invoke);

export const uploadMediaNative = (
  mimeType: string,
  bytes: number[]
): Promise<{ mxc: string } | 'legacy'> =>
  uploadMediaWithNativeOwner(mimeType, bytes, isSynaraDesktop(), invoke);

export const getOwnProfileNative = (): Promise<NativeOwnProfile | 'legacy'> =>
  getOwnProfileWithNativeOwner(isSynaraDesktop(), invoke);
