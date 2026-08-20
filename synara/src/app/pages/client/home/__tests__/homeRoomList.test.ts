import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import {
  DEFAULT_ROOM_LIST_SORT,
  ROOM_LIST_SORT_STORAGE_KEY,
  favoriteRoomIdSet,
  partitionHomeRooms,
  readRoomListSort,
  roomListSortStorageKey,
  sortHomeRoomIds,
  writeRoomListSort,
} from '../homeRoomList';
import type { RoomSummary } from '../../../../features/matrix-dto/room';

const room = (overrides: Partial<RoomSummary> & Pick<RoomSummary, 'roomId'>): RoomSummary =>
  ({
    membership: 'join',
    isDirect: false,
    isSpace: false,
    isCall: false,
    isFavorite: false,
    isEncrypted: false,
    unreadCount: 0,
    highlightCount: 0,
    markedUnread: false,
    ...overrides,
  } as RoomSummary);

test('home rooms split favorites from remaining rooms', () => {
  const favoriteIds = favoriteRoomIdSet([
    room({ roomId: '!fav:example.org', isFavorite: true }),
    room({ roomId: '!plain:example.org', isFavorite: false }),
  ]);
  const partition = partitionHomeRooms(
    ['!plain:example.org', '!fav:example.org', '!other:example.org'],
    favoriteIds
  );
  assert.deepEqual(partition.favoriteRoomIds, ['!fav:example.org']);
  assert.deepEqual(partition.remainingRoomIds, ['!plain:example.org', '!other:example.org']);
});

test('room list sort preference defaults to recent and persists name as device chrome', () => {
  const store = new Map<string, string>();
  const storage = {
    getItem: (key: string) => store.get(key) ?? null,
    setItem: (key: string, value: string) => {
      store.set(key, value);
    },
  };
  assert.equal(ROOM_LIST_SORT_STORAGE_KEY, 'synara.roomListSort');
  assert.equal(roomListSortStorageKey('favorites'), 'synara.roomListSort.favorites');
  assert.equal(DEFAULT_ROOM_LIST_SORT, 'recent');
  assert.equal(readRoomListSort(storage), 'recent');
  writeRoomListSort(storage, 'name');
  assert.equal(readRoomListSort(storage), 'name');
});

test('favorites and rooms persist independent sort orders and fall back to the legacy key', () => {
  const store = new Map<string, string>();
  const storage = {
    getItem: (key: string) => store.get(key) ?? null,
    setItem: (key: string, value: string) => {
      store.set(key, value);
    },
  };
  writeRoomListSort(storage, 'name', 'favorites');
  assert.equal(readRoomListSort(storage, 'favorites'), 'name');
  assert.equal(readRoomListSort(storage, 'rooms'), 'recent');
  writeRoomListSort(storage, 'name', 'rooms');
  assert.equal(readRoomListSort(storage, 'favorites'), 'name');
  assert.equal(readRoomListSort(storage, 'rooms'), 'name');
  writeRoomListSort(storage, 'recent', 'favorites');
  assert.equal(readRoomListSort(storage, 'favorites'), 'recent');
  assert.equal(readRoomListSort(storage, 'rooms'), 'name');

  const legacy = new Map<string, string>([[ROOM_LIST_SORT_STORAGE_KEY, 'name']]);
  const legacyStorage = {
    getItem: (key: string) => legacy.get(key) ?? null,
    setItem: (key: string, value: string) => {
      legacy.set(key, value);
    },
  };
  assert.equal(readRoomListSort(legacyStorage, 'favorites'), 'name');
  assert.equal(readRoomListSort(legacyStorage, 'rooms'), 'name');
});

test('home rooms sort by native lastActivityTs and leave missing timestamps last', () => {
  const rooms = [
    room({
      roomId: '!encrypted:example.org',
      name: 'Encrypted',
      isEncrypted: true,
      lastActivityTs: 40,
    }),
    room({ roomId: '!old:example.org', name: 'Old', lastActivityTs: 10 }),
    room({ roomId: '!none:example.org', name: 'None' }),
    room({ roomId: '!alpha:example.org', name: 'Alpha', lastActivityTs: 20 }),
  ];
  const ids = rooms.map((item) => item.roomId);
  assert.deepEqual(sortHomeRoomIds(ids, rooms, 'recent'), [
    '!encrypted:example.org',
    '!alpha:example.org',
    '!old:example.org',
    '!none:example.org',
  ]);
  assert.deepEqual(sortHomeRoomIds(ids, rooms, 'name'), [
    '!alpha:example.org',
    '!encrypted:example.org',
    '!none:example.org',
    '!old:example.org',
  ]);
});

test('desktop and iOS room lists no longer implement a Recent 24h partition', () => {
  const cwd = process.cwd();
  const home = readFileSync(join(cwd, 'src/app/pages/client/home/Home.tsx'), 'utf8');
  const hook = readFileSync(join(cwd, 'src/app/hooks/useRoomActivity.ts'), 'utf8');
  const activity = readFileSync(join(cwd, 'src/app/state/room-list/roomActivity.ts'), 'utf8');
  const activityTests = readFileSync(
    join(cwd, 'src/app/state/room-list/__tests__/roomActivity.test.ts'),
    'utf8'
  );
  const iosView = readFileSync(
    join(cwd, '../synara-ios/Synara/Features/RoomListView.swift'),
    'utf8'
  );
  const iosService = readFileSync(
    join(cwd, '../synara-ios/Synara/Services/RoomListService.swift'),
    'utf8'
  );
  const contract = readFileSync(
    join(cwd, '../docs/timeline-room-state-reliability-contract.md'),
    'utf8'
  );
  const coreSort = readFileSync(
    join(cwd, '../crates/synara-core/src/app/room_list/sort.rs'),
    'utf8'
  );

  assert.equal(home.includes('Recent (24h)'), false);
  assert.equal(home.includes('useRecentRoomPartition'), false);
  assert.equal(hook.includes('useRecentRoomPartition'), false);
  assert.equal(activity.includes('RECENT_ROOM_WINDOW_MS'), false);
  assert.equal(activity.includes('partitionRoomIdsByActivity'), false);
  assert.equal(activityTests.includes('partitionRoomIdsByActivity'), false);
  assert.equal(iosView.includes('Recent activity (24h)'), false);
  assert.equal(iosService.includes('enum RoomListRecentActivity'), false);
  assert.equal(iosService.includes('TimeInterval = 86400'), false);
  assert.equal(coreSort.includes('fn recent_joined_rooms'), false);
  assert.equal(contract.includes('24-hour cutoff'), false);
  assert.equal(home.includes('by recent activity'), true);
  assert.equal(home.includes('by name'), true);
  assert.equal(home.includes('css.SortIconButton'), true);
  assert.equal(home.includes('handleFavoriteSort'), true);
  assert.equal(home.includes('handleRoomsSort'), true);
  assert.equal(iosView.includes('RoomListSortMenu'), true);
  assert.equal(iosService.includes('synara.roomListSort'), true);
});

test('Rooms section title is title-case like Favorites, not overline uppercase', () => {
  const cwd = process.cwd();
  const home = readFileSync(join(cwd, 'src/app/pages/client/home/Home.tsx'), 'utf8');
  const category = readFileSync(
    join(cwd, 'src/app/features/room-nav/RoomNavCategoryButton.tsx'),
    'utf8'
  );
  assert.match(home, />\s*Rooms\s*</);
  assert.match(category, /size="B300"/);
  assert.equal(category.includes('size="O400"'), false);
});
