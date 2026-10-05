import assert from 'node:assert/strict';
import test from 'node:test';
import type { RoomSummary } from '../../../features/matrix-dto/room';
import { nativeNavigationScope } from '../navigationUnread';

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
  const parents = new Map([
    ['!dm', new Set(['!space'])],
    ['!child', new Set(['!space'])],
  ]);
  const dm = nativeNavigationScope(rooms, parents, 'direct');
  assert.deepEqual(dm.roomIds, ['!dm']);
  assert.deepEqual(dm.unread, { total: 2, highlight: 0, from: new Set(['!dm']) });
  const home = nativeNavigationScope(rooms, parents, 'home');
  assert.deepEqual(home.roomIds, ['!home']);
  assert.deepEqual(home.unread, { total: 4, highlight: 0, from: new Set(['!home']) });
});

test('muted rooms remain in destinations but do not contribute aggregate unread', () => {
  const scope = nativeNavigationScope(
    [
      room({
        roomId: '!muted',
        isDirect: true,
        notificationMode: 'mute',
        unreadCount: 100,
        highlightCount: 10,
      }),
      room({ roomId: '!mention', isDirect: true, unreadCount: 0, highlightCount: 2 }),
      room({ roomId: '!marked', isDirect: true, markedUnread: true }),
      room({ roomId: '!read', isDirect: true }),
    ],
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

test('native classification change moves an unread room between destinations and badges', () => {
  const direct = room({ roomId: '!room', isDirect: true, unreadCount: 3 });
  assert.equal(nativeNavigationScope([direct], new Map(), 'direct').unread?.total, 3);
  assert.deepEqual(nativeNavigationScope([direct], new Map(), 'home').roomIds, []);
  const regular = { ...direct, isDirect: false };
  assert.deepEqual(nativeNavigationScope([regular], new Map(), 'direct').roomIds, []);
  assert.equal(nativeNavigationScope([regular], new Map(), 'direct').unread, undefined);
  assert.equal(nativeNavigationScope([regular], new Map(), 'home').unread?.total, 3);
});

test('read receipts and empty signed-out projection clear aggregate counts', () => {
  const before = room({
    roomId: '!dm',
    isDirect: true,
    unreadCount: 2,
    highlightCount: 1,
    markedUnread: true,
  });
  assert.equal(nativeNavigationScope([before], new Map(), 'direct').unread?.total, 2);
  assert.equal(
    nativeNavigationScope(
      [{ ...before, unreadCount: 0, highlightCount: 0, markedUnread: false }],
      new Map(),
      'direct'
    ).unread,
    undefined
  );
  assert.deepEqual(nativeNavigationScope([], new Map(), 'direct'), {
    roomIds: [],
    unread: undefined,
  });
  assert.deepEqual(nativeNavigationScope([], new Map(), 'home'), {
    roomIds: [],
    unread: undefined,
  });
});
