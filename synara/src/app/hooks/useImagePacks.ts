import { Room } from 'matrix-js-sdk';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { AccountDataEvent } from '../../types/matrix/accountData';
import { StateEvent } from '../../types/matrix/room';
import {
  getGlobalImagePacks,
  getRoomImagePack,
  getRoomImagePacks,
  getUserImagePack,
  ImagePack,
  ImageUsage,
} from '../plugins/custom-emoji';
import {
  getGlobalImagePacksNative,
  getRoomImagePacksNative,
  getUserImagePackNative,
} from '../features/room/nativeImagePack';
import { isSynaraDesktop, listen } from '../utils/desktop';
import { useMatrixClient } from './useMatrixClient';
import { useAccountDataCallback } from './useAccountDataCallback';
import { useStateEventCallback } from './useStateEventCallback';

/** Must match Rust IMAGE_PACKS_UPDATED_EVENT. Signal only; re-snapshot via get*Native. */
const IMAGE_PACKS_UPDATED_EVENT = 'matrix-image-packs-updated';

/**
 * V-SEND.R-PACK-READ subscribe: when native session is active, listen for
 * pack-change signals and bump a refresh token so snapshot effects re-run.
 * Web fallback keeps useAccountDataCallback / useStateEventCallback.
 */
function useNativeImagePackRefreshToken(nativeActive: boolean): number {
  const [token, setToken] = useState(0);

  useEffect(() => {
    if (!nativeActive || !isSynaraDesktop()) return;
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
 * matrix-image-packs-updated. JS account-data/state callbacks stay for
 * non-native (web) sessions only.
 */
export const useUserImagePack = (): ImagePack | undefined => {
  const mx = useMatrixClient();
  const [userPack, setUserPack] = useState<ImagePack | undefined>(() =>
    isSynaraDesktop() ? undefined : getUserImagePack(mx)
  );
  const [nativeActive, setNativeActive] = useState(false);
  const refreshToken = useNativeImagePackRefreshToken(nativeActive);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const result = await getUserImagePackNative();
      if (cancelled) return;
      if (result === 'legacy') {
        setNativeActive(false);
        setUserPack(getUserImagePack(mx));
        return;
      }
      setNativeActive(true);
      setUserPack(result);
    })().catch(() => {
      if (!cancelled && isSynaraDesktop()) {
        // Fail-closed: leave empty rather than silent JS fallthrough on desktop.
        setNativeActive(true);
        setUserPack(undefined);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [mx, refreshToken]);

  useAccountDataCallback(
    mx,
    useCallback(
      (mEvent) => {
        if (nativeActive) return;
        if (mEvent.getType() === AccountDataEvent.PoniesUserEmotes) {
          setUserPack(getUserImagePack(mx));
        }
      },
      [mx, nativeActive]
    )
  );

  return userPack;
};

export const useGlobalImagePacks = (): ImagePack[] => {
  const mx = useMatrixClient();
  const [globalPacks, setGlobalPacks] = useState<ImagePack[]>(() =>
    isSynaraDesktop() ? [] : getGlobalImagePacks(mx)
  );
  const [nativeActive, setNativeActive] = useState(false);
  const refreshToken = useNativeImagePackRefreshToken(nativeActive);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const result = await getGlobalImagePacksNative();
      if (cancelled) return;
      if (result === 'legacy') {
        setNativeActive(false);
        setGlobalPacks(getGlobalImagePacks(mx));
        return;
      }
      setNativeActive(true);
      setGlobalPacks(result);
    })().catch(() => {
      if (!cancelled && isSynaraDesktop()) {
        setNativeActive(true);
        setGlobalPacks([]);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [mx, refreshToken]);

  useAccountDataCallback(
    mx,
    useCallback(
      (mEvent) => {
        if (nativeActive) return;
        if (mEvent.getType() === AccountDataEvent.PoniesEmoteRooms) {
          setGlobalPacks(getGlobalImagePacks(mx));
        }
      },
      [mx, nativeActive]
    )
  );

  useStateEventCallback(
    mx,
    useCallback(
      (mEvent) => {
        if (nativeActive) return;
        const eventType = mEvent.getType();
        const roomId = mEvent.getRoomId();
        const stateKey = mEvent.getStateKey();
        if (eventType === StateEvent.PoniesRoomEmotes && roomId && typeof stateKey === 'string') {
          const global = !!globalPacks.find(
            (pack) =>
              pack.address && pack.address.roomId === roomId && pack.address.stateKey === stateKey
          );
          if (global) {
            setGlobalPacks(getGlobalImagePacks(mx));
          }
        }
      },
      [mx, globalPacks, nativeActive]
    )
  );

  return globalPacks;
};

export const useRoomImagePack = (room: Room, stateKey: string): ImagePack | undefined => {
  const mx = useMatrixClient();
  const [roomPack, setRoomPack] = useState<ImagePack | undefined>(() =>
    isSynaraDesktop() ? undefined : getRoomImagePack(room, stateKey)
  );
  const [nativeActive, setNativeActive] = useState(false);
  const refreshToken = useNativeImagePackRefreshToken(nativeActive);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const result = await getRoomImagePacksNative(room.roomId);
      if (cancelled) return;
      if (result === 'legacy') {
        setNativeActive(false);
        setRoomPack(getRoomImagePack(room, stateKey));
        return;
      }
      setNativeActive(true);
      setRoomPack(result.find((p) => p.address?.stateKey === stateKey));
    })().catch(() => {
      if (!cancelled && isSynaraDesktop()) {
        setNativeActive(true);
        setRoomPack(undefined);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [mx, room, stateKey, refreshToken]);

  useStateEventCallback(
    mx,
    useCallback(
      (mEvent) => {
        if (nativeActive) return;
        if (
          mEvent.getRoomId() === room.roomId &&
          mEvent.getType() === StateEvent.PoniesRoomEmotes &&
          mEvent.getStateKey() === stateKey
        ) {
          setRoomPack(getRoomImagePack(room, stateKey));
        }
      },
      [room, stateKey, nativeActive]
    )
  );

  return roomPack;
};

export const useRoomImagePacks = (room: Room): ImagePack[] => {
  const mx = useMatrixClient();
  const [roomPacks, setRoomPacks] = useState<ImagePack[]>(() =>
    isSynaraDesktop() ? [] : getRoomImagePacks(room)
  );
  const [nativeActive, setNativeActive] = useState(false);
  const refreshToken = useNativeImagePackRefreshToken(nativeActive);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const result = await getRoomImagePacksNative(room.roomId);
      if (cancelled) return;
      if (result === 'legacy') {
        setNativeActive(false);
        setRoomPacks(getRoomImagePacks(room));
        return;
      }
      setNativeActive(true);
      setRoomPacks(result);
    })().catch(() => {
      if (!cancelled && isSynaraDesktop()) {
        setNativeActive(true);
        setRoomPacks([]);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [mx, room, refreshToken]);

  useStateEventCallback(
    mx,
    useCallback(
      (mEvent) => {
        if (nativeActive) return;
        if (
          mEvent.getRoomId() === room.roomId &&
          mEvent.getType() === StateEvent.PoniesRoomEmotes
        ) {
          setRoomPacks(getRoomImagePacks(room));
        }
      },
      [room, nativeActive]
    )
  );

  return roomPacks;
};

export const useRoomsImagePacks = (roomIds: string[]) => {
  const mx = useMatrixClient();
  const roomKey = roomIds.join(',');
  const [roomPacks, setRoomPacks] = useState<ImagePack[]>(() => {
    if (isSynaraDesktop()) return [];
    return roomIds.flatMap((id) => {
      const room = mx.getRoom(id);
      return room ? getRoomImagePacks(room) : [];
    });
  });
  const [nativeActive, setNativeActive] = useState(false);
  const refreshToken = useNativeImagePackRefreshToken(nativeActive);

  const loadLegacyPacks = useCallback(() => {
    return roomIds.flatMap((id) => {
      const room = mx.getRoom(id);
      return room ? getRoomImagePacks(room) : [];
    });
  }, [mx, roomIds]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      if (!isSynaraDesktop()) {
        setNativeActive(false);
        setRoomPacks(loadLegacyPacks());
        return;
      }
      try {
        const all: ImagePack[] = [];
        for (const roomId of roomIds) {
          const result = await getRoomImagePacksNative(roomId);
          if (result === 'legacy') {
            setNativeActive(false);
            setRoomPacks(loadLegacyPacks());
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
  }, [mx, roomKey, roomIds, refreshToken, loadLegacyPacks]);

  useStateEventCallback(
    mx,
    useCallback(
      (mEvent) => {
        if (nativeActive) return;
        if (
          roomIds.includes(mEvent.getRoomId() ?? '') &&
          mEvent.getType() === StateEvent.PoniesRoomEmotes
        ) {
          setRoomPacks(loadLegacyPacks());
        }
      },
      [roomIds, nativeActive, loadLegacyPacks]
    )
  );

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
