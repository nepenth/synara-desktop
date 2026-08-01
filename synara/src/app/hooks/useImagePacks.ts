import { useEffect, useMemo, useState } from 'react';
import { ImagePack, ImageUsage } from '../plugins/custom-emoji';
import {
  getGlobalImagePacksNative,
  getRoomImagePacksNative,
  getUserImagePackNative,
} from '../features/room/nativeImagePack';
import { listen } from '../utils/desktop';

type RoomWithId = {
  roomId: string;
};

/** Must match Rust IMAGE_PACKS_UPDATED_EVENT. Signal only; re-snapshot via get*Native. */
const IMAGE_PACKS_UPDATED_EVENT = 'matrix-image-packs-updated';

/**
 * V-SEND.R-PACK-READ subscribe: when native session is active, listen for
 * pack-change signals and bump a refresh token so snapshot effects re-run.
 */
function useNativeImagePackRefreshToken(nativeActive: boolean): number {
  const [token, setToken] = useState(0);

  useEffect(() => {
    if (!nativeActive) return;
    let cancelled = false;
    let unlisten: (() => void | Promise<void>) | undefined;

    (async () => {
      const handle = await listen(IMAGE_PACKS_UPDATED_EVENT, () => {
        setToken((n) => n + 1);
      });
      if (cancelled) {
        await handle?.();
        return;
      }
      unlisten = handle;
    })();

    return () => {
      cancelled = true;
      void unlisten?.();
    };
  }, [nativeActive]);

  return token;
}

/**
 * V-SEND.R-PACK-READ: when desktop native session is live, pack reads use
 * matrix_get_*_image_packs (fail-closed) with live subscribe refresh via
 * matrix-image-packs-updated. There is no JS account-data/state fallback.
 */
export const useUserImagePack = (): ImagePack | undefined => {
  const [userPack, setUserPack] = useState<ImagePack | undefined>();
  const [nativeActive, setNativeActive] = useState(false);
  const refreshToken = useNativeImagePackRefreshToken(nativeActive);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const result = await getUserImagePackNative();
      if (cancelled) return;
      setNativeActive(true);
      setUserPack(result === 'legacy' ? undefined : result);
    })().catch(() => {
      if (!cancelled) {
        // Fail-closed: leave empty rather than falling through to JS reads.
        setNativeActive(true);
        setUserPack(undefined);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [refreshToken]);

  return userPack;
};

export const useGlobalImagePacks = (): ImagePack[] => {
  const [globalPacks, setGlobalPacks] = useState<ImagePack[]>([]);
  const [nativeActive, setNativeActive] = useState(false);
  const refreshToken = useNativeImagePackRefreshToken(nativeActive);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const result = await getGlobalImagePacksNative();
      if (cancelled) return;
      setNativeActive(true);
      setGlobalPacks(result === 'legacy' ? [] : result);
    })().catch(() => {
      if (!cancelled) {
        setNativeActive(true);
        setGlobalPacks([]);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [refreshToken]);

  return globalPacks;
};

export const useRoomImagePack = (room: RoomWithId, stateKey: string): ImagePack | undefined => {
  const [roomPack, setRoomPack] = useState<ImagePack | undefined>();
  const [nativeActive, setNativeActive] = useState(false);
  const refreshToken = useNativeImagePackRefreshToken(nativeActive);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const result = await getRoomImagePacksNative(room.roomId);
      if (cancelled) return;
      setNativeActive(true);
      setRoomPack(
        result === 'legacy' ? undefined : result.find((p) => p.address?.stateKey === stateKey)
      );
    })().catch(() => {
      if (!cancelled) {
        setNativeActive(true);
        setRoomPack(undefined);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [room.roomId, stateKey, refreshToken]);

  return roomPack;
};

export const useRoomImagePacks = (room: RoomWithId): ImagePack[] => {
  const [roomPacks, setRoomPacks] = useState<ImagePack[]>([]);
  const [nativeActive, setNativeActive] = useState(false);
  const refreshToken = useNativeImagePackRefreshToken(nativeActive);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const result = await getRoomImagePacksNative(room.roomId);
      if (cancelled) return;
      setNativeActive(true);
      setRoomPacks(result === 'legacy' ? [] : result);
    })().catch(() => {
      if (!cancelled) {
        setNativeActive(true);
        setRoomPacks([]);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [room.roomId, refreshToken]);

  return roomPacks;
};

export const useRoomsImagePacks = (roomIds: string[]) => {
  const roomKey = roomIds.join(',');
  const [roomPacks, setRoomPacks] = useState<ImagePack[]>([]);
  const [nativeActive, setNativeActive] = useState(false);
  const refreshToken = useNativeImagePackRefreshToken(nativeActive);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const all: ImagePack[] = [];
        for (const roomId of roomIds) {
          const result = await getRoomImagePacksNative(roomId);
          if (result === 'legacy') {
            if (!cancelled) {
              setNativeActive(true);
              setRoomPacks([]);
            }
            return;
          }
          all.push(...result);
        }
        if (cancelled) return;
        setNativeActive(true);
        setRoomPacks(all);
      } catch {
        if (!cancelled) {
          setNativeActive(true);
          setRoomPacks([]);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [roomKey, roomIds, refreshToken]);

  return roomPacks;
};

/** roomIds: current room + parent space ids (from useImagePackRooms). */
export const useRelevantImagePacks = (usage: ImageUsage, roomIds: string[]): ImagePack[] => {
  const userPack = useUserImagePack();
  const globalPacks = useGlobalImagePacks();
  const roomsPacks = useRoomsImagePacks(roomIds);

  const relevantPacks = useMemo(() => {
    const packs = userPack ? [userPack] : [];
    const globalPackIds = new Set(globalPacks.map((pack) => pack.id));

    const relPacks = packs.concat(
      globalPacks,
      roomsPacks.filter((pack) => !globalPackIds.has(pack.id))
    );

    return relPacks.filter((pack) => pack.getImages(usage).length > 0);
  }, [userPack, globalPacks, roomsPacks, usage]);

  return relevantPacks;
};
