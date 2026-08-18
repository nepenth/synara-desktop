import { useEffect, useState } from 'react';
import { readRoomMembersWithNativeOwner } from './nativeRoomMembersOwner';
import type { RoomMember as NativeRoomMember } from '../features/matrix-dto/member';
import type { MatrixClientReading, MatrixEventReading } from '../utils/room';
import { RoomMemberEvent, type JsRoomMemberReading } from '../utils/roomEvents';

export type RoomMemberListItem = JsRoomMemberReading | NativeRoomMember;

type EventedRoomMembersReading = {
  getRoomId(): string;
  getMembers(): JsRoomMemberReading[];
  loadMembersIfNeeded(): Promise<unknown>;
  on(event: string, listener: (...args: any[]) => void): void;
  removeListener(event: string, listener: (...args: any[]) => void): void;
};

export function useRoomMembers(mx: MatrixClientReading, roomId: string): JsRoomMemberReading[];
export function useRoomMembers(
  mx: MatrixClientReading,
  roomId: string,
  nativeSession: boolean
): RoomMemberListItem[] | null | undefined;

export function useRoomMembers(
  mx: MatrixClientReading,
  roomId: string,
  nativeSession = false
): RoomMemberListItem[] | null | undefined {
  const [members, setMembers] = useState<JsRoomMemberReading[]>([]);
  const [nativeMembers, setNativeMembers] = useState<NativeRoomMember[] | null | undefined>(null);
  const eventedClient = mx as unknown as {
    on(event: string, listener: (...args: any[]) => void): void;
    removeListener(event: string, listener: (...args: any[]) => void): void;
  };

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

    const room = mx.getRoom(roomId) as unknown as EventedRoomMembersReading | null;
    let loadingMembers = true;
    let disposed = false;

    const updateMemberList = (event?: MatrixEventReading) => {
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

    eventedClient.on(RoomMemberEvent.Membership, updateMemberList);
    eventedClient.on(RoomMemberEvent.PowerLevel, updateMemberList);
    return () => {
      disposed = true;
      eventedClient.removeListener(RoomMemberEvent.Membership, updateMemberList);
      eventedClient.removeListener(RoomMemberEvent.PowerLevel, updateMemberList);
    };
  }, [mx, roomId, nativeSession, eventedClient]);

  return nativeSession ? nativeMembers : members;
}
