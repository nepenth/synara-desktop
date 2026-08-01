import { useBindAllInvitesAtom } from '../room-list/inviteList';
import { useBindAllRoomsAtom } from '../room-list/roomList';
import { useBindMDirectAtom } from '../mDirectList';
import { useBindLaterContentAtom } from '../laterList';
import { useBindRoomNotesContentAtom } from '../roomNotesList';
import { useBindRoomToUnreadAtom } from '../room/roomToUnread';
import { useBindRoomToParentsAtom } from '../room/roomToParents';
import { useBindRoomIdToTypingMembersAtom } from '../typingMembers';

export const useBindAtoms = () => {
  useBindMDirectAtom();
  useBindLaterContentAtom();
  useBindRoomNotesContentAtom();
  useBindAllInvitesAtom();
  useBindAllRoomsAtom();
  useBindRoomToParentsAtom();
  useBindRoomToUnreadAtom();

  useBindRoomIdToTypingMembersAtom();
};
