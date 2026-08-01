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
  fetchNativeGlobalImagePacks,
  fetchNativeRoomImagePacks,
  fetchNativeUserImagePack,
} from '../plugins/custom-emoji/nativeImagePacks';
import { useMatrixClient } from './useMatrixClient';
import { useAccountDataCallback } from './useAccountDataCallback';
import { useStateEventCallback } from './useStateEventCallback';
import { isSynaraDesktop } from '../utils/desktop';

const NATIVE_PACK_POLL_MS = 1_000;

/**
 * V-SEND.R-PACK-READ: on desktop the pack-read projection is owned by the Rust
 * host (read-only IPC). These hooks invoke the native commands and fall back to
 * the legacy `matrix-js-sdk` read path only on non-native web sessions.
 */

export const useUserImagePack = (): ImagePack | undefined => {
  const mx = useMatrixClient();
  const [userPack, setUserPack] = useState(() => getUserImagePack(mx));

  useEffect(() => {
    if (!isSynaraDesktop()) return undefined;
    let disposed = false;
    let inFlight = false;
    const refresh = async () => {
      if (inFlight) return;
      inFlight = true;
      try {
        const pack = await fetchNativeUserImagePack();
        if (!disposed) setUserPack(pack);
      } catch {
        // Fail-closed: keep the last known projection during transient failures.
      } finally {
        inFlight = false;
      }
    };
    void refresh();
    const pollId = window.setInterval(() => void refresh(), NATIVE_PACK_POLL_MS);
    return () => {
      disposed = true;
      window.clearInterval(pollId);
    };
  }, []);

  useAccountDataCallback(
    mx,
    useCallback(
      (mEvent) => {
        if (mEvent.getType() === AccountDataEvent.PoniesUserEmotes) {
          setUserPack(getUserImagePack(mx));
        }
      },
      [mx]
    )
  );

  return userPack;
};

export const useGlobalImagePacks = (): ImagePack[] => {
  const mx = useMatrixClient();
  const [globalPacks, setGlobalPacks] = useState(() => getGlobalImagePacks(mx));

  useEffect(() => {
    if (!isSynaraDesktop()) return undefined;
    let disposed = false;
    let inFlight = false;
    const refresh = async () => {
      if (inFlight) return;
      inFlight = true;
      try {
        const packs = await fetchNativeGlobalImagePacks();
        if (!disposed) setGlobalPacks(packs);
      } catch {
        // Fail-closed: keep the last known projection during transient failures.
      } finally {
        inFlight = false;
      }
    };
    void refresh();
    const pollId = window.setInterval(() => void refresh(), NATIVE_PACK_POLL_MS);
    return () => {
      disposed = true;
      window.clearInterval(pollId);
    };
  }, []);

  useAccountDataCallback(
    mx,
    useCallback(
      (mEvent) => {
        if (mEvent.getType() === AccountDataEvent.PoniesEmoteRooms) {
          setGlobalPacks(getGlobalImagePacks(mx));
        }
      },
      [mx]
    )
  );

  useStateEventCallback(
    mx,
    useCallback(
      (mEvent) => {
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
      [mx, globalPacks]
    )
  );

  return globalPacks;
};

export const useRoomImagePack = (room: Room, stateKey: string): ImagePack | undefined => {
  const mx = useMatrixClient();
  const [roomPack, setRoomPack] = useState(() => getRoomImagePack(room, stateKey));

  useEffect(() => {
    if (!isSynaraDesktop()) return undefined;
    let disposed = false;
    let inFlight = false;
    const refresh = async () => {
      if (inFlight) return;
      inFlight = true;
      try {
        const packs = await fetchNativeRoomImagePacks(room.roomId);
        if (!disposed) {
          setRoomPack(packs.find((pack) => pack.address?.stateKey === stateKey));
        }
      } catch {
        // Fail-closed: keep the last known projection during transient failures.
      } finally {
        inFlight = false;
      }
    };
    void refresh();
    const pollId = window.setInterval(() => void refresh(), NATIVE_PACK_POLL_MS);
    return () => {
      disposed = true;
      window.clearInterval(pollId);
    };
  }, [room.roomId, stateKey]);

  useStateEventCallback(
    mx,
    useCallback(
      (mEvent) => {
        if (
          mEvent.getRoomId() === room.roomId &&
          mEvent.getType() === StateEvent.PoniesRoomEmotes &&
          mEvent.getStateKey() === stateKey
        ) {
          setRoomPack(getRoomImagePack(room, stateKey));
        }
      },
      [room, stateKey]
    )
  );

  return roomPack;
};

export const useRoomImagePacks = (room: Room): ImagePack[] => {
  const mx = useMatrixClient();
  const [roomPacks, setRoomPacks] = useState(() => getRoomImagePacks(room));

  useEffect(() => {
    if (!isSynaraDesktop()) return undefined;
    let disposed = false;
    let inFlight = false;
    const refresh = async () => {
      if (inFlight) return;
      inFlight = true;
      try {
        const packs = await fetchNativeRoomImagePacks(room.roomId);
        if (!disposed) setRoomPacks(packs);
      } catch {
        // Fail-closed: keep the last known projection during transient failures.
      } finally {
        inFlight = false;
      }
    };
    void refresh();
    const pollId = window.setInterval(() => void refresh(), NATIVE_PACK_POLL_MS);
    return () => {
      disposed = true;
      window.clearInterval(pollId);
    };
  }, [room.roomId]);

  useStateEventCallback(
    mx,
    useCallback(
      (mEvent) => {
        if (
          mEvent.getRoomId() === room.roomId &&
          mEvent.getType() === StateEvent.PoniesRoomEmotes
        ) {
          setRoomPacks(getRoomImagePacks(room));
        }
      },
      [room]
    )
  );

  return roomPacks;
};

export const useRoomsImagePacks = (rooms: Room[]) => {
  const mx = useMatrixClient();
  const [roomPacks, setRoomPacks] = useState(() => rooms.flatMap(getRoomImagePacks));

  useEffect(() => {
    if (!isSynaraDesktop()) return undefined;
    let disposed = false;
    let inFlight = false;
    const refresh = async () => {
      if (inFlight) return;
      inFlight = true;
      try {
        const results = await Promise.all(rooms.map((room) => fetchNativeRoomImagePacks(room.roomId)));
        if (!disposed) setRoomPacks(results.flat());
      } catch {
        // Fail-closed: keep the last known projection during transient failures.
      } finally {
        inFlight = false;
      }
    };
    void refresh();
    const pollId = window.setInterval(() => void refresh(), NATIVE_PACK_POLL_MS);
    return () => {
      disposed = true;
      window.clearInterval(pollId);
    };
  }, [rooms]);

  useStateEventCallback(
    mx,
    useCallback(
      (mEvent) => {
        if (
          rooms.find((room) => room.roomId === mEvent.getRoomId()) &&
          mEvent.getType() === StateEvent.PoniesRoomEmotes
        ) {
          setRoomPacks(rooms.flatMap(getRoomImagePacks));
        }
      },
      [rooms]
    )
  );

  return roomPacks;
};

export const useRelevantImagePacks = (usage: ImageUsage, rooms: Room[]): ImagePack[] => {
  const userPack = useUserImagePack();
  const globalPacks = useGlobalImagePacks();
  const roomsPacks = useRoomsImagePacks(rooms);

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
