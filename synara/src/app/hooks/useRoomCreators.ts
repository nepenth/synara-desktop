import { MatrixClient, MatrixEvent, Room } from 'matrix-js-sdk';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useStateEvent } from './useStateEvent';
import { useStateEventCallback } from './useStateEventCallback';
import { useMatrixClient } from './useMatrixClient';
import { IRoomCreateContent, StateEvent } from '../../types/matrix/room';
import { creatorsSupported } from '../utils/matrix';
import { getStateEvent } from '../utils/room';
import { isNativeMatrixSession } from '../features/verification/nativeVerification';
import { readRoomCreatorsWithNativeOwner } from './nativeRoomCreatorsOwner';

export const getRoomCreators = (createEvent: MatrixEvent): Set<string> => {
  const createContent = createEvent.getContent<IRoomCreateContent>();

  const creators: Set<string> = new Set();

  if (!creatorsSupported(createContent.room_version)) return creators;

  if (createEvent.event.sender) {
    creators.add(createEvent.event.sender);
  }

  if ('additional_creators' in createContent && Array.isArray(createContent.additional_creators)) {
    createContent.additional_creators.forEach((creator) => {
      if (typeof creator === 'string') {
        creators.add(creator);
      }
    });
  }

  return creators;
};

export const useRoomCreators = (room: Room): Set<string> => {
  const nativeSession = isNativeMatrixSession();
  const createEvent = useStateEvent(room, StateEvent.RoomCreate, '', !nativeSession);
  const [nativeState, setNativeState] = useState<
    | { roomId: string; status: 'idle' | 'loading' }
    | { roomId: string; status: 'ready'; creators: Set<string> }
    | { roomId: string; status: 'error'; error: Error }
  >({ roomId: room.roomId, status: 'idle' });

  const legacyCreators = useMemo(
    () => (createEvent ? getRoomCreators(createEvent) : new Set<string>()),
    [createEvent]
  );

  useEffect(() => {
    if (!nativeSession) return undefined;

    let disposed = false;
    setNativeState({ roomId: room.roomId, status: 'loading' });
    void readRoomCreatorsWithNativeOwner(room.roomId, true)
      .then((snapshot) => {
        if (!disposed && snapshot) {
          setNativeState({
            roomId: room.roomId,
            status: 'ready',
            creators: new Set(snapshot.creators),
          });
        }
      })
      .catch((error: unknown) => {
        if (!disposed) {
          setNativeState({
            roomId: room.roomId,
            status: 'error',
            error:
              error instanceof Error
                ? error
                : new Error('Native Matrix room creators are unavailable.'),
          });
        }
      });

    return () => {
      disposed = true;
    };
  }, [nativeSession, room.roomId]);

  if (!nativeSession) return legacyCreators;
  if (nativeState.status === 'error') throw nativeState.error;
  if (nativeState.roomId !== room.roomId || nativeState.status !== 'ready') {
    return new Set<string>();
  }
  return nativeState.creators;
};

/**
 * Read creators for a set of rooms without reopening the JS state-event path
 * for native sessions. Loading and unavailable native results return empty
 * creator sets so permission checks remain fail-closed.
 */
export const useRoomsCreators = (rooms: Room[]): Map<string, Set<string>> => {
  const mx = useMatrixClient();
  const nativeSession = isNativeMatrixSession();
  const roomIdsKey = rooms.map((room) => room.roomId).join('\u0000');
  const getLegacyCreators = useCallback(() => {
    const roomToCreators = new Map<string, Set<string>>();
    rooms.forEach((room) => {
      const createEvent = getStateEvent(room, StateEvent.RoomCreate);
      roomToCreators.set(room.roomId, createEvent ? getRoomCreators(createEvent) : new Set());
    });
    return roomToCreators;
  }, [rooms]);

  const [roomToCreators, setRoomToCreators] = useState(() =>
    nativeSession ? new Map<string, Set<string>>() : getLegacyCreators()
  );
  const [nativeState, setNativeState] = useState<
    | { roomIdsKey: string; status: 'idle' | 'loading' }
    | { roomIdsKey: string; status: 'ready'; values: Map<string, Set<string>> }
    | { roomIdsKey: string; status: 'error'; error: Error }
  >({ roomIdsKey, status: 'idle' });

  useEffect(() => {
    if (!nativeSession) return undefined;

    let disposed = false;
    setNativeState({ roomIdsKey, status: 'loading' });
    void Promise.all(rooms.map((room) => readRoomCreatorsWithNativeOwner(room.roomId, true)))
      .then((snapshots) => {
        if (disposed) return;
        const values = new Map<string, Set<string>>();
        rooms.forEach((room, index) => {
          const snapshot = snapshots[index];
          if (snapshot) values.set(room.roomId, new Set(snapshot.creators));
        });
        setNativeState({ roomIdsKey, status: 'ready', values });
      })
      .catch((error: unknown) => {
        if (!disposed) {
          setNativeState({
            roomIdsKey,
            status: 'error',
            error:
              error instanceof Error
                ? error
                : new Error('Native Matrix room creators are unavailable.'),
          });
        }
      });

    return () => {
      disposed = true;
    };
  }, [nativeSession, roomIdsKey, rooms]);

  useStateEventCallback(
    mx,
    useCallback(
      (event) => {
        if (nativeSession) return;
        const roomId = event.getRoomId();
        if (
          roomId &&
          event.getType() === StateEvent.RoomCreate &&
          event.getStateKey() === '' &&
          rooms.some((room) => room.roomId === roomId)
        ) {
          setRoomToCreators(getLegacyCreators());
        }
      },
      [getLegacyCreators, nativeSession, rooms]
    )
  );

  if (nativeSession) {
    if (nativeState.status === 'error') throw nativeState.error;
    if (nativeState.roomIdsKey !== roomIdsKey || nativeState.status !== 'ready') {
      return new Map(rooms.map((room) => [room.roomId, new Set<string>()]));
    }
    return nativeState.values;
  }

  return roomToCreators;
};

export const getRoomCreatorsForRoomId = (mx: MatrixClient, roomId: string): Set<string> => {
  if (isNativeMatrixSession()) return new Set();

  const room = mx.getRoom(roomId);
  if (!room) return new Set();

  const createEvent = getStateEvent(room, StateEvent.RoomCreate);
  if (!createEvent) return new Set();

  return getRoomCreators(createEvent);
};
