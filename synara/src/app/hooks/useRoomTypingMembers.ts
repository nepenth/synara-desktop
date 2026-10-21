import { useAtomValue } from 'jotai';
import { selectAtom } from 'jotai/utils';
import { useCallback, useMemo } from 'react';
import {
  IRoomIdToTypingMembers,
  TypingReceipt,
  roomIdToTypingMembersAtom,
} from '../state/typingMembers';

const typingReceiptEqual = (a: TypingReceipt, b: TypingReceipt): boolean =>
  a.userId === b.userId && a.ts === b.ts;

const equalTypingMembers = (x: TypingReceipt[], y: TypingReceipt[]): boolean => {
  if (x.length !== y.length) return false;
  return x.every((a, i) => typingReceiptEqual(a, y[i]));
};

export const useRoomTypingMember = (roomId: string) => {
  const selector = useCallback(
    (roomToTyping: IRoomIdToTypingMembers) => roomToTyping.get(roomId) ?? [],
    [roomId]
  );

  // A new selectAtom per render would re-subscribe every render (D10).
  const typingAtom = useMemo(
    () => selectAtom(roomIdToTypingMembersAtom, selector, equalTypingMembers),
    [selector]
  );
  return useAtomValue(typingAtom);
};
