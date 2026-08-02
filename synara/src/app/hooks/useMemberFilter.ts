import { useMemo } from 'react';
import { Membership } from '../../types/matrix/room';
import type { RoomMember as NativeRoomMember } from '../features/matrix-dto/member';
import type { RoomMemberListItem } from './useRoomMembers';

const isNativeRoomMember = (member: RoomMemberListItem): member is NativeRoomMember =>
  !('getMxcAvatarUrl' in member);

export const MembershipFilter = {
  filterJoined: (m: RoomMemberListItem) => m.membership === Membership.Join,
  filterInvited: (m: RoomMemberListItem) => m.membership === Membership.Invite,
  filterLeaved: (m: RoomMemberListItem) =>
    m.membership === Membership.Leave &&
    (isNativeRoomMember(m) || m.events.member?.getStateKey() === m.events.member?.getSender()),
  filterKicked: (m: RoomMemberListItem) =>
    m.membership === Membership.Leave &&
    !isNativeRoomMember(m) &&
    m.events.member?.getStateKey() !== m.events.member?.getSender(),
  filterBanned: (m: RoomMemberListItem) => m.membership === Membership.Ban,
};

export type MembershipFilterFn = (m: RoomMemberListItem) => boolean;

export type MembershipFilterItem = {
  name: string;
  filterFn: MembershipFilterFn;
};

export const useMembershipFilterMenu = (): MembershipFilterItem[] =>
  useMemo(
    () => [
      {
        name: 'Joined',
        filterFn: MembershipFilter.filterJoined,
      },
      {
        name: 'Invited',
        filterFn: MembershipFilter.filterInvited,
      },
      {
        name: 'Left',
        filterFn: MembershipFilter.filterLeaved,
      },
      {
        name: 'Kicked',
        filterFn: MembershipFilter.filterKicked,
      },
      {
        name: 'Banned',
        filterFn: MembershipFilter.filterBanned,
      },
    ],
    []
  );

export const useMembershipFilter = (
  index: number,
  membershipFilter: MembershipFilterItem[]
): MembershipFilterItem => {
  const filter = membershipFilter[index] ?? membershipFilter[0];
  return filter;
};
