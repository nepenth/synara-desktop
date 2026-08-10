import { getStateEvent } from '../utils/room';
import type { RoomReading } from '../utils/room';
import { StateEvent } from '../../types/matrix/room';
import {
  normalizeRoomJoinRulePresentation,
  type RoomJoinRulePresentation,
} from '../features/matrix-dto/roomJoinRule';

export type LocalRoomSummary = {
  roomId: string;
  name: string;
  topic?: string;
  avatarUrl?: string;
  canonicalAlias?: string;
  worldReadable?: boolean;
  guestCanJoin?: boolean;
  memberCount?: number;
  roomType?: string;
  joinRule?: RoomJoinRulePresentation | null;
};
export const useLocalRoomSummary = (room: RoomReading): LocalRoomSummary => {
  const topicEvent = getStateEvent(room, StateEvent.RoomTopic);
  const topicContent = topicEvent?.getContent();
  const topic =
    topicContent && typeof topicContent.topic === 'string' ? topicContent.topic : undefined;

  const historyEvent = getStateEvent(room, StateEvent.RoomHistoryVisibility);
  const historyContent = historyEvent?.getContent();
  const worldReadable =
    historyContent && typeof historyContent.history_visibility === 'string'
      ? historyContent.history_visibility === 'world_readable'
      : undefined;

  const guestCanJoin =
    (room as unknown as { getGuestAccess(): string | null }).getGuestAccess() === 'can_join';

  return {
    roomId: room.roomId,
    name: (room as RoomReading & { name: string }).name,
    topic,
    avatarUrl: room.getMxcAvatarUrl() ?? undefined,
    canonicalAlias: room.getCanonicalAlias() ?? undefined,
    worldReadable,
    guestCanJoin,
    memberCount: room.getJoinedMemberCount(),
    roomType: room.getType(),
    joinRule: normalizeRoomJoinRulePresentation(room.getJoinRule()),
  };
};
