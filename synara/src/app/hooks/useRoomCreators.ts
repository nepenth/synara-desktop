import { MatrixClient, MatrixEvent, Room } from 'matrix-js-sdk';
import { useEffect, useMemo, useState } from 'react';
import { useStateEvent } from './useStateEvent';
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

export const getRoomCreatorsForRoomId = (mx: MatrixClient, roomId: string): Set<string> => {
  const room = mx.getRoom(roomId);
  if (!room) return new Set();

  const createEvent = getStateEvent(room, StateEvent.RoomCreate);
  if (!createEvent) return new Set();

  return getRoomCreators(createEvent);
};
