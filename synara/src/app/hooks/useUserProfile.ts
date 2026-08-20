import { useCallback, useEffect, useState } from 'react';
import {
  getOwnProfileNative,
  OWN_PROFILE_CHANGED_EVENT,
} from '../features/settings/account/nativeProfile';
import { UserEvent } from '../utils/roomEvents';
import { useMatrixClient } from './useMatrixClient';

export type UserProfile = {
  avatarUrl?: string;
  displayName?: string;
};

type UserEventedReading = {
  avatarUrl?: string;
  displayName?: string;
  on(event: string, listener: (...args: any[]) => void): void;
  removeListener(event: string, listener: (...args: any[]) => void): void;
};

const isOwnUser = (mx: { getUserId(): string | null }, userId: string): boolean =>
  mx.getUserId() === userId;

export const useUserProfile = (userId: string): UserProfile => {
  const mx = useMatrixClient();

  const [profile, setProfile] = useState<UserProfile>(() => {
    const user = mx.getUser(userId);
    return {
      avatarUrl: user?.avatarUrl,
      displayName: user?.displayName,
    };
  });
  const [refreshGeneration, setRefreshGeneration] = useState(0);
  const refresh = useCallback(() => setRefreshGeneration((generation) => generation + 1), []);

  useEffect(() => {
    const user = mx.getUser(userId) as unknown as UserEventedReading | null;
    let cancelled = false;

    const onAvatarChange = (event: unknown, myUser: UserEventedReading) => {
      setProfile((cp) => ({
        ...cp,
        avatarUrl: myUser.avatarUrl,
      }));
    };
    const onDisplayNameChange = (event: unknown, myUser: UserEventedReading) => {
      setProfile((cp) => ({
        ...cp,
        displayName: myUser.displayName,
      }));
    };

    const load = async () => {
      if (isOwnUser(mx, userId)) {
        try {
          const native = await getOwnProfileNative();
          if (cancelled) return;
          if (native !== 'legacy') {
            setProfile({
              avatarUrl: native.avatarUrl,
              displayName: native.displayName,
            });
            return;
          }
        } catch {
          return;
        }
      }
      try {
        const info = await mx.getProfileInfo(userId);
        if (cancelled) return;
        setProfile({
          avatarUrl: info.avatar_url,
          displayName: info.displayname,
        });
      } catch {
        // Keep the last known profile. Native own-profile already failed above.
      }
    };

    void load();

    user?.on(UserEvent.AvatarUrl, onAvatarChange);
    user?.on(UserEvent.DisplayName, onDisplayNameChange);
    return () => {
      cancelled = true;
      user?.removeListener(UserEvent.AvatarUrl, onAvatarChange);
      user?.removeListener(UserEvent.DisplayName, onDisplayNameChange);
    };
  }, [mx, userId, refreshGeneration]);

  useEffect(() => {
    if (!isOwnUser(mx, userId)) return undefined;
    window.addEventListener(OWN_PROFILE_CHANGED_EVENT, refresh);
    return () => window.removeEventListener(OWN_PROFILE_CHANGED_EVENT, refresh);
  }, [mx, refresh, userId]);

  return profile;
};
