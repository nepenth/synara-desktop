import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import test from 'node:test';
import { EventStatus, RoomEvent } from 'matrix-js-sdk';
import {
  RECENT_ROOM_WINDOW_MS,
  RoomActivityStore,
  getNextRecentRoomExpiry,
  partitionRoomIdsByActivity,
} from '../roomActivity';

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
    status?: EventStatus | null;
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

test('room activity partitions every room exactly once and sorts recent rooms newest first', () => {
  const now = 2 * RECENT_ROOM_WINDOW_MS;
  const snapshot = {
    revision: 1,
    entries: new Map([
      ['!newest:example.org', { roomId: '!newest:example.org', activityTs: now - 1, revision: 1 }],
      [
        '!recent:example.org',
        { roomId: '!recent:example.org', activityTs: now - 1_000, revision: 1 },
      ],
      [
        '!old:example.org',
        { roomId: '!old:example.org', activityTs: now - RECENT_ROOM_WINDOW_MS, revision: 1 },
      ],
    ]),
  };

  const partition = partitionRoomIdsByActivity(
    ['!old:example.org', '!recent:example.org', '!newest:example.org'],
    snapshot,
    now
  );

  assert.deepEqual(partition.recentRoomIds, ['!newest:example.org', '!recent:example.org']);
  assert.deepEqual(partition.nonRecentRoomIds, ['!old:example.org']);
  assert.equal(new Set([...partition.recentRoomIds, ...partition.nonRecentRoomIds]).size, 3);
});

test('a live message moves an old room directly into Recent without losing it from both lists', () => {
  const now = 3 * RECENT_ROOM_WINDOW_MS;
  const events = [createEvent('$old', now - RECENT_ROOM_WINDOW_MS - 1)];
  const room = createRoom('!room:example.org', events, 'Room');
  const mx = new MockMatrixClient([room]);
  const store = new RoomActivityStore(mx as any);
  const unsubscribe = store.subscribe(() => undefined);

  assert.deepEqual(
    partitionRoomIdsByActivity([room.roomId], store.getSnapshot(), now).nonRecentRoomIds,
    [room.roomId]
  );

  const liveMessage = createEvent('$live', now - 1);
  events.push(liveMessage);
  mx.emit(RoomEvent.Timeline, liveMessage, room, false, false, { liveEvent: true });

  const partition = partitionRoomIdsByActivity([room.roomId], store.getSnapshot(), now);
  assert.deepEqual(partition.recentRoomIds, [room.roomId]);
  assert.deepEqual(partition.nonRecentRoomIds, []);
  unsubscribe();
});

test('back-pagination, reactions, and edits do not change room activity', () => {
  const initial = createEvent('$message', 100);
  const events = [initial];
  const room = createRoom('!room:example.org', events);
  const mx = new MockMatrixClient([room]);
  const store = new RoomActivityStore(mx as any);
  const unsubscribe = store.subscribe(() => undefined);

  const reaction = createEvent('$reaction', 200, { type: 'm.reaction' });
  mx.emit(RoomEvent.Timeline, reaction, room, false, false, { liveEvent: true });
  const edit = createEvent('$edit', 300, { relation: { rel_type: 'm.replace' } });
  mx.emit(RoomEvent.Timeline, edit, room, false, false, { liveEvent: true });
  const paginated = createEvent('$paginated', 400);
  mx.emit(RoomEvent.Timeline, paginated, room, true, false, { liveEvent: false });

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

  mx.emit(RoomEvent.LocalEchoUpdated, localMessage, room);
  assert.equal(store.getSnapshot().entries.get(room.roomId)?.activityTs, 200);

  events.pop();
  const cancelled = createEvent('~local', 200, { status: EventStatus.CANCELLED });
  mx.emit(RoomEvent.LocalEchoUpdated, cancelled, room);
  assert.equal(store.getSnapshot().entries.get(room.roomId)?.activityTs, 100);
  assert.equal(store.getSnapshot().entries.get(room.roomId)?.latestEventId, '$old');
  unsubscribe();
});

test('next Recent expiry selects the earliest active room boundary', () => {
  const now = 10_000;
  const snapshot = {
    revision: 1,
    entries: new Map([
      ['!first:example.org', { roomId: '!first:example.org', activityTs: 100, revision: 1 }],
      ['!second:example.org', { roomId: '!second:example.org', activityTs: 200, revision: 1 }],
    ]),
  };

  assert.equal(
    getNextRecentRoomExpiry(['!second:example.org', '!first:example.org'], snapshot, now),
    100 + RECENT_ROOM_WINDOW_MS
  );
});
