import { useEffect, useState } from 'react';
import { useMatrixClient } from './useMatrixClient';
import type { RoomReading } from '../utils/room';

type LocalMx = ReturnType<typeof useMatrixClient>;
type RtcManager = LocalMx['matrixRTC'];
type RtcSession = ReturnType<RtcManager['getRoomSession']>;

/** Narrow structural projection of a MatrixRTC call membership (satisfied by
 * the live js-sdk CallMembership; mirrors the call-status boundary type). */
export type CallMembershipReading = {
  sender: string;
  membershipID: string;
};

export const useCallSession = (room: RoomReading): RtcSession => {
  const mx = useMatrixClient();

  const [session, setSession] = useState(
    mx.matrixRTC.getRoomSession(room as unknown as Parameters<RtcManager['getRoomSession']>[0])
  );

  useEffect(() => {
    const start = (roomId: string) => {
      if (roomId !== room.roomId) return;
      setSession(
        mx.matrixRTC.getRoomSession(room as unknown as Parameters<RtcManager['getRoomSession']>[0])
      );
    };
    const end = (roomId: string) => {
      if (roomId !== room.roomId) return;
      setSession(
        mx.matrixRTC.getRoomSession(room as unknown as Parameters<RtcManager['getRoomSession']>[0])
      );
    };
    mx.matrixRTC.on('session_started' as unknown as Parameters<RtcManager['on']>[0], start);
    mx.matrixRTC.on('session_ended' as unknown as Parameters<RtcManager['on']>[0], end);
    return () => {
      mx.matrixRTC.off('session_started' as unknown as Parameters<RtcManager['off']>[0], start);
      mx.matrixRTC.off('session_ended' as unknown as Parameters<RtcManager['off']>[0], end);
    };
  }, [mx, room]);

  return session;
};

export const useCallMembers = (room: RoomReading, session: RtcSession): CallMembershipReading[] => {
  const [memberships, setMemberships] = useState<CallMembershipReading[]>(() => [
    ...session.memberships,
  ]);

  useEffect(() => {
    const updateMemberships = () => {
      setMemberships([...session.memberships]);
    };

    updateMemberships();

    session.on(
      'memberships_changed' as unknown as Parameters<RtcSession['on']>[0],
      updateMemberships
    );
    return () => {
      session.removeListener(
        'memberships_changed' as unknown as Parameters<RtcSession['removeListener']>[0],
        updateMemberships
      );
    };
  }, [session]);

  return memberships;
};

export const useCallMembersChange = (session: RtcSession, callback: () => void): void => {
  useEffect(() => {
    session.on('memberships_changed' as unknown as Parameters<RtcSession['on']>[0], callback);
    return () => {
      session.removeListener(
        'memberships_changed' as unknown as Parameters<RtcSession['removeListener']>[0],
        callback
      );
    };
  }, [session, callback]);
};
