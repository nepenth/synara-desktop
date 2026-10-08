import { useEffect, useState } from 'react';
import { readRoomMembersWithNativeOwner } from './nativeRoomMembersOwner';
import type { RoomMember as NativeRoomMember } from '../features/matrix-dto/member';
import type { JsRoomMemberReading } from '../utils/roomEvents';

export type RoomMemberListItem = JsRoomMemberReading | NativeRoomMember;

const NO_MEMBERS: JsRoomMemberReading[] = [];

export function useRoomMembers(roomId: string): JsRoomMemberReading[];
export function useRoomMembers(
  roomId: string,
  nativeSession: boolean
): RoomMemberListItem[] | null | undefined;

export function useRoomMembers(
  roomId: string,
  nativeSession = false
): RoomMemberListItem[] | null | undefined {
  const [nativeMembers, setNativeMembers] = useState<NativeRoomMember[] | null | undefined>(null);

  useEffect(() => {
    if (nativeSession) {
      let disposed = false;
      setNativeMembers(null);
      void readRoomMembersWithNativeOwner(roomId, true)
        .then((nextMembers) => {
          if (!disposed) setNativeMembers(nextMembers ?? undefined);
        })
        .catch(() => {
          // Native ownership is fail-closed. Expose an unavailable state instead
          // of presenting a failed request as an authoritative empty room.
          if (!disposed) setNativeMembers(undefined);
        });

      return () => {
        disposed = true;
      };
    }

    // Members come only from the native owner; a non-native session has none.
    return undefined;
  }, [roomId, nativeSession]);

  return nativeSession ? nativeMembers : NO_MEMBERS;
}
