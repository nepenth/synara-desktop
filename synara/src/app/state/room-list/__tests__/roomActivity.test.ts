import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import test from 'node:test';
// Event/status literals are the probed js-sdk values:
// EventStatus CANCELLED/NOT_SENT = 'cancelled'/'not_sent';
// RoomEvent Timeline/LocalEchoUpdated/Redaction/RedactionCancelled =
//   'Room.timeline'/'Room.localEchoUpdated'/'Room.redaction'/'Room.redactionCancelled';
// MatrixEventEvent.Decrypted = 'Event.decrypted'.
import { RoomActivityStore, getLegacyRoomActivitySnapshot } from '../roomActivity';
import {
  clearDesktopDiagnostics,
  getDesktopDiagnosticEntries,
} from '../../../utils/desktopDiagnostics';

const createEvent = (
  id: string,
  ts: number,
  {
    type = 'm.room.message',
    relation,
    status = null,
  }: {
    type?: string;
    relation?: { rel_type: string };
    status?: string | null;
  } = {}
) =>
  ({
    status,
    getId: () => id,
    getTs: () => ts,
    getType: () => type,
    getRelation: () => relation,
    getRoomId: () => '!room:example.org',
    isRedacted: () => false,
  } as any);

const createRoom = (roomId: string, events: any[], name = roomId) =>
  ({
    roomId,
    name,
    getLiveTimeline: () => ({ getEvents: () => events }),
    getThreads: () => [],
    getLastActiveTimestamp: () => events.at(-1)?.getTs() ?? 0,
    getBumpStamp: () => undefined,
  } as any);

class MockMatrixClient extends EventEmitter {
  public constructor(public rooms: any[]) {
    super();
  }

  public getRooms = () => this.rooms;

  public getRoom = (roomId: string) =>
    this.rooms.find((room: { roomId: string }) => room.roomId === roomId) ?? null;

  public removeListener(eventName: string | symbol, listener: (...args: any[]) => void): this {
    return super.removeListener(eventName, listener);
  }
}

test('legacy activity fallback uses bounded room summary metadata without scanning timelines', () => {
  let scans = 0;
  const room = {
    ...createRoom('!legacy:example.org', []),
    getLastActiveTimestamp: () => 123,
    getLiveTimeline: () => {
      scans += 1;
      return { getEvents: () => [] };
    },
  } as any;
  const mx = new MockMatrixClient([room]);

  const snapshot = getLegacyRoomActivitySnapshot(mx as any, [room.roomId, '!missing:example.org']);

  assert.equal(scans, 0);
  assert.equal(snapshot.entries.size, 1);
  assert.equal(snapshot.entries.get(room.roomId)?.activityTs, 123);
});

test('a live message updates stored activity without inventing a timestamp', () => {
  const events = [createEvent('$old', 100)];
  const room = createRoom('!room:example.org', events, 'Room');
  const mx = new MockMatrixClient([room]);
  const store = new RoomActivityStore(mx as any);
  const unsubscribe = store.subscribe(() => undefined);
  clearDesktopDiagnostics();

  assert.equal(store.getSnapshot().entries.get(room.roomId)?.activityTs, 100);

  const liveMessage = createEvent('$live', 200);
  events.push(liveMessage);
  mx.emit('Room.timeline', liveMessage, room, false, false, { liveEvent: true });

  assert.equal(store.getSnapshot().entries.get(room.roomId)?.activityTs, 200);
  const diagnostic = getDesktopDiagnosticEntries().find((entry) =>
    entry.includes('room-activity.updated')
  );
  assert.ok(diagnostic);
  assert.equal(diagnostic.includes(room.roomId), false);
  assert.equal(diagnostic.includes('$live'), false);
  unsubscribe();
});

test('loaded thread-only activity keeps the latest qualifying timestamp', () => {
  const mainEvents = [createEvent('$old-main', 100)];
  const threadEvent = createEvent('$thread-live', 200);
  const room = {
    ...createRoom('!thread-room:example.org', mainEvents),
    getThreads: () => [{ events: [threadEvent] }],
  } as any;
  const mx = new MockMatrixClient([room]);
  const store = new RoomActivityStore(mx as any);

  assert.equal(store.getSnapshot().entries.get(room.roomId)?.activityTs, 200);
  assert.equal(store.getSnapshot().entries.get(room.roomId)?.latestEventId, '$thread-live');
});

test('back-pagination, reactions, and edits do not change room activity', () => {
  const initial = createEvent('$message', 100);
  const events = [initial];
  const room = createRoom('!room:example.org', events);
  const mx = new MockMatrixClient([room]);
  const store = new RoomActivityStore(mx as any);
  const unsubscribe = store.subscribe(() => undefined);

  const reaction = createEvent('$reaction', 200, { type: 'm.reaction' });
  mx.emit('Room.timeline', reaction, room, false, false, { liveEvent: true });
  const edit = createEvent('$edit', 300, { relation: { rel_type: 'm.replace' } });
  mx.emit('Room.timeline', edit, room, false, false, { liveEvent: true });
  const paginated = createEvent('$paginated', 400);
  mx.emit('Room.timeline', paginated, room, true, false, { liveEvent: false });

  assert.equal(store.getSnapshot().entries.get(room.roomId)?.activityTs, 100);
  assert.equal(store.getSnapshot().entries.get(room.roomId)?.latestEventId, '$message');
  unsubscribe();
});

test('cancelled local echoes fall back to the previous relevant event', () => {
  const oldMessage = createEvent('$old', 100);
  const localMessage = createEvent('~local', 200);
  const events = [oldMessage, localMessage];
  const room = createRoom('!room:example.org', events);
  const mx = new MockMatrixClient([room]);
  const store = new RoomActivityStore(mx as any);
  const unsubscribe = store.subscribe(() => undefined);

  mx.emit('Room.localEchoUpdated', localMessage, room);
  assert.equal(store.getSnapshot().entries.get(room.roomId)?.activityTs, 200);

  localMessage.status = 'cancelled';
  mx.emit('Room.localEchoUpdated', localMessage, room);
  assert.equal(store.getSnapshot().entries.get(room.roomId)?.activityTs, 100);
  assert.equal(store.getSnapshot().entries.get(room.roomId)?.latestEventId, '$old');
  unsubscribe();
});

test('directly failed local echoes fall back to the previous relevant event', () => {
  const oldMessage = createEvent('$old', 100);
  const localMessage = createEvent('~local', 200);
  const events = [oldMessage, localMessage];
  const room = createRoom('!room:example.org', events);
  const mx = new MockMatrixClient([room]);
  const store = new RoomActivityStore(mx as any);
  const unsubscribe = store.subscribe(() => undefined);

  localMessage.status = 'not_sent';
  mx.emit('Room.localEchoUpdated', localMessage, room);

  assert.equal(store.getSnapshot().entries.get(room.roomId)?.activityTs, 100);
  assert.equal(store.getSnapshot().entries.get(room.roomId)?.latestEventId, '$old');
  unsubscribe();
});

for (const terminalStatus of ['cancelled', 'not_sent']) {
  test(`${terminalStatus} summary-only local echoes retain the room summary timestamp`, () => {
    const localMessage = createEvent('~local', 200);
    const events = [localMessage];
    const room = {
      ...createRoom('!summary-only:example.org', events),
      getLastActiveTimestamp: () => 123,
    } as any;
    const mx = new MockMatrixClient([room]);
    const store = new RoomActivityStore(mx as any);
    const unsubscribe = store.subscribe(() => undefined);

    localMessage.status = terminalStatus;
    mx.emit('Room.localEchoUpdated', localMessage, room);

    assert.equal(store.getSnapshot().entries.get(room.roomId)?.activityTs, 123);
    assert.equal(store.getSnapshot().entries.get(room.roomId)?.latestEventId, undefined);
    unsubscribe();
  });
}

test('decryption rescans only an ineligible live activity head', () => {
  const events = Array.from({ length: 100 }, (_, index) => createEvent(`$event-${index}`, index));
  const head = events.at(-1)!;
  let scans = 0;
  const room = {
    ...createRoom('!room:example.org', events),
    getLiveTimeline: () => ({
      getEvents: () => {
        scans += 1;
        return events;
      },
    }),
  } as any;
  const mx = new MockMatrixClient([room]);
  const store = new RoomActivityStore(mx as any);
  const unsubscribe = store.subscribe(() => undefined);
  const baselineScans = scans;

  events.slice(0, -1).forEach((event) => mx.emit('Event.decrypted', event));
  assert.equal(scans, baselineScans);

  mx.emit('Event.decrypted', head);
  assert.equal(scans, baselineScans);

  head.getRelation = () => ({ rel_type: 'm.replace' });
  mx.emit('Event.decrypted', head);
  assert.equal(scans, baselineScans + 1);
  assert.equal(store.getSnapshot().entries.get(room.roomId)?.latestEventId, '$event-98');

  mx.emit('Event.decrypted', head);
  assert.equal(scans, baselineScans + 1);
  unsubscribe();
});

test('an ineligible decrypted head preserves prior activity when no concrete fallback is loaded', () => {
  const head = createEvent('$encrypted-head', 200, { type: 'm.room.encrypted' });
  const events = [head];
  const room = createRoom('!room:example.org', events);
  const mx = new MockMatrixClient([room]);
  const store = new RoomActivityStore(mx as any);
  const unsubscribe = store.subscribe(() => undefined);

  head.getType = () => 'm.room.message';
  head.getRelation = () => ({ rel_type: 'm.replace' });
  mx.emit('Event.decrypted', head);

  assert.equal(store.getSnapshot().entries.get(room.roomId)?.activityTs, 200);
  assert.equal(store.getSnapshot().entries.get(room.roomId)?.latestEventId, undefined);
  unsubscribe();
});

test('redacting the only concrete activity head preserves the prior summary timestamp', () => {
  const head = createEvent('$head', 200);
  const events = [head];
  const room = createRoom('!room:example.org', events);
  const mx = new MockMatrixClient([room]);
  const store = new RoomActivityStore(mx as any);
  const unsubscribe = store.subscribe(() => undefined);
  const redaction = createEvent('$redaction', 300, { type: 'm.room.redaction' });
  redaction.getAssociatedId = () => '$head';
  head.isRedacted = () => true;

  mx.emit('Room.redaction', redaction, room);

  assert.equal(store.getSnapshot().entries.get(room.roomId)?.activityTs, 200);
  assert.equal(store.getSnapshot().entries.get(room.roomId)?.latestEventId, undefined);
  unsubscribe();
});

test('cancelling a redaction restores the formerly eligible activity head', () => {
  const head = createEvent('$head', 200);
  const events = [head];
  const room = createRoom('!room:example.org', events);
  const mx = new MockMatrixClient([room]);
  const store = new RoomActivityStore(mx as any);
  const unsubscribe = store.subscribe(() => undefined);
  const redaction = createEvent('$redaction', 300, { type: 'm.room.redaction' });
  redaction.getAssociatedId = () => '$head';
  let redacted = true;
  head.isRedacted = () => redacted;

  mx.emit('Room.redaction', redaction, room);
  assert.equal(store.getSnapshot().entries.get(room.roomId)?.latestEventId, undefined);

  redacted = false;
  mx.emit('Room.redactionCancelled', redaction, room);
  assert.equal(store.getSnapshot().entries.get(room.roomId)?.activityTs, 200);
  assert.equal(store.getSnapshot().entries.get(room.roomId)?.latestEventId, '$head');
  unsubscribe();
});

test('5k-room snapshot sources ignore unrelated updates without copying the activity map', () => {
  const rooms = Array.from({ length: 5_000 }, (_, index) =>
    createRoom(`!room-${index}:example.org`, [createEvent(`$event-${index}`, index)])
  );
  const subscribedRoom = rooms[0];
  const unrelatedRoom = rooms.at(-1)!;
  const mx = new MockMatrixClient(rooms);
  const store = new RoomActivityStore(mx as any);
  const source = store.createSnapshotSource([subscribedRoom.roomId]);
  let notifications = 0;
  const unsubscribe = source.subscribe(() => {
    notifications += 1;
  });
  const initialSnapshot = source.getSnapshot();
  const initialEntries = initialSnapshot.entries;

  const unrelatedMessage = createEvent('$unrelated-live', 10_000);
  mx.emit('Room.timeline', unrelatedMessage, unrelatedRoom, false, false, { liveEvent: true });

  assert.equal(notifications, 0);
  assert.equal(source.getSnapshot(), initialSnapshot);
  assert.equal(source.getSnapshot().entries, initialEntries);

  const subscribedMessage = createEvent('$subscribed-live', 20_000);
  mx.emit('Room.timeline', subscribedMessage, subscribedRoom, false, false, { liveEvent: true });

  assert.equal(notifications, 1);
  assert.notEqual(source.getSnapshot(), initialSnapshot);
  assert.equal(source.getSnapshot().entries, initialEntries);
  assert.equal(source.getSnapshot().entries.get(subscribedRoom.roomId)?.activityTs, 20_000);
  unsubscribe();
});
