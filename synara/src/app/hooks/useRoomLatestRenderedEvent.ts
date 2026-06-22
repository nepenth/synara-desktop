/* eslint-disable no-continue */
import { MatrixEvent, Room, RoomEvent, RoomEventHandlerMap } from 'matrix-js-sdk';
import { useEffect, useState } from 'react';
import { settingsAtom } from '../state/settings';
import { useSetting } from '../state/hooks/settings';
import { MessageEvent, StateEvent } from '../../types/matrix/room';
import { isMembershipChanged, reactionOrEditEvent } from '../utils/room';
import { getLoadedLiveTimelineEvents } from '../utils/timelineLifecycle';

export const useRoomLatestRenderedEvent = (room: Room) => {
  const [hideMembershipEvents] = useSetting(settingsAtom, 'hideMembershipEvents');
  const [hideNickAvatarEvents] = useSetting(settingsAtom, 'hideNickAvatarEvents');
  const [showHiddenEvents] = useSetting(settingsAtom, 'showHiddenEvents');
  const [latestEvent, setLatestEvent] = useState<MatrixEvent>();

  useEffect(() => {
    const getLatestEvent = (): MatrixEvent | undefined => {
      const liveEvents = getLoadedLiveTimelineEvents(room);
      for (let i = liveEvents.length - 1; i >= 0; i -= 1) {
        const evt = liveEvents[i];

        if (!evt) continue;
        if (reactionOrEditEvent(evt)) continue;
        if (evt.getType() === StateEvent.RoomMember) {
          const membershipChanged = isMembershipChanged(evt);
          if (membershipChanged && hideMembershipEvents) continue;
          if (!membershipChanged && hideNickAvatarEvents) continue;
          return evt;
        }

        if (
          evt.getType() === MessageEvent.RoomMessage ||
          evt.getType() === MessageEvent.RoomMessageEncrypted ||
          evt.getType() === MessageEvent.Sticker ||
          evt.getType() === StateEvent.RoomName ||
          evt.getType() === StateEvent.RoomTopic ||
          evt.getType() === StateEvent.RoomAvatar
        ) {
          return evt;
        }

        if (showHiddenEvents) return evt;
      }
      return undefined;
    };

    const handleTimelineEvent: RoomEventHandlerMap[RoomEvent.Timeline] = () => {
      setLatestEvent(getLatestEvent());
    };
    const handleTimelineLifecycleChange = () => {
      setLatestEvent(getLatestEvent());
    };
    setLatestEvent(getLatestEvent());

    room.on(RoomEvent.Timeline, handleTimelineEvent);
    room.on(RoomEvent.TimelineReset, handleTimelineLifecycleChange);
    room.on(RoomEvent.TimelineRefresh, handleTimelineLifecycleChange);
    return () => {
      room.removeListener(RoomEvent.Timeline, handleTimelineEvent);
      room.removeListener(RoomEvent.TimelineReset, handleTimelineLifecycleChange);
      room.removeListener(RoomEvent.TimelineRefresh, handleTimelineLifecycleChange);
    };
  }, [room, hideMembershipEvents, hideNickAvatarEvents, showHiddenEvents]);

  return latestEvent;
};
