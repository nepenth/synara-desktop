import { useCallback, useEffect, useState } from 'react';
import {
  getOwnProfileNative,
  OWN_PROFILE_CHANGED_EVENT,
  subscribeOwnProfileNativePush,
} from '../features/settings/account/nativeProfile';
import { getMyUserId } from '../state/nativeIdentity';

import { getNativeProfileInfo } from '../native/nativeCommands';
export type UserProfile = {
  avatarUrl?: string;
  displayName?: string;
};

const isOwnUser = (userId: string): boolean => getMyUserId() === userId;

export const useUserProfile = (userId: string): UserProfile => {
  const [profile, setProfile] = useState<UserProfile>({});
  const [refreshGeneration, setRefreshGeneration] = useState(0);
  const refresh = useCallback(() => setRefreshGeneration((generation) => generation + 1), []);

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      if (isOwnUser(userId)) {
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
        const info = await getNativeProfileInfo(userId);
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

    return () => {
      cancelled = true;
    };
  }, [userId, refreshGeneration]);

  useEffect(() => {
    if (!isOwnUser(userId)) return undefined;
    window.addEventListener(OWN_PROFILE_CHANGED_EVENT, refresh);
    const unsubscribe = subscribeOwnProfileNativePush((native) => {
      if (native.userId !== userId) return;
      setProfile({
        avatarUrl: native.avatarUrl,
        displayName: native.displayName,
      });
    });
    return () => {
      window.removeEventListener(OWN_PROFILE_CHANGED_EVENT, refresh);
      unsubscribe();
    };
  }, [refresh, userId]);

  return profile;
};
