import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import {
  favoriteRoomIdSet,
  partitionHomeRooms,
  readHomeRoomSort,
  sortHomeRoomIds,
  writeHomeRoomSort,
} from '../homeRoomList';
import type { RoomSummary } from '../../../../features/matrix-dto/room';

test('home rooms split favorites from remaining rooms', () => {
  const favoriteIds = favoriteRoomIdSet([
    { roomId: '!fav:example.org', isFavorite: true } as RoomSummary,
    { roomId: '!plain:example.org', isFavorite: false } as RoomSummary,
  ]);
  const partition = partitionHomeRooms(
    ['!plain:example.org', '!fav:example.org', '!other:example.org'],
    favoriteIds
  );
  assert.deepEqual(partition.favoriteRoomIds, ['!fav:example.org']);
  assert.deepEqual(partition.remainingRoomIds, ['!plain:example.org', '!other:example.org']);
});

test('home room sort preference defaults to name and persists recent', () => {
  const store = new Map<string, string>();
  const storage = {
    getItem: (key: string) => store.get(key) ?? null,
    setItem: (key: string, value: string) => {
      store.set(key, value);
    },
  };
  assert.equal(readHomeRoomSort(storage), 'name');
  writeHomeRoomSort(storage, 'recent');
  assert.equal(readHomeRoomSort(storage), 'recent');
});

test('home remaining rooms sort by name or recent activity', () => {
  const rooms: Record<string, { name: string; ts: number }> = {
    '!b:example.org': { name: 'Zeta', ts: 30 },
    '!a:example.org': { name: 'Alpha', ts: 10 },
  };
  const mx = {
    getRoom: (roomId: string) => ({
      name: rooms[roomId]?.name,
      getLastActiveTimestamp: () => rooms[roomId]?.ts,
    }),
  } as any;
  const ids = ['!b:example.org', '!a:example.org'];
  assert.deepEqual(sortHomeRoomIds(mx, ids, 'name'), ['!a:example.org', '!b:example.org']);
  assert.deepEqual(sortHomeRoomIds(mx, ids, 'recent'), ['!b:example.org', '!a:example.org']);
});

test('desktop and iOS room lists no longer show a Recent 24h partition', () => {
  const home = readFileSync(join(process.cwd(), 'src/app/pages/client/home/Home.tsx'), 'utf8');
  const ios = readFileSync(
    join(process.cwd(), '../synara-ios/Synara/Features/RoomListView.swift'),
    'utf8'
  );
  assert.equal(home.includes('Recent (24h)'), false);
  assert.equal(home.includes('useRecentRoomPartition'), false);
  assert.equal(home.includes('Favorites'), true);
  assert.equal(home.includes('Sort rooms by name'), true);
  assert.equal(home.includes('Sort rooms by recent activity'), true);
  assert.equal(ios.includes('Recent activity (24h)'), false);
  assert.equal(ios.includes('RoomListRecentActivity.partition'), false);
  assert.equal(ios.includes('Favorites'), true);
  assert.equal(ios.includes('RoomListSortMenu'), true);
});
