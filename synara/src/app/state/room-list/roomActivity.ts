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
import { recordFoundationDiagnostic } from '../../utils/foundationDiagnostics';
import { getLoadedLiveTimelineEvents } from '../../utils/timelineLifecycle';

export const RECENT_ROOM_WINDOW_MS = 24 * 60 * 60 * 1000;
const MAX_RECENT_ROOM_TIMEOUT_MS = 2_147_483_647;

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

type RoomActivitySubscription = {
  roomIds: ReadonlySet<string>;
  listener: RoomActivityListener;
};

export type RoomActivitySnapshotSource = {
  subscribe: (listener: RoomActivityListener) => () => void;
  getSnapshot: () => RoomActivitySnapshot;
};

export const isRoomActivityEvent = (event: MatrixEvent): boolean =>
  event.status !== EventStatus.CANCELLED &&
  event.status !== EventStatus.NOT_SENT &&
  event.getType() !== 'm.room.create' &&
  isNotificationEvent(event);

const getLatestActivityEvent = (room: Room): MatrixEvent | undefined => {
  const timelines = [
    getLoadedLiveTimelineEvents(room),
    ...room.getThreads().map((thread) => thread.events),
  ];
  let latest: MatrixEvent | undefined;

  for (const events of timelines) {
    for (let index = events.length - 1; index >= 0; index -= 1) {
      const event = events[index];
      if (!event || !isRoomActivityEvent(event)) continue;
      if (!latest || eventActivityTimestamp(event) >= eventActivityTimestamp(latest)) {
        latest = event;
      }
      break;
    }
  }
  return latest;
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

export const getRecentRoomExpiryDelay = (
  roomIds: readonly string[],
  activity: RoomActivitySnapshot,
  nowMs: number,
  windowMs = RECENT_ROOM_WINDOW_MS
): number | undefined => {
  const nextExpiry = getNextRecentRoomExpiry(roomIds, activity, nowMs, windowMs);
  if (nextExpiry === undefined) return undefined;
  return Math.min(MAX_RECENT_ROOM_TIMEOUT_MS, Math.max(1, nextExpiry - nowMs + 1));
};

export const getLegacyRoomActivitySnapshot = (
  mx: MatrixClient,
  roomIds: readonly string[]
): RoomActivitySnapshot => ({
  revision: 0,
  entries: new Map(
    roomIds.flatMap((roomId) => {
      const room = mx.getRoom(roomId);
      if (!room) return [];
      return [
        [
          roomId,
          {
            roomId,
            activityTs: room.getLastActiveTimestamp(),
            bumpStamp: getRoomBumpStamp(room),
            revision: 0,
          },
        ] as const,
      ];
    })
  ),
});

export class RoomActivityStore {
  private snapshot: RoomActivitySnapshot = { revision: 0, entries: new Map() };

  private readonly listeners = new Set<RoomActivityListener>();

  private readonly roomSubscriptions = new Set<RoomActivitySubscription>();

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
      this.stopIfUnused();
    };
  };

  public createSnapshotSource(roomIds: readonly string[]): RoomActivitySnapshotSource {
    const subscribedRoomIds = new Set(roomIds);
    let selectedSnapshot = this.snapshot;

    return {
      getSnapshot: () => selectedSnapshot,
      subscribe: (listener) => {
        // Close the render-to-subscribe race before React checks the snapshot.
        selectedSnapshot = this.snapshot;
        const subscription = {
          roomIds: subscribedRoomIds,
          listener: () => {
            selectedSnapshot = this.snapshot;
            listener();
          },
        };
        this.roomSubscriptions.add(subscription);
        if (!this.started) this.start();
        return () => {
          this.roomSubscriptions.delete(subscription);
          this.stopIfUnused();
        };
      },
    };
  }

  private publish(changedRoomIds: ReadonlySet<string>, entries = this.snapshot.entries): void {
    this.snapshot = { revision: this.snapshot.revision + 1, entries };
    this.listeners.forEach((listener) => listener());
    this.roomSubscriptions.forEach((subscription) => {
      for (const roomId of changedRoomIds) {
        if (subscription.roomIds.has(roomId)) {
          subscription.listener();
          break;
        }
      }
    });
  }

  private updateRoom(
    room: Room,
    event?: MatrixEvent,
    missingPolicy: 'preserve' | 'summary' = 'preserve'
  ): void {
    const previous = this.snapshot.entries.get(room.roomId);
    const latestEvent = event && isRoomActivityEvent(event) ? event : getLatestActivityEvent(room);
    const latestTimestamp = latestEvent ? eventActivityTimestamp(latestEvent) : 0;
    const fallbackTimestamp =
      missingPolicy === 'preserve'
        ? previous?.activityTs ?? room.getLastActiveTimestamp()
        : room.getLastActiveTimestamp();
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

    const entries = this.snapshot.entries as Map<string, RoomActivity>;
    entries.set(room.roomId, next);
    recordFoundationDiagnostic('activity', 'room-activity.updated', {
      roomId: room.roomId,
      eventId: latestEventId,
      fields: {
        revision: this.snapshot.revision + 1,
        hasConcreteHead: Boolean(latestEvent),
        preservedSummary: !latestEvent && Boolean(fallbackTimestamp),
        activityChanged: previous?.activityTs !== activityTs,
        latestChanged: previous?.latestEventId !== latestEventId,
      },
    });
    this.publish(new Set([room.roomId]));
  }

  private deleteRoom(roomId: string): void {
    if (!this.snapshot.entries.has(roomId)) return;
    const entries = this.snapshot.entries as Map<string, RoomActivity>;
    entries.delete(roomId);
    this.publish(new Set([roomId]));
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
    if (notify) {
      const changedRoomIds = new Set<string>();
      entries.forEach((entry, roomId) => {
        if (previousEntries.get(roomId) !== entry) changedRoomIds.add(roomId);
      });
      previousEntries.forEach((_entry, roomId) => {
        if (!entries.has(roomId)) changedRoomIds.add(roomId);
      });
      this.publish(changedRoomIds, entries);
    } else this.snapshot = { revision: this.snapshot.revision, entries };
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
    if (event.status === EventStatus.CANCELLED || event.status === EventStatus.NOT_SENT) {
      this.updateRoom(room, undefined, 'summary');
      return;
    }
    if (isRoomActivityEvent(event)) this.updateRoom(room, event);
  };

  private readonly handleTimelineReset = (room?: Room): void => {
    if (room) this.updateRoom(room);
  };

  private readonly handleRoomRefresh = (room: Room): void => this.updateRoom(room);

  private readonly handleRedaction = (event: MatrixEvent, room: Room): void => {
    const redactedEventId = event.getAssociatedId();
    if (
      redactedEventId &&
      this.snapshot.entries.get(room.roomId)?.latestEventId !== redactedEventId
    ) {
      return;
    }
    this.updateRoom(room);
  };

  private readonly handleRedactionCancelled = (_event: MatrixEvent, room: Room): void => {
    this.updateRoom(room);
  };

  private readonly handleDecrypted = (event: MatrixEvent): void => {
    const roomId = event.getRoomId();
    const room = roomId ? this.mx.getRoom(roomId) : null;
    const eventId = event.getId();
    if (!room || !eventId || this.snapshot.entries.get(roomId)?.latestEventId !== eventId) return;
    if (isRoomActivityEvent(event)) return;
    this.updateRoom(room);
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
    this.mx.on(RoomEvent.RedactionCancelled, this.handleRedactionCancelled);
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
    this.mx.removeListener(RoomEvent.RedactionCancelled, this.handleRedactionCancelled);
    this.mx.removeListener(MatrixEventEvent.Decrypted, this.handleDecrypted);
  }

  private stopIfUnused(): void {
    if (this.listeners.size === 0 && this.roomSubscriptions.size === 0) this.stop();
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
