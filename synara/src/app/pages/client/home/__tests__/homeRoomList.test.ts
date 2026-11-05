import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
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
import type { RoomListPresentation } from '../../../../features/matrix-dto/generated';
import { EMPTY_ROOM_LIST_PRESENTATION } from '../../../../state/room-list/roomListPresentation';

const presentation = (overrides: Partial<RoomListPresentation>): RoomListPresentation => ({
  ...EMPTY_ROOM_LIST_PRESENTATION,
  ...overrides,
});

test('home rooms split favorites from remaining rooms using Core favorites', () => {
  const favoriteIds = favoriteRoomIdSet(presentation({ favoriteRoomIds: ['!fav:example.org'] }));
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

test('home rooms follow Core order and keep rooms Core did not list last', () => {
  const order = presentation({
    recentOrder: ['!encrypted:example.org', '!alpha:example.org', '!old:example.org'],
    nameOrder: ['!alpha:example.org', '!encrypted:example.org', '!old:example.org'],
  });
  const ids = [
    '!late:example.org',
    '!old:example.org',
    '!alpha:example.org',
    '!encrypted:example.org',
  ];
  assert.deepEqual(sortHomeRoomIds(ids, order, 'recent'), [
    '!encrypted:example.org',
    '!alpha:example.org',
    '!old:example.org',
    '!late:example.org',
  ]);
  assert.deepEqual(sortHomeRoomIds(ids, order, 'name'), [
    '!alpha:example.org',
    '!encrypted:example.org',
    '!old:example.org',
    '!late:example.org',
  ]);
});

test('the renderer does not re-implement room ordering rules', () => {
  const source = readFileSync(
    join(process.cwd(), 'src/app/pages/client/home/homeRoomList.ts'),
    'utf8'
  );
  assert.doesNotMatch(source, /lastActivityTs|localeCompare|toLocaleLowerCase/);
});

test('desktop and iOS room lists no longer implement a Recent 24h partition', () => {
  const cwd = process.cwd();
  const home = readFileSync(join(cwd, 'src/app/pages/client/home/Home.tsx'), 'utf8');
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
  // The legacy js-sdk room-activity store and its retired hook are deleted.
  for (const retired of [
    'src/app/hooks/useRoomActivity.ts',
    'src/app/state/room-list/roomActivity.ts',
    'src/app/state/room-list/__tests__/roomActivity.test.ts',
  ]) {
    assert.equal(existsSync(join(cwd, retired)), false, retired);
  }
  assert.equal(iosView.includes('Recent activity (24h)'), false);
  assert.equal(iosService.includes('enum RoomListRecentActivity'), false);
  assert.equal(iosService.includes('TimeInterval = 86400'), false);
  assert.equal(coreSort.includes('fn recent_joined_rooms'), false);
  assert.equal(contract.includes('24-hour cutoff'), false);
  // One shared sort menu (Recent activity / Name) per section header.
  assert.equal(home.includes('<RoomListSortMenu'), true);
  assert.equal(home.includes("value: 'recent' as const"), true);
  assert.equal(home.includes("value: 'name' as const"), true);
  assert.equal(home.includes('handleFavoriteSort'), true);
  assert.equal(home.includes('handleRoomsSort'), true);
  assert.equal(iosView.includes('RoomListSortMenu'), true);
  assert.equal(iosService.includes('synara.roomListSort'), true);
});

test('section headers are full-width 28px rows with 12px semibold caption labels', () => {
  const cwd = process.cwd();
  const home = readFileSync(join(cwd, 'src/app/pages/client/home/Home.tsx'), 'utf8');
  const header = readFileSync(join(cwd, 'src/app/components/nav/NavSectionHeader.tsx'), 'utf8');
  const navCss = readFileSync(join(cwd, 'src/app/components/nav/styles.css.ts'), 'utf8');
  assert.match(home, /t\('modernization\.nav\.rooms', 'Rooms'\)/);
  assert.match(home, /t\('modernization\.nav\.favorites', 'Favorites'\)/);
  // Favorites collapses like every other section.
  assert.match(home, /FAVORITES_CATEGORY_ID = makeNavCategoryId\('home', 'favorites'\)/);
  assert.match(header, /aria-expanded=\{!closed\}/);
  assert.match(navCss, /NavSectionHeader = style\(\{[\s\S]*?minHeight: toRem\(28\)/);
  // Wave 3 caption: 12px semibold, secondary, 0.02em tracking, chevron always shown.
  assert.match(
    navCss,
    /NavSectionLabel = style\(\{\s*fontSize: toRem\(12\),\s*lineHeight: toRem\(16\),\s*fontWeight: config\.fontWeight\.W600,\s*letterSpacing: '0\.02em'/
  );
  assert.doesNotMatch(navCss, /NavSectionChevron = style\(\{[^}]*opacity: 0/);
  assert.equal(navCss.includes('O400'), false);
  assert.equal(existsSync(join(cwd, 'src/app/features/room-nav/RoomNavCategoryButton.tsx')), false);
});
