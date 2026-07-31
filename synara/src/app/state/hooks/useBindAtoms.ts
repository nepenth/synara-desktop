import { useBindAllInvitesAtom } from '../room-list/inviteList';
import { useBindAllRoomsAtom } from '../room-list/roomList';
import { useBindMDirectAtom } from '../mDirectList';
import { useBindRoomToUnreadAtom } from '../room/roomToUnread';
import { useBindRoomToParentsAtom } from '../room/roomToParents';
import { useBindRoomIdToTypingMembersAtom } from '../typingMembers';

export const useBindAtoms = () => {
  useBindMDirectAtom();
  useBindAllInvitesAtom();
  useBindAllRoomsAtom();
  useBindRoomToParentsAtom();
  useBindRoomToUnreadAtom();

  useBindRoomIdToTypingMembersAtom();
};
