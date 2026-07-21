import {
  ClientEvent,
  EventStatus,
  MatrixClient,
  MatrixEvent,
  MatrixEventEvent,
  Room,
  RoomEvent,
} from 'matrix-js-sdk';
import { isNotificationEvent } from '../../utils/room';
import { getLoadedLiveTimelineEvents } from '../../utils/timelineLifecycle';

export const RECENT_ROOM_WINDOW_MS = 24 * 60 * 60 * 1000;

export type RoomActivity = {
  roomId: string;
  activityTs: number;
  latestEventId?: string;
  bumpStamp?: number;
  revision: number;
};

export type RoomActivitySnapshot = {
  revision: number;
  entries: ReadonlyMap<string, RoomActivity>;
};

export type RoomActivityPartition = {
  recentRoomIds: string[];
  nonRecentRoomIds: string[];
};

type RoomActivityListener = () => void;

const isRoomActivityEvent = (event: MatrixEvent): boolean =>
  event.getType() !== 'm.room.create' && isNotificationEvent(event);

const getLatestActivityEvent = (room: Room): MatrixEvent | undefined => {
  const events = getLoadedLiveTimelineEvents(room);
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const event = events[index];
    if (event && isRoomActivityEvent(event)) return event;
  }
  return undefined;
};

const getRoomBumpStamp = (room: Room): number | undefined => {
  const bumpStamp = room.getBumpStamp();
  return typeof bumpStamp === 'number' && Number.isFinite(bumpStamp) ? bumpStamp : undefined;
};

const eventActivityTimestamp = (event: MatrixEvent): number => {
  const timestamp = event.getTs();
  return typeof timestamp === 'number' && Number.isFinite(timestamp) ? timestamp : 0;
};

const snapshotsEqual = (left: RoomActivity | undefined, right: RoomActivity): boolean =>
  left?.activityTs === right.activityTs &&
  left.latestEventId === right.latestEventId &&
  left.bumpStamp === right.bumpStamp;

export const partitionRoomIdsByActivity = (
  roomIds: readonly string[],
  activity: RoomActivitySnapshot,
  nowMs: number,
  getRoomName: (roomId: string) => string = (roomId) => roomId,
  windowMs = RECENT_ROOM_WINDOW_MS
): RoomActivityPartition => {
  const cutoff = nowMs - windowMs;
  const recentRoomIds: string[] = [];
  const nonRecentRoomIds: string[] = [];

  roomIds.forEach((roomId) => {
    const timestamp = activity.entries.get(roomId)?.activityTs ?? 0;
    if (timestamp > cutoff) recentRoomIds.push(roomId);
    else nonRecentRoomIds.push(roomId);
  });

  recentRoomIds.sort((leftId, rightId) => {
    const timestampDelta =
      (activity.entries.get(rightId)?.activityTs ?? 0) -
      (activity.entries.get(leftId)?.activityTs ?? 0);
    if (timestampDelta !== 0) return timestampDelta;

    const nameDelta = getRoomName(leftId).localeCompare(getRoomName(rightId), undefined, {
      sensitivity: 'base',
    });
    return nameDelta || leftId.localeCompare(rightId);
  });

  return { recentRoomIds, nonRecentRoomIds };
};

export const getNextRecentRoomExpiry = (
  roomIds: readonly string[],
  activity: RoomActivitySnapshot,
  nowMs: number,
  windowMs = RECENT_ROOM_WINDOW_MS
): number | undefined => {
  let nextExpiry: number | undefined;
  roomIds.forEach((roomId) => {
    const timestamp = activity.entries.get(roomId)?.activityTs ?? 0;
    const expiry = timestamp + windowMs;
    if (timestamp <= 0 || expiry <= nowMs) return;
    if (nextExpiry === undefined || expiry < nextExpiry) nextExpiry = expiry;
  });
  return nextExpiry;
};

export class RoomActivityStore {
  private snapshot: RoomActivitySnapshot = { revision: 0, entries: new Map() };

  private readonly listeners = new Set<RoomActivityListener>();

  private started = false;

  public constructor(private readonly mx: MatrixClient) {
    this.refreshAll(false);
  }

  public getSnapshot = (): RoomActivitySnapshot => this.snapshot;

  public subscribe = (listener: RoomActivityListener): (() => void) => {
    this.listeners.add(listener);
    if (!this.started) this.start();
    return () => {
      this.listeners.delete(listener);
      if (this.listeners.size === 0) this.stop();
    };
  };

  private emit(entries: Map<string, RoomActivity>): void {
    this.snapshot = { revision: this.snapshot.revision + 1, entries };
    this.listeners.forEach((listener) => listener());
  }

  private updateRoom(room: Room, event?: MatrixEvent, preserveMissing = true): void {
    const previous = this.snapshot.entries.get(room.roomId);
    const latestEvent = event && isRoomActivityEvent(event) ? event : getLatestActivityEvent(room);
    const latestTimestamp = latestEvent ? eventActivityTimestamp(latestEvent) : 0;
    const fallbackTimestamp = preserveMissing
      ? previous?.activityTs ?? room.getLastActiveTimestamp()
      : 0;
    const candidateTimestamp = latestTimestamp || fallbackTimestamp;
    const activityTs =
      previous && event && candidateTimestamp < previous.activityTs
        ? previous.activityTs
        : candidateTimestamp;
    const latestEventId =
      previous && event && candidateTimestamp < previous.activityTs
        ? previous.latestEventId
        : latestEvent?.getId();
    const next: RoomActivity = {
      roomId: room.roomId,
      activityTs,
      latestEventId,
      bumpStamp: getRoomBumpStamp(room),
      revision: (previous?.revision ?? 0) + 1,
    };
    if (snapshotsEqual(previous, next)) return;

    const entries = new Map(this.snapshot.entries);
    entries.set(room.roomId, next);
    this.emit(entries);
  }

  private deleteRoom(roomId: string): void {
    if (!this.snapshot.entries.has(roomId)) return;
    const entries = new Map(this.snapshot.entries);
    entries.delete(roomId);
    this.emit(entries);
  }

  private refreshAll(notify = true): void {
    const previousEntries = this.snapshot.entries;
    const entries = new Map<string, RoomActivity>();
    this.mx.getRooms().forEach((room) => {
      const previous = previousEntries.get(room.roomId);
      const latestEvent = getLatestActivityEvent(room);
      const next: RoomActivity = {
        roomId: room.roomId,
        activityTs:
          (latestEvent && eventActivityTimestamp(latestEvent)) ||
          previous?.activityTs ||
          room.getLastActiveTimestamp(),
        latestEventId: latestEvent?.getId() ?? previous?.latestEventId,
        bumpStamp: getRoomBumpStamp(room),
        revision: (previous?.revision ?? 0) + 1,
      };
      entries.set(room.roomId, snapshotsEqual(previous, next) && previous ? previous : next);
    });

    const changed =
      entries.size !== previousEntries.size ||
      Array.from(entries).some(([roomId, entry]) => previousEntries.get(roomId) !== entry);
    if (!changed) return;
    if (notify) this.emit(entries);
    else this.snapshot = { revision: this.snapshot.revision, entries };
  }

  private readonly handleTimeline = (
    event: MatrixEvent,
    room: Room | undefined,
    toStartOfTimeline?: boolean,
    removed = false,
    data?: { liveEvent?: boolean }
  ): void => {
    if (!room || toStartOfTimeline || removed || !data?.liveEvent || !isRoomActivityEvent(event)) {
      return;
    }
    this.updateRoom(room, event);
  };

  private readonly handleLocalEcho = (event: MatrixEvent, room: Room): void => {
    if (event.status === EventStatus.CANCELLED) {
      this.updateRoom(room, undefined, false);
      return;
    }
    if (isRoomActivityEvent(event)) this.updateRoom(room, event);
  };

  private readonly handleTimelineReset = (room?: Room): void => {
    if (room) this.updateRoom(room);
  };

  private readonly handleRoomRefresh = (room: Room): void => this.updateRoom(room);

  private readonly handleRedaction = (_event: MatrixEvent, room: Room): void =>
    this.updateRoom(room, undefined, false);

  private readonly handleDecrypted = (event: MatrixEvent): void => {
    const roomId = event.getRoomId();
    const room = roomId ? this.mx.getRoom(roomId) : null;
    if (room) this.updateRoom(room, undefined, false);
  };

  private readonly handleAddRoom = (room: Room): void => this.updateRoom(room);

  private readonly handleDeleteRoom = (roomId: string): void => this.deleteRoom(roomId);

  private start(): void {
    this.started = true;
    this.refreshAll();
    this.mx.on(ClientEvent.Room, this.handleAddRoom);
    this.mx.on(ClientEvent.DeleteRoom, this.handleDeleteRoom);
    this.mx.on(RoomEvent.MyMembership, this.handleRoomRefresh);
    this.mx.on(RoomEvent.Timeline, this.handleTimeline);
    this.mx.on(RoomEvent.TimelineReset, this.handleTimelineReset);
    this.mx.on(RoomEvent.LocalEchoUpdated, this.handleLocalEcho);
    this.mx.on(RoomEvent.Redaction, this.handleRedaction);
    this.mx.on(RoomEvent.RedactionCancelled, this.handleRedaction);
    this.mx.on(MatrixEventEvent.Decrypted, this.handleDecrypted);
  }

  private stop(): void {
    this.started = false;
    this.mx.removeListener(ClientEvent.Room, this.handleAddRoom);
    this.mx.removeListener(ClientEvent.DeleteRoom, this.handleDeleteRoom);
    this.mx.removeListener(RoomEvent.MyMembership, this.handleRoomRefresh);
    this.mx.removeListener(RoomEvent.Timeline, this.handleTimeline);
    this.mx.removeListener(RoomEvent.TimelineReset, this.handleTimelineReset);
    this.mx.removeListener(RoomEvent.LocalEchoUpdated, this.handleLocalEcho);
    this.mx.removeListener(RoomEvent.Redaction, this.handleRedaction);
    this.mx.removeListener(RoomEvent.RedactionCancelled, this.handleRedaction);
    this.mx.removeListener(MatrixEventEvent.Decrypted, this.handleDecrypted);
  }
}

const activityStores = new WeakMap<MatrixClient, RoomActivityStore>();

export const getRoomActivityStore = (mx: MatrixClient): RoomActivityStore => {
  let store = activityStores.get(mx);
  if (!store) {
    store = new RoomActivityStore(mx);
    activityStores.set(mx, store);
  }
  return store;
};
