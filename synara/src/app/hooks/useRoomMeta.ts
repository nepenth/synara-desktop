import { useEffect, useState } from 'react';
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

export const useRoomName = (room: EventedRoomReading): string => {
  const [name, setName] = useState(room.name ?? '');

  useEffect(() => {
    setName(room.name ?? '');

    const handleRoomNameChange: (...args: unknown[]) => void = () => {
      setName(room.name ?? '');
    };
    room.on(RoomEvent.Name, handleRoomNameChange as (...args: any[]) => void);
    return () => {
      room.removeListener(RoomEvent.Name, handleRoomNameChange);
    };
  }, [room]);

  return name;
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
