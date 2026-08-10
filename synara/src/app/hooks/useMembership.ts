import { useEffect, useState } from 'react';
import { RoomMemberEvent } from '../utils/roomEvents';
import type { EventedRoomReading } from '../utils/roomEvents';
import type { MemberReading } from '../utils/room';
import { Membership } from '../../types/matrix/room';

export const useMembership = (room: EventedRoomReading, userId: string): Membership => {
  const member = room.getMember(userId) as
    | (MemberReading & {
        membership?: string;
        on(event: string, listener: (...args: any[]) => void): void;
        removeListener(event: string, listener: (...args: any[]) => void): void;
      })
    | null;

  const [membership, setMembership] = useState<Membership>(
    () => (member?.membership as Membership | undefined) ?? Membership.Leave
  );

  useEffect(() => {
    const handleMembershipChange = (
      event: import('../utils/room').MatrixEventReading,
      m: { userId: string; membership?: string }
    ) => {
      if (event.getRoomId() === room.roomId && m.userId === userId) {
        setMembership((m.membership as Membership | undefined) ?? Membership.Leave);
      }
    };
    member?.on(RoomMemberEvent.Membership, handleMembershipChange);
    return () => {
      member?.removeListener(RoomMemberEvent.Membership, handleMembershipChange);
    };
  }, [room, member, userId]);

  return membership;
};
