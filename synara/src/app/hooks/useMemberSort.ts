import { RoomMember as MatrixRoomMember } from 'matrix-js-sdk';
import { useCallback, useMemo } from 'react';
import type { RoomMember as NativeRoomMember } from '../features/matrix-dto/member';

type RoomMemberListItem = MatrixRoomMember | NativeRoomMember;

const getMemberName = (member: RoomMemberListItem): string =>
  !('getMxcAvatarUrl' in member) ? member.displayName ?? member.userId : member.name;

const getMemberEventTs = (member: RoomMemberListItem): number =>
  !('getMxcAvatarUrl' in member) ? 0 : member.events.member?.getTs() ?? 0;

export const MemberSort = {
  Ascending: (a: RoomMemberListItem, b: RoomMemberListItem) =>
    getMemberName(a).toLowerCase() < getMemberName(b).toLowerCase() ? -1 : 1,
  Descending: (a: RoomMemberListItem, b: RoomMemberListItem) =>
    getMemberName(a).toLowerCase() > getMemberName(b).toLowerCase() ? -1 : 1,
  NewestFirst: (a: RoomMemberListItem, b: RoomMemberListItem) =>
    getMemberEventTs(b) - getMemberEventTs(a),
  Oldest: (a: RoomMemberListItem, b: RoomMemberListItem) =>
    getMemberEventTs(a) - getMemberEventTs(b),
};

export type MemberSortFn = (a: RoomMemberListItem, b: RoomMemberListItem) => number;

export type MemberSortItem = {
  name: string;
  sortFn: MemberSortFn;
};

export const useMemberSortMenu = (): MemberSortItem[] =>
  useMemo(
    () => [
      {
        name: 'A to Z',
        sortFn: MemberSort.Ascending,
      },
      {
        name: 'Z to A',
        sortFn: MemberSort.Descending,
      },
      {
        name: 'Newest',
        sortFn: MemberSort.NewestFirst,
      },
      {
        name: 'Oldest',
        sortFn: MemberSort.Oldest,
      },
    ],
    []
  );

export const useMemberSort = (index: number, memberSort: MemberSortItem[]): MemberSortItem => {
  const item = memberSort[index] ?? memberSort[0];
  return item;
};

export const useMemberPowerSort = (
  creators: Set<string>,
  getPowerLevel: (userId: string) => number
): MemberSortFn => {
  const sort: MemberSortFn = useCallback(
    (a, b) => {
      if (creators.has(a.userId) && creators.has(b.userId)) {
        return 0;
      }
      if (creators.has(a.userId)) return -1;
      if (creators.has(b.userId)) return 1;

      return getPowerLevel(b.userId) - getPowerLevel(a.userId);
    },
    [creators, getPowerLevel]
  );

  return sort;
};
