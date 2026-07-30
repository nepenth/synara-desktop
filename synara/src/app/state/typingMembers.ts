import produce from 'immer';
import { atom, useSetAtom } from 'jotai';
import { useEffect } from 'react';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../utils/desktop';
import { useSetting } from './hooks/settings';
import { settingsAtom } from './settings';

export const TYPING_TIMEOUT_MS = 5000; // 5 seconds

export type TypingReceipt = {
  userId: string;
  ts: number;
};
export type IRoomIdToTypingMembers = Map<string, TypingReceipt[]>;

type TypingMemberPutAction = {
  type: 'PUT';
  roomId: string;
  userId: string;
  ts: number;
};
type TypingMemberDeleteAction = {
  type: 'DELETE';
  roomId: string;
  userId: string;
};
type TypingMemberResetAction = {
  type: 'RESET';
  rooms: IRoomIdToTypingMembers;
};
export type IRoomIdToTypingMembersAction =
  | TypingMemberPutAction
  | TypingMemberDeleteAction
  | TypingMemberResetAction;

type NativeTypingRoom = {
  roomId: string;
  userIds: string[];
};

type NativeTypingSnapshot = {
  sessionGeneration: number;
  rooms: NativeTypingRoom[];
};

const baseRoomIdToTypingMembersAtom = atom<IRoomIdToTypingMembers>(new Map());

const putTypingMember = (
  roomToMembers: IRoomIdToTypingMembers,
  action: TypingMemberPutAction
): IRoomIdToTypingMembers => {
  let typingMembers = roomToMembers.get(action.roomId) ?? [];

  typingMembers = typingMembers.filter((receipt) => receipt.userId !== action.userId);
  typingMembers.push({
    userId: action.userId,
    ts: action.ts,
  });
  roomToMembers.set(action.roomId, typingMembers);
  return roomToMembers;
};

const deleteTypingMember = (
  roomToMembers: IRoomIdToTypingMembers,
  action: TypingMemberDeleteAction
): IRoomIdToTypingMembers => {
  let typingMembers = roomToMembers.get(action.roomId) ?? [];

  typingMembers = typingMembers.filter((receipt) => receipt.userId !== action.userId);
  if (typingMembers.length === 0) {
    roomToMembers.delete(action.roomId);
  } else {
    roomToMembers.set(action.roomId, typingMembers);
  }
  return roomToMembers;
};

export const typingMembersFromNativeSnapshot = (
  rooms: NativeTypingRoom[],
  ts = Date.now()
): IRoomIdToTypingMembers => {
  const next: IRoomIdToTypingMembers = new Map();
  for (const room of rooms) {
    if (!room.userIds.length) continue;
    next.set(
      room.roomId,
      room.userIds.map((userId) => ({ userId, ts }))
    );
  }
  return next;
};

export const roomIdToTypingMembersAtom = atom<
  IRoomIdToTypingMembers,
  [IRoomIdToTypingMembersAction],
  undefined
>(
  (get) => get(baseRoomIdToTypingMembersAtom),
  (get, set, action) => {
    if (action.type === 'RESET') {
      set(baseRoomIdToTypingMembersAtom, action.rooms);
      return;
    }

    const rToTyping = get(baseRoomIdToTypingMembersAtom);

    if (action.type === 'PUT') {
      set(
        baseRoomIdToTypingMembersAtom,
        produce(rToTyping, (draft) => putTypingMember(draft, action))
      );
      return;
    }

    if (
      action.type === 'DELETE' &&
      rToTyping.get(action.roomId)?.find((receipt) => receipt.userId === action.userId)
    ) {
      set(
        baseRoomIdToTypingMembersAtom,
        produce(rToTyping, (draft) => deleteTypingMember(draft, action))
      );
    }
  }
);

/**
 * Drive typing indicators from the native Rust typing index.
 * hideActivity clears the local projection and skips refresh.
 */
export const useBindRoomIdToTypingMembersAtom = (
  typingMembersAtom: typeof roomIdToTypingMembersAtom = roomIdToTypingMembersAtom
) => {
  const setTypingMembers = useSetAtom(typingMembersAtom);
  const [hideActivity] = useSetting(settingsAtom, 'hideActivity');

  useEffect(() => {
    let disposed = false;
    let inFlight = false;

    const clear = () => {
      setTypingMembers({ type: 'RESET', rooms: new Map() });
    };

    const refresh = async () => {
      if (inFlight || hideActivity) return;
      inFlight = true;
      try {
        const session = await invokeDesktopWithAvailability<{
          status: 'logged_out' | 'logged_in';
        }>('matrix_session_snapshot');
        if (disposed) return;
        if (!session.available || session.value?.status !== 'logged_in') {
          clear();
          return;
        }
        const result = await invokeDesktopWithAvailability<NativeTypingSnapshot>(
          'matrix_typing_snapshot'
        );
        if (!disposed && result.available && result.value) {
          setTypingMembers({
            type: 'RESET',
            rooms: typingMembersFromNativeSnapshot(result.value.rooms),
          });
        }
      } catch {
        // Preserve the last known typing projection during transient failures.
      } finally {
        inFlight = false;
      }
    };

    if (!isSynaraDesktop() || hideActivity) {
      clear();
      return undefined;
    }

    void refresh();
    const pollId = window.setInterval(() => void refresh(), 1_000);
    return () => {
      disposed = true;
      window.clearInterval(pollId);
    };
  }, [hideActivity, setTypingMembers]);
};
