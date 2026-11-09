import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import type { RoomSummary } from '../../../features/matrix-dto/room';
import {
  knownRoomListRevision,
  parseNativeRoomListSnapshotReply,
  sameNativeRoomListSnapshot,
  sameStringList,
} from '../roomList';
import { EMPTY_ROOM_LIST_PRESENTATION } from '../roomListPresentation';

const room = (overrides: Partial<RoomSummary> & Pick<RoomSummary, 'roomId'>): RoomSummary =>
  ({
    membership: 'join',
    isDirect: false,
    isSpace: false,
    isCall: false,
    hasActiveCall: false,
    activeCallParticipantCount: 0,
    isFavorite: false,
    isEncrypted: false,
    encryptionStatus: 'not_encrypted',
    unreadCount: 0,
    highlightCount: 0,
    markedUnread: false,
    lastMessageIsAgentApproval: false,
    ...overrides,
  }) as RoomSummary;

test('sameStringList compares order and identity, not array reference', () => {
  const first = ['!a', '!b'];
  assert.equal(sameStringList(first, ['!a', '!b']), true);
  assert.equal(sameStringList(first, first), true);
  assert.equal(sameStringList(first, ['!b', '!a']), false);
  assert.equal(sameStringList(first, ['!a']), false);
});

test('sameNativeRoomListSnapshot ignores a new object with the same unread-bearing fields', () => {
  const snapshot = {
    sessionGeneration: 3,
    orderedRoomIds: ['!a'],
    rooms: [room({ roomId: '!a', unreadCount: 2, name: 'Room' })],
    presentation: EMPTY_ROOM_LIST_PRESENTATION,
  };
  assert.equal(
    sameNativeRoomListSnapshot(snapshot, {
      sessionGeneration: 3,
      orderedRoomIds: ['!a'],
      rooms: [room({ roomId: '!a', unreadCount: 2, name: 'Room' })],
      presentation: { ...EMPTY_ROOM_LIST_PRESENTATION },
    }),
    true
  );
  assert.equal(
    sameNativeRoomListSnapshot(snapshot, {
      sessionGeneration: 3,
      orderedRoomIds: ['!a'],
      rooms: [room({ roomId: '!a', unreadCount: 3, name: 'Room' })],
      presentation: EMPTY_ROOM_LIST_PRESENTATION,
    }),
    false
  );
  assert.equal(
    sameNativeRoomListSnapshot(snapshot, {
      ...snapshot,
      presentation: { ...EMPTY_ROOM_LIST_PRESENTATION, unreadTotal: 2 },
    }),
    false
  );
});

test('room list poll skips atom writes when the snapshot is unchanged', () => {
  const source = readFileSync('src/app/state/room-list/roomList.ts', 'utf8');
  assert.match(source, /if \(sameStringList\(current, action\.rooms\)\) return/);
  assert.match(
    source,
    /if \(sameNativeRoomListSnapshot\(latestNativeRoomListSnapshot, snapshot\)\) return/
  );
});

const wireSnapshot = (sessionGeneration = 3) => ({
  sessionGeneration,
  orderedRoomIds: [],
  rooms: [],
  presentation: EMPTY_ROOM_LIST_PRESENTATION,
});

test('room list reply parser accepts exactly the unchanged and snapshot shapes', () => {
  assert.deepEqual(parseNativeRoomListSnapshotReply({ sessionGeneration: 3, revision: 7 }), {
    kind: 'unchanged',
    sessionGeneration: 3,
    revision: 7,
  });
  const reply = parseNativeRoomListSnapshotReply({ revision: 0, snapshot: wireSnapshot() });
  assert.equal(reply?.kind, 'snapshot');
  assert.equal(reply?.revision, 0);
  assert.equal(reply?.kind === 'snapshot' && reply.snapshot.sessionGeneration, 3);
});

test('room list reply parser fails closed on unknown or malformed shapes', () => {
  const rejected: unknown[] = [
    null,
    undefined,
    7,
    'unchanged',
    [],
    {},
    { revision: 7 },
    { sessionGeneration: 3 },
    { sessionGeneration: 3, revision: -1 },
    { sessionGeneration: 3, revision: 1.5 },
    { sessionGeneration: '3', revision: 7 },
    { sessionGeneration: 3, revision: 7, extra: true },
    { revision: 7, snapshot: wireSnapshot(), sessionGeneration: 3 },
    { revision: 7, snapshot: null },
    { revision: '7', snapshot: wireSnapshot() },
    { revision: 7, snapshot: { ...wireSnapshot(), rooms: [{}] } },
    // A bare snapshot (the old command's shape) is not a reply.
    wireSnapshot(),
  ];
  for (const value of rejected) {
    assert.equal(parseNativeRoomListSnapshotReply(value), null, JSON.stringify(value));
  }
});

test('knownRevision is sent only for a held snapshot of the signed-in session', () => {
  const held = { ...wireSnapshot(3), presentation: EMPTY_ROOM_LIST_PRESENTATION };
  assert.equal(knownRoomListRevision(held, 5, 3), 5);
  assert.equal(knownRoomListRevision(held, 0, 3), 0);
  assert.equal(knownRoomListRevision(held, undefined, 3), undefined);
  assert.equal(knownRoomListRevision(held, 5, 4), undefined);
});

test('room list refresh probes with the held revision and skips work when unchanged', () => {
  const source = readFileSync('src/app/state/room-list/roomList.ts', 'utf8');
  assert.match(source, /'matrix_room_list_snapshot_since',\s*\{ knownRevision \}/);
  assert.doesNotMatch(
    source,
    /invokeDesktopWithAvailability<unknown>\('matrix_room_list_snapshot'\)/
  );
  assert.match(source, /latestNativeRoomListSnapshot,\s*latestNativeRoomListRevision,/);
  assert.match(source, /latestNativeRoomListRevision = reply\.revision;/);
});
