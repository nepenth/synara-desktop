import assert from 'node:assert/strict';
import test from 'node:test';
import type { RoomSummary } from '../../../features/matrix-dto/room';
import type { RoomListPresentation } from '../../../features/matrix-dto/generated';
import { EMPTY_ROOM_LIST_PRESENTATION } from '../../room-list/roomListPresentation';
import { nativeNavigationScope } from '../navigationUnread';

// Core decides which rooms need attention (tested in Rust room_list::presentation).
// These tests feed its rows and check only scoping and aggregation.
const attention = (
  rows: Array<{ roomId: string; total: number; highlight?: number }>
): RoomListPresentation => ({
  ...EMPTY_ROOM_LIST_PRESENTATION,
  unread: rows.map(({ roomId, total, highlight }) => ({
    roomId,
    total,
    highlight: highlight ?? 0,
  })),
});
const none = attention([]);

const room = (overrides: Partial<RoomSummary> & Pick<RoomSummary, 'roomId'>): RoomSummary => ({
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
});

test('Home and DM scope share native classification, including DMs inside spaces', () => {
  const rooms = [
    room({ roomId: '!dm', isDirect: true, unreadCount: 2 }),
    room({ roomId: '!home', unreadCount: 4 }),
    room({ roomId: '!child', unreadCount: 8 }),
    room({ roomId: '!space', isSpace: true, unreadCount: 16 }),
    room({ roomId: '!left-dm', isDirect: true, membership: 'leave', unreadCount: 32 }),
    room({ roomId: '!invite', membership: 'invite', unreadCount: 64 }),
  ];
  const rows = attention([
    { roomId: '!dm', total: 2 },
    { roomId: '!home', total: 4 },
    { roomId: '!child', total: 8 },
  ]);
  const parents = new Map([
    ['!dm', new Set(['!space'])],
    ['!child', new Set(['!space'])],
  ]);
  const dm = nativeNavigationScope(rooms, rows, parents, 'direct');
  assert.deepEqual(dm.roomIds, ['!dm']);
  assert.deepEqual(dm.unread, { total: 2, highlight: 0, from: new Set(['!dm']) });
  const home = nativeNavigationScope(rooms, rows, parents, 'home');
  assert.deepEqual(home.roomIds, ['!home']);
  assert.deepEqual(home.unread, { total: 4, highlight: 0, from: new Set(['!home']) });
});

test('rooms Core gives no attention stay in destinations without adding to the badge', () => {
  const scope = nativeNavigationScope(
    [
      room({ roomId: '!muted', isDirect: true, notificationMode: 'mute', unreadCount: 100 }),
      room({ roomId: '!mention', isDirect: true, highlightCount: 2 }),
      room({ roomId: '!marked', isDirect: true, markedUnread: true }),
      room({ roomId: '!read', isDirect: true }),
    ],
    attention([
      { roomId: '!mention', total: 2, highlight: 2 },
      { roomId: '!marked', total: 1 },
    ]),
    new Map(),
    'direct'
  );
  assert.deepEqual(scope.roomIds, ['!muted', '!mention', '!marked', '!read']);
  assert.deepEqual(scope.unread, {
    total: 3,
    highlight: 2,
    from: new Set(['!mention', '!marked']),
  });
});

test('space rollup rows never count toward a destination badge', () => {
  const rows: RoomListPresentation = {
    ...attention([{ roomId: '!home', total: 1 }]),
  };
  rows.unread.push({ roomId: '!space', total: 9, highlight: 0, fromRoomIds: ['!home'] });
  const home = nativeNavigationScope([room({ roomId: '!home' })], rows, new Map(), 'home');
  assert.equal(home.unread?.total, 1);
});

test('native classification change moves an unread room between destinations and badges', () => {
  const direct = room({ roomId: '!room', isDirect: true, unreadCount: 3 });
  const rows = attention([{ roomId: '!room', total: 3 }]);
  assert.equal(nativeNavigationScope([direct], rows, new Map(), 'direct').unread?.total, 3);
  assert.deepEqual(nativeNavigationScope([direct], rows, new Map(), 'home').roomIds, []);
  const regular = { ...direct, isDirect: false };
  assert.deepEqual(nativeNavigationScope([regular], rows, new Map(), 'direct').roomIds, []);
  assert.equal(nativeNavigationScope([regular], rows, new Map(), 'direct').unread, undefined);
  assert.equal(nativeNavigationScope([regular], rows, new Map(), 'home').unread?.total, 3);
});

test('read receipts and empty signed-out projection clear aggregate counts', () => {
  const dm = room({ roomId: '!dm', isDirect: true });
  assert.equal(
    nativeNavigationScope(
      [dm],
      attention([{ roomId: '!dm', total: 2, highlight: 1 }]),
      new Map(),
      'direct'
    ).unread?.total,
    2
  );
  assert.equal(nativeNavigationScope([dm], none, new Map(), 'direct').unread, undefined);
  for (const scope of ['direct', 'home', 'merged'] as const) {
    assert.deepEqual(nativeNavigationScope([], none, new Map(), scope), {
      roomIds: [],
      unread: undefined,
      attention: 0,
    });
  }
});

test('the merged Home scope is rooms plus DMs with one attention count (NAV-12)', () => {
  const rooms = [
    room({ roomId: '!dm', isDirect: true }),
    room({ roomId: '!dm-mention', isDirect: true }),
    room({ roomId: '!home' }),
    room({ roomId: '!home-mention' }),
    room({ roomId: '!child' }),
    room({ roomId: '!space-dm', isDirect: true }),
  ];
  const rows = attention([
    { roomId: '!dm', total: 2 },
    // Mention-only rows can report no plain total; the DM still counts once.
    { roomId: '!dm-mention', total: 0, highlight: 1 },
    { roomId: '!home', total: 4 },
    { roomId: '!home-mention', total: 5, highlight: 2 },
    { roomId: '!child', total: 8, highlight: 8 },
    { roomId: '!space-dm', total: 3 },
  ]);
  const parents = new Map([
    ['!child', new Set(['!space'])],
    ['!space-dm', new Set(['!space'])],
  ]);
  const merged = nativeNavigationScope(rooms, rows, parents, 'merged');
  assert.deepEqual(merged.roomIds, ['!dm', '!dm-mention', '!home', '!home-mention', '!space-dm']);
  assert.equal(merged.unread?.total, 14);
  // Red count: room mentions (2) plus unread DM messages (2 + 1 + 3); plain
  // room unread (!home) adds nothing and space children stay out.
  assert.equal(merged.attention, 8);
  const home = nativeNavigationScope(rooms, rows, parents, 'home');
  const direct = nativeNavigationScope(rooms, rows, parents, 'direct');
  assert.equal(home.attention, 2);
  assert.equal(direct.attention, 6);
  assert.equal(merged.attention, home.attention + direct.attention);
  assert.deepEqual([...merged.roomIds].sort(), [...home.roomIds, ...direct.roomIds].sort());
});
