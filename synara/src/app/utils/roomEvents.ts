import type { MatrixClientReading, MatrixEventReading, RoomReading } from './room';

/** SDK-neutral literal event names (mirror of the js-sdk RoomEvent subset used by Synara hooks). */
export const RoomEvent = {
  Name: 'Room.name',
  Receipt: 'Room.receipt',
  Timeline: 'Room.timeline',
  TimelineReset: 'Room.timelineReset',
  TimelineRefresh: 'Room.timelineRefresh',
  LocalEchoUpdated: 'Room.localEchoUpdated',
} as const;

export const RoomMemberEvent = {
  Membership: 'RoomMember.membership',
} as const;

export const RoomStateEvent = {
  Events: 'RoomState.events',
} as const;

type Listener = (...args: any[]) => void;

/**
 * Structural projection of a room that also supports event subscription and a
 * client accessor — js-sdk Room satisfies this at runtime, so consumers can
 * keep passing real rooms.
 */
export type EventedRoomReading = RoomReading & {
  client: MatrixClientReading;
  on(event: string, listener: Listener): void;
  removeListener(event: string, listener: Listener): void;
  getUsersReadUpTo(event: MatrixEventReading): string[];
};
