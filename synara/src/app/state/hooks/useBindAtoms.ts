import { MatrixClient } from 'matrix-js-sdk';
import { useBindAllInvitesAtom } from '../room-list/inviteList';
import { useBindAllRoomsAtom } from '../room-list/roomList';
import { mDirectAtom, useBindMDirectAtom } from '../mDirectList';
import { useBindRoomToUnreadAtom } from '../room/roomToUnread';
import { useBindRoomToParentsAtom } from '../room/roomToParents';
import { useBindRoomIdToTypingMembersAtom } from '../typingMembers';

export const useBindAtoms = (mx: MatrixClient) => {
  useBindMDirectAtom(mx, mDirectAtom);
  useBindAllInvitesAtom();
  useBindAllRoomsAtom();
  useBindRoomToParentsAtom();
  useBindRoomToUnreadAtom();

  useBindRoomIdToTypingMembersAtom();
};
