import type { MatrixClientReading, MatrixEventReading, RoomReading } from '../../utils/room';
import { isNotificationEvent } from '../../utils/room';
import { recordFoundationDiagnostic } from '../../utils/foundationDiagnostics';
import { ClientEvent, MatrixEventEvent, RoomEvent } from '../../utils/roomEvents';

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

type RoomActivityListener = () => void;

type RoomActivitySubscription = {
  roomIds: ReadonlySet<string>;
  listener: RoomActivityListener;
};

export type RoomActivitySnapshotSource = {
  subscribe: (listener: RoomActivityListener) => () => void;
  getSnapshot: () => RoomActivitySnapshot;
};

type ClientEventedReading = MatrixClientReading & {
  on(event: string, listener: (...args: any[]) => unknown): unknown;
  removeListener(event: string, listener: (...args: any[]) => unknown): unknown;
};

export const isRoomActivityEvent = (event: MatrixEventReading): boolean =>
  event.status !== 'cancelled' &&
  event.status !== 'not_sent' &&
  event.getType() !== 'm.room.create' &&
  isNotificationEvent(event);

const getLatestActivityEvent = (room: RoomReading): MatrixEventReading | undefined => {
  const timelines: MatrixEventReading[][] = [
    room.getLiveTimeline().getEvents(),
    ...(room.getThreads?.() ?? []).map((thread) => thread.events),
  ];
  let latest: MatrixEventReading | undefined;

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

const getRoomBumpStamp = (room: RoomReading): number | undefined => {
  const bumpStamp = room.getBumpStamp?.();
  return typeof bumpStamp === 'number' && Number.isFinite(bumpStamp) ? bumpStamp : undefined;
};

const eventActivityTimestamp = (event: MatrixEventReading): number => {
  const timestamp = event.getTs();
  return typeof timestamp === 'number' && Number.isFinite(timestamp) ? timestamp : 0;
};

const snapshotsEqual = (left: RoomActivity | undefined, right: RoomActivity): boolean =>
  left?.activityTs === right.activityTs &&
  left.latestEventId === right.latestEventId &&
  left.bumpStamp === right.bumpStamp;

export const getLegacyRoomActivitySnapshot = (
  mx: MatrixClientReading,
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
            activityTs: room.getLastActiveTimestamp?.() ?? 0,
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

  public constructor(private readonly mx: ClientEventedReading) {
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
    room: RoomReading,
    event?: MatrixEventReading,
    missingPolicy: 'preserve' | 'summary' = 'preserve'
  ): void {
    const previous = this.snapshot.entries.get(room.roomId);
    const latestEvent = event && isRoomActivityEvent(event) ? event : getLatestActivityEvent(room);
    const latestTimestamp = latestEvent ? eventActivityTimestamp(latestEvent) : 0;
    const fallbackTimestamp =
      missingPolicy === 'preserve'
        ? previous?.activityTs ?? room.getLastActiveTimestamp?.() ?? 0
        : room.getLastActiveTimestamp?.() ?? 0;
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
          (room.getLastActiveTimestamp?.() ?? 0),
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
    event: MatrixEventReading,
    room: RoomReading | undefined,
    toStartOfTimeline?: boolean,
    removed = false,
    data?: { liveEvent?: boolean }
  ): void => {
    if (!room || toStartOfTimeline || removed || !data?.liveEvent || !isRoomActivityEvent(event)) {
      return;
    }
    this.updateRoom(room, event);
  };

  private readonly handleLocalEcho = (event: MatrixEventReading, room: RoomReading): void => {
    if (event.status === 'cancelled' || event.status === 'not_sent') {
      this.updateRoom(room, undefined, 'summary');
      return;
    }
    if (isRoomActivityEvent(event)) this.updateRoom(room, event);
  };

  private readonly handleTimelineReset = (room?: RoomReading): void => {
    if (room) this.updateRoom(room);
  };

  private readonly handleRoomRefresh = (room: RoomReading): void => this.updateRoom(room);

  private readonly handleRedaction = (event: MatrixEventReading, room: RoomReading): void => {
    const redactedEventId = event.getAssociatedId?.();
    if (
      redactedEventId &&
      this.snapshot.entries.get(room.roomId)?.latestEventId !== redactedEventId
    ) {
      return;
    }
    this.updateRoom(room);
  };

  private readonly handleRedactionCancelled = (
    _event: MatrixEventReading,
    room: RoomReading
  ): void => {
    this.updateRoom(room);
  };

  private readonly handleDecrypted = (event: MatrixEventReading): void => {
    const roomId = event.getRoomId();
    const room = roomId ? this.mx.getRoom(roomId) : null;
    const eventId = event.getId();
    if (!room || !eventId || this.snapshot.entries.get(room.roomId)?.latestEventId !== eventId) {
      return;
    }
    if (isRoomActivityEvent(event)) return;
    this.updateRoom(room);
  };

  private readonly handleAddRoom = (room: RoomReading): void => this.updateRoom(room);

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

const activityStores = new WeakMap<ClientEventedReading, RoomActivityStore>();

export const getRoomActivityStore = (mx: ClientEventedReading): RoomActivityStore => {
  let store = activityStores.get(mx);
  if (!store) {
    store = new RoomActivityStore(mx);
    activityStores.set(mx, store);
  }
  return store;
};
