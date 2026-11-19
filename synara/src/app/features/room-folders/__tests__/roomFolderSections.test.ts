import assert from 'node:assert/strict';
import test from 'node:test';
import type { RoomFolderPresentation, RoomListPresentation } from '../../matrix-dto/generated';
import { EMPTY_ROOM_LIST_PRESENTATION } from '../../../state/room-list/roomListPresentation';
import {
  adjacentRoomTarget,
  adjacentUnreadTarget,
  roomListNavTargets,
} from '../../desktop-keyboard/roomListNavigation';
import {
  buildRoomFolderSections,
  roomFolderCategoryId,
  roomFolderRoomIdsInView,
  roomFolderUnread,
  roomFoldersSignature,
  roomListNavSections,
  showsEmptyFolderLine,
} from '../roomFolderSections';
import {
  buildHomeRoomListSections,
  HOME_SECTION_CATEGORY_IDS,
  type RoomListSorts,
} from '../../../pages/client/home/homeRoomList';

const folder = (
  id: string,
  name: string,
  roomIds: string[],
  overrides: Partial<RoomFolderPresentation> = {}
): RoomFolderPresentation => ({
  id,
  name,
  roomIds,
  recentOrder: [...roomIds],
  nameOrder: [...roomIds].sort(),
  unreadHighlight: 0,
  unreadTotal: 0,
  unreadRoomIds: [],
  ...overrides,
});

const presentation = (overrides: Partial<RoomListPresentation>): RoomListPresentation => ({
  ...EMPTY_ROOM_LIST_PRESENTATION,
  ...overrides,
});

const recent: RoomListSorts = { favorites: 'recent', rooms: 'recent', directs: 'recent' };
const none = () => false;

// Home: !a !b !c !d plain rooms, !fav a favorite (filed in Work), two DMs.
const homeRooms = ['!a', '!b', '!c', '!d', '!fav'];
const directs = ['!dm1', '!dm2'];
const homePresentation = presentation({
  recentOrder: ['!d', '!c', '!fav', '!b', '!a', '!dm2', '!dm1', '!space-room'],
  nameOrder: ['!a', '!b', '!c', '!d', '!dm1', '!dm2', '!fav', '!space-room'],
  favoriteRoomIds: ['!fav'],
  folders: [
    // Core leaves favorites out of `roomIds`; `!space-room` is not a Home room.
    folder('work', 'Work', ['!b', '!d', '!space-room'], {
      recentOrder: ['!d', '!space-room', '!b'],
      nameOrder: ['!b', '!d', '!space-room'],
    }),
    folder('empty', 'Empty', []),
    folder('spaced', 'Only space rooms', ['!space-room']),
  ],
});

const build = (overrides: Partial<Parameters<typeof buildHomeRoomListSections>[0]> = {}) =>
  buildHomeRoomListSections({
    roomIds: homeRooms,
    directIds: directs,
    presentation: homePresentation,
    sorts: recent,
    closedCategories: new Set(),
    hasUnread: none,
    ...overrides,
  });

test('Home lists Favorites, each folder in order, Rooms, then Direct messages', () => {
  const sections = build();
  assert.deepEqual(
    sections.map((section) => section.id),
    ['favorites', 'folder:work', 'folder:empty', 'folder:spaced', 'rooms', 'directs']
  );
  const byId = Object.fromEntries(sections.map((section) => [section.id, section.roomIds]));
  // A favorite stays in Favorites; its folder does not list it.
  assert.deepEqual(byId.favorites, ['!fav']);
  // Only the rooms Home lists, in Core's folder order for the Rooms sort.
  assert.deepEqual(byId['folder:work'], ['!d', '!b']);
  // Filed rooms leave Rooms.
  assert.deepEqual(byId.rooms, ['!c', '!a']);
  assert.deepEqual(byId.directs, ['!dm2', '!dm1']);
  // Every folder is shown on Home, also with no rooms here.
  assert.deepEqual(byId['folder:empty'], []);
  assert.deepEqual(byId['folder:spaced'], []);
});

test('folders follow the Rooms sort, not their own', () => {
  const sections = build({ sorts: { favorites: 'recent', rooms: 'name', directs: 'recent' } });
  const work = sections.find((section) => section.id === 'folder:work');
  assert.deepEqual(work?.roomIds, ['!b', '!d']);
  assert.deepEqual(sections.find((section) => section.id === 'rooms')?.roomIds, ['!a', '!c']);
});

test('without folders Home is unchanged: Favorites, Rooms, Direct messages', () => {
  const sections = build({ presentation: { ...homePresentation, folders: [] } });
  assert.deepEqual(
    sections.map((section) => [section.id, section.roomIds]),
    [
      ['favorites', ['!fav']],
      ['rooms', ['!d', '!c', '!b', '!a']],
      ['directs', ['!dm2', '!dm1']],
    ]
  );
  assert.deepEqual(
    sections.map((section) => section.categoryId),
    [
      HOME_SECTION_CATEGORY_IDS.favorites,
      HOME_SECTION_CATEGORY_IDS.rooms,
      HOME_SECTION_CATEGORY_IDS.directs,
    ]
  );
  assert.equal(HOME_SECTION_CATEGORY_IDS.rooms, 'home|room');
  assert.equal(HOME_SECTION_CATEGORY_IDS.directs, 'direct|direct');
});

test('a collapsed folder keeps its unread and selected rows, like every section', () => {
  const closed = new Set([roomFolderCategoryId('home', 'work')]);
  assert.equal(roomFolderCategoryId('home', 'work'), 'home|folder:work');
  const sections = build({
    closedCategories: closed,
    hasUnread: (roomId) => roomId === '!b',
    selectedRoomId: '!d',
  });
  const work = sections.find((section) => section.id === 'folder:work');
  assert.equal(work?.closed, true);
  assert.deepEqual(work?.rows, ['!d', '!b']);
  const quiet = build({ closedCategories: closed });
  assert.deepEqual(quiet.find((section) => section.id === 'folder:work')?.rows, []);
  // Its rooms still leave Rooms while it is collapsed.
  assert.deepEqual(quiet.find((section) => section.id === 'rooms')?.roomIds, ['!c', '!a']);
});

test('an open folder with no rooms shows its empty line; a collapsed one only its header', () => {
  const sections = build({ closedCategories: new Set([roomFolderCategoryId('home', 'spaced')]) });
  const byId = Object.fromEntries(sections.map((section) => [section.id, section]));
  assert.equal(showsEmptyFolderLine(byId['folder:empty']), true);
  assert.equal(showsEmptyFolderLine(byId['folder:spaced']), false);
  assert.equal(showsEmptyFolderLine(byId['folder:work']), false);
  // Default sections never have one: they are left out when empty.
  assert.equal(showsEmptyFolderLine(byId.rooms), false);
  assert.equal(
    build({ roomIds: [], presentation: { ...homePresentation, favoriteRoomIds: [] } }).some(
      (section) => section.id === 'rooms' || section.id === 'favorites'
    ),
    false
  );
});

test('previous and next room walk folder sections in display order', () => {
  const revealed: Array<[string, number]> = [];
  const navSections = roomListNavSections(
    build(),
    (id) => `/home/${id}/`,
    (sectionId, index) => revealed.push([sectionId, index])
  );
  const targets = roomListNavTargets({ sections: navSections });
  assert.deepEqual(
    targets.map((target) => target.roomId),
    ['!fav', '!d', '!b', '!c', '!a', '!dm2', '!dm1']
  );
  const step = (selected: string, direction: 'previous' | 'next') =>
    adjacentRoomTarget(targets, selected, direction)?.roomId;
  assert.equal(step('!fav', 'next'), '!d');
  assert.equal(step('!b', 'next'), '!c');
  assert.equal(step('!c', 'previous'), '!b');
  assert.equal(
    adjacentUnreadTarget(targets, '!fav', 'next', (roomId) => roomId === '!a')?.roomId,
    '!a'
  );
  // Each target scrolls its own section's virtual list.
  targets[2].section.reveal?.(targets[2].index);
  assert.deepEqual(revealed, [['folder:work', 1]]);
});

test('in a space: only folders with rooms of that space, in the space order', () => {
  const spacePresentation = presentation({
    folders: [
      folder('work', 'Work', ['!s1', '!other', '!s3'], {
        recentOrder: ['!s3', '!other', '!s1'],
      }),
      folder('elsewhere', 'Elsewhere', ['!other']),
    ],
  });
  const inSpace = new Set(['!s1', '!s2', '!s3']);
  const sections = buildRoomFolderSections({
    presentation: spacePresentation,
    scope: '!space',
    inView: (roomId) => inSpace.has(roomId),
    order: ['!s1', '!s2', '!s3'],
    closedCategories: new Set(),
    hasUnread: none,
    keepEmpty: 'unfiled',
  });
  assert.deepEqual(
    sections.map((section) => [section.id, section.categoryId, section.roomIds]),
    [['folder:work', '!space|folder:work', ['!s1', '!s3']]]
  );
  assert.deepEqual(
    roomFolderRoomIdsInView(spacePresentation.folders[0], (id) => inSpace.has(id), 'recent'),
    ['!s3', '!s1']
  );
  // Position among all folders, for Move up / Move down.
  assert.deepEqual(sections[0].folder, { id: 'work', name: 'Work', index: 0, count: 2 });

  // A folder with no rooms anywhere (just created) shows in the space too.
  const withNew = buildRoomFolderSections({
    presentation: {
      ...spacePresentation,
      folders: [...spacePresentation.folders, folder('new', 'New', [])],
    },
    scope: '!space',
    inView: (roomId) => inSpace.has(roomId),
    order: ['!s1', '!s2', '!s3'],
    closedCategories: new Set(),
    hasUnread: none,
    keepEmpty: 'unfiled',
  });
  assert.deepEqual(
    withNew.map((section) => section.id),
    ['folder:work', 'folder:new']
  );
});

test("a folder's unread is Core's rollup, or the shown rooms' rows in a narrower view", () => {
  const work = folder('work', 'Work', ['!a', '!b'], {
    unreadHighlight: 2,
    unreadTotal: 5,
    unreadRoomIds: ['!a', '!b'],
  });
  const rows = presentation({
    unread: [
      { roomId: '!a', highlight: 2, total: 3 },
      { roomId: '!b', highlight: 0, total: 2 },
      { roomId: '!space', highlight: 9, total: 9, fromRoomIds: ['!a'] },
    ],
  });
  assert.deepEqual(roomFolderUnread(work, ['!a', '!b'], rows), { highlight: 2, total: 5 });
  // A space shows only `!b`: its own row, never a space rollup.
  assert.deepEqual(roomFolderUnread(work, ['!b'], rows), { highlight: 0, total: 2 });
});

test('the folder signature changes with ids and names, not with room activity', () => {
  const base = roomFoldersSignature(homePresentation);
  assert.equal(
    roomFoldersSignature({ ...homePresentation, recentOrder: [...homePresentation.nameOrder] }),
    base
  );
  assert.notEqual(
    roomFoldersSignature({
      ...homePresentation,
      folders: [{ ...homePresentation.folders[0], name: 'Renamed' }],
    }),
    base
  );
});
