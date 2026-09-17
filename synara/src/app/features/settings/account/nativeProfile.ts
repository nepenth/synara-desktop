import { invokeDesktopWithAvailability, isSynaraDesktop, listen } from '../../../utils/desktop';
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
export const MATRIX_OWN_PROFILE_CHANGED_EVENT = 'matrix-own-profile-changed';

const isSafeMxc = (value: unknown): value is string =>
  typeof value === 'string' && value.startsWith('mxc://') && value.split('/').length >= 4;

export const parseOwnProfilePush = (value: unknown): NativeOwnProfile | null => {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
  const body = value as Record<string, unknown>;
  if (typeof body.userId !== 'string' || !body.userId.startsWith('@')) return null;
  const displayName = typeof body.displayName === 'string' ? body.displayName : undefined;
  const avatarUrl = isSafeMxc(body.avatarUrl) ? body.avatarUrl : undefined;
  return { userId: body.userId, displayName, avatarUrl };
};

export const notifyOwnProfileChanged = (): void => {
  if (typeof window === 'undefined') return;
  window.dispatchEvent(new Event(OWN_PROFILE_CHANGED_EVENT));
};

export const subscribeOwnProfileNativePush = (
  onProfile: (profile: NativeOwnProfile) => void
): (() => void) => {
  let cancelled = false;
  let unlisten: (() => void) | undefined;
  void listen<unknown>(MATRIX_OWN_PROFILE_CHANGED_EVENT, (event) => {
    const profile = parseOwnProfilePush(event.payload);
    if (profile) onProfile(profile);
  }).then((handle) => {
    if (!handle) return;
    if (cancelled) {
      void handle();
      return;
    }
    unlisten = () => {
      void handle();
    };
  });
  return () => {
    cancelled = true;
    unlisten?.();
  };
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
