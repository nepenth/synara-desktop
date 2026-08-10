import { useBindAllInvitesAtom } from '../room-list/inviteList';
import {
  useBindAllRoomsAtom,
  type NativeRoomListSnapshot,
  type NativeSessionSnapshot,
} from '../room-list/roomList';
import { useBindMDirectAtom } from '../mDirectList';
import { useBindLaterContentAtom } from '../laterList';
import { useBindRoomNotesContentAtom } from '../roomNotesList';
import { useBindRoomToUnreadAtom } from '../room/roomToUnread';
import { useBindRoomToParentsAtom } from '../room/roomToParents';
import { useBindRoomIdToTypingMembersAtom } from '../typingMembers';

export const useBindAtoms = (
  onRoomListSnapshot?: (snapshot: NativeRoomListSnapshot) => void,
  onNativeSessionSnapshot?: (snapshot: NativeSessionSnapshot) => void
) => {
  useBindMDirectAtom();
  useBindLaterContentAtom();
  useBindRoomNotesContentAtom();
  useBindAllInvitesAtom();
  useBindAllRoomsAtom(onRoomListSnapshot, onNativeSessionSnapshot);
  useBindRoomToParentsAtom();
  useBindRoomToUnreadAtom();

  useBindRoomIdToTypingMembersAtom();
};
