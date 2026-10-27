import { useEffect, useReducer } from 'react';
import { StateEvent } from '../../types/matrix/room';
import type { EventedRoomReading } from '../utils/roomEvents';
import { RoomEvent } from '../utils/roomEvents';
import { useStateEvent } from './useStateEvent';

type RoomJoinRulesEventContent = {
  join_rule?: string;
  allow?: string[];
};

export const useRoomAvatar = (room: EventedRoomReading, dm?: boolean): string | undefined => {
  const avatarEvent = useStateEvent(room, StateEvent.RoomAvatar);

  if (dm) {
    return room.getAvatarFallbackMember()?.getMxcAvatarUrl();
  }
  const content = avatarEvent?.getContent();
  const avatarMxc = content && typeof content.url === 'string' ? content.url : undefined;

  return avatarMxc;
};

/**
 * Reads the name on every render. Native rooms are an in-place facade over the
 * latest summary and emit no `RoomEvent.Name`, so their callers re-render from
 * the summary subscription; caching the name in state would freeze a row at
 * whatever it first saw (a new room's "Empty Room"). The listener still
 * re-renders rooms that do emit the event.
 */
export const useRoomName = (room: EventedRoomReading): string => {
  const [, onNameChange] = useReducer((n: number) => n + 1, 0);

  useEffect(() => {
    const handleRoomNameChange: (...args: unknown[]) => void = () => onNameChange();
    room.on(RoomEvent.Name, handleRoomNameChange as (...args: any[]) => void);
    return () => {
      room.removeListener(RoomEvent.Name, handleRoomNameChange);
    };
  }, [room]);

  return room.name ?? '';
};

export const useRoomTopic = (room: EventedRoomReading): string | undefined => {
  const topicEvent = useStateEvent(room, StateEvent.RoomTopic);

  const content = topicEvent?.getContent();
  const topic = content && typeof content.topic === 'string' ? content.topic : undefined;

  return topic;
};

export const useRoomJoinRule = (
  room: EventedRoomReading
): RoomJoinRulesEventContent | undefined => {
  const mEvent = useStateEvent(room, StateEvent.RoomJoinRules);
  const joinRuleContent = mEvent?.getContent<RoomJoinRulesEventContent>();
  return joinRuleContent;
};
