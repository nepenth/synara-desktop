import { useEffect, useState } from 'react';
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

export const useUserProfile = (userId: string): UserProfile => {
  const mx = useMatrixClient();

  const [profile, setProfile] = useState<UserProfile>(() => {
    const user = mx.getUser(userId);
    return {
      avatarUrl: user?.avatarUrl,
      displayName: user?.displayName,
    };
  });

  useEffect(() => {
    const user = mx.getUser(userId) as unknown as UserEventedReading | null;

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

    mx.getProfileInfo(userId).then((info) =>
      setProfile({
        avatarUrl: info.avatar_url,
        displayName: info.displayname,
      })
    );

    user?.on(UserEvent.AvatarUrl, onAvatarChange);
    user?.on(UserEvent.DisplayName, onDisplayNameChange);
    return () => {
      user?.removeListener(UserEvent.AvatarUrl, onAvatarChange);
      user?.removeListener(UserEvent.DisplayName, onDisplayNameChange);
    };
  }, [mx, userId]);

  return profile;
};
