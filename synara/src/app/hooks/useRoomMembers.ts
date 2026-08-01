import {
  MatrixClient,
  MatrixEvent,
  RoomMember as MatrixRoomMember,
  RoomMemberEvent,
} from 'matrix-js-sdk';
import { useEffect, useState } from 'react';
import { readRoomMembersWithNativeOwner } from './nativeRoomMembersOwner';
import type { RoomMember as NativeRoomMember } from '../features/matrix-dto/member';

export type RoomMemberListItem = MatrixRoomMember | NativeRoomMember;

export function useRoomMembers(mx: MatrixClient, roomId: string): MatrixRoomMember[];
export function useRoomMembers(
  mx: MatrixClient,
  roomId: string,
  nativeSession: boolean
): RoomMemberListItem[] | null;

export function useRoomMembers(
  mx: MatrixClient,
  roomId: string,
  nativeSession = false
): RoomMemberListItem[] | null {
  const [members, setMembers] = useState<MatrixRoomMember[]>([]);
  const [nativeMembers, setNativeMembers] = useState<NativeRoomMember[] | null>(null);

  useEffect(() => {
    if (nativeSession) {
      let disposed = false;
      setNativeMembers([]);
      void readRoomMembersWithNativeOwner(roomId, true)
        .then((nextMembers) => {
          if (!disposed && nextMembers) setNativeMembers(nextMembers);
        })
        .catch(() => {
          // Native ownership is fail-closed. Keep the list empty instead of
          // falling through to mx.getRoom().getMembers().
          if (!disposed) setNativeMembers([]);
        });

      return () => {
        disposed = true;
      };
    }

    const room = mx.getRoom(roomId);
    let loadingMembers = true;
    let disposed = false;

    const updateMemberList = (event?: MatrixEvent) => {
      if (!room || disposed || (event && event.getRoomId() !== roomId)) return;
      if (loadingMembers) return;
      setMembers(room.getMembers());
    };

    if (room) {
      setMembers(room.getMembers());
      room.loadMembersIfNeeded().then(() => {
        loadingMembers = false;
        if (disposed) return;
        updateMemberList();
      });
    }

    mx.on(RoomMemberEvent.Membership, updateMemberList);
    mx.on(RoomMemberEvent.PowerLevel, updateMemberList);
    return () => {
      disposed = true;
      mx.removeListener(RoomMemberEvent.Membership, updateMemberList);
      mx.removeListener(RoomMemberEvent.PowerLevel, updateMemberList);
    };
  }, [mx, roomId, nativeSession]);

  return nativeSession ? nativeMembers : members;
}
