// Room folders (`in.synara.room_folders`) on the production Home and Space
// panes: 60 Home rooms, 3 DMs and a "Design" space with 6 rooms, filed into
// "Work" (Home and Design rooms, and a favorite), "Side projects" (with a
// mention) and an empty "Reading list". Native IPC is synthetic: the room
// list (its presentation mirrors Core's folder rule, `presentation.ts`), the
// space graph and fake `matrix_room_folders_*` commands that validate like
// Core and wake the room list after each write.
//
// Query flags: `light`, `readOnly` (a newer client's format), `maxFolders=N`,
// `tooLarge` (create and file fail with content-too-large), `manyFiled` (300
// more rooms filed in an "Archive" folder), `space` (open the Design space).
import React, { useCallback, useLayoutEffect, useState } from 'react';
import { Provider } from 'jotai';
import { enableMapSet } from 'immer';
import { MemoryRouter, Outlet, Route, Routes, useLocation, useParams } from 'react-router-dom';
import { configClass, varsClass } from 'folds';
import { darkTheme, synaraLightTheme as lightTheme } from '../../src/colors.css';
import { presentationFor, withFolderIds, type FixtureRoomFolder } from './presentation';
import { ScreenSize, ScreenSizeProvider } from '../../src/app/hooks/useScreenSize';
import { MobileFriendlyPageNav } from '../../src/app/pages/MobileFriendly';
import { NavToActivePathProvider } from '../../src/app/state/hooks/navToActivePath';
import { makeNavToActivePathAtom } from '../../src/app/state/navToActivePath';
import {
  createNativeSession,
  getNativeRoom,
  setNativeSessionForTests,
} from '../../src/app/native/nativeSession';
import { useBindAllRoomsAtom } from '../../src/app/state/room-list/roomList';
import { useBindMDirectAtom } from '../../src/app/state/mDirectList';
import { useBindRoomToUnreadAtom } from '../../src/app/state/room/roomToUnread';
import { useBindRoomToParentsAtom } from '../../src/app/state/room/roomToParents';
import type { RoomSummary } from '../../src/app/features/matrix-dto/room';
import { Home } from '../../src/app/pages/client/home/Home';
import { Space } from '../../src/app/pages/client/space/Space';
import { SpaceProvider } from '../../src/app/hooks/useSpace';
import { ClosedNavCategoriesProvider } from '../../src/app/state/hooks/closedNavCategories';
import { makeClosedNavCategoriesAtom } from '../../src/app/state/closedNavCategories';
import { RoomsNotificationPreferencesProvider } from '../../src/app/hooks/useRoomsNotificationPreferences';
import { Page, PageHeader, PageRoot } from '../../src/app/components/page';
import { ApprovalInboxProvider } from '../../src/app/features/approvals/ApprovalInboxProvider';
import { OpenedSidebarFolderProvider } from '../../src/app/state/hooks/openedSidebarFolder';
import { makeOpenedSidebarFolderAtom } from '../../src/app/state/openedSidebarFolder';
import { NavBottomBarHost } from '../../src/app/features/nav-pane/NavBottomBarSlot';
import { DesktopAppShortcuts } from '../../src/app/features/desktop-keyboard/DesktopAppShortcuts';

const params = new URLSearchParams(location.search);
const light = params.has('light');
const readOnly = params.has('readOnly');
const tooLarge = params.has('tooLarge');
const maxFolders = Number(params.get('maxFolders') ?? 100);

const summary = (roomId: string, name: string, change: Partial<RoomSummary> = {}): RoomSummary => ({
  roomId,
  name,
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
  joinRule: 'public',
  ...change,
});

const NAMES = [
  'general',
  'random',
  'announcements',
  'standup',
  'design-reviews',
  'release-train',
  'incidents',
  'oncall',
  'frontend',
  'backend',
  'infra',
  'security',
  'hiring',
  'onboarding',
  'docs',
  'support',
  'sales',
  'marketing',
  'finance',
  'legal',
];
const id = (slug: string) => `!${slug}:example.test`;
const now = Date.UTC(2026, 10, 19, 12, 0, 0);
// Recent order is list order: room 0 is the most recent.
const homeRooms = Array.from({ length: 60 }, (_, index) => {
  const base = NAMES[index % NAMES.length];
  const name = index < NAMES.length ? base : `${base}-${Math.floor(index / NAMES.length) + 1}`;
  return summary(id(name), name, { lastActivityTs: now - index * 60_000 });
});
const update = (roomId: string, change: Partial<RoomSummary>) => {
  rooms = rooms.map((r) => (r.roomId === roomId ? { ...r, ...change } : r));
};
const dms = [
  summary(id('dm-alice'), 'Alice', {
    isDirect: true,
    directUserId: '@alice:example.test',
    unreadCount: 2,
    lastActivityTs: now - 5_000,
  }),
  summary(id('dm-bob'), 'Bob', { isDirect: true, directUserId: '@bob:example.test' }),
  summary(id('dm-carol'), 'Carol', { isDirect: true, directUserId: '@carol:example.test' }),
];
const SPACE = id('space-design');
const spaceRooms = ['design-system', 'brand', 'ux-research', 'icons', 'motion', 'critique'].map(
  (name, index) => summary(id(name), name, { lastActivityTs: now - (index * 7 + 3) * 60_000 })
);
const archive = params.has('manyFiled')
  ? Array.from({ length: 300 }, (_, index) =>
      summary(id(`archive-${String(index).padStart(3, '0')}`), `archive-${index}`, {
        lastActivityTs: now - (100 + index) * 60_000,
      })
    )
  : [];

let rooms: RoomSummary[] = [
  ...homeRooms,
  ...archive,
  ...dms,
  ...spaceRooms,
  summary(SPACE, 'Design', { isSpace: true }),
];
// Favorites: one is filed in Work (favorite wins), one is not filed.
update(id('announcements'), { isFavorite: true });
update(id('standup'), { isFavorite: true });
// Attention across the list.
update(id('general'), { unreadCount: 3 });
update(id('incidents'), { unreadCount: 4, highlightCount: 1 });
update(id('docs-2'), { unreadCount: 6, highlightCount: 2 });
update(id('support-2'), { unreadCount: 1 });
update(id('brand'), { unreadCount: 2 });

let folders: FixtureRoomFolder[] = [
  {
    id: 'work',
    name: 'Work',
    rooms: [
      'announcements',
      'release-train',
      'incidents',
      'oncall',
      'frontend',
      'backend',
      'infra',
      'security',
      'design-system',
      'ux-research',
    ].map(id),
  },
  {
    id: 'side',
    name: 'Side projects',
    rooms: ['docs-2', 'support-2', 'sales-2', 'marketing-2', 'finance-2', 'legal-2'].map(id),
  },
  { id: 'reading', name: 'Reading list', rooms: [] },
  ...(archive.length
    ? [{ id: 'archive', name: 'Archive', rooms: archive.map((r) => r.roomId) }]
    : []),
];

const session = {
  status: 'logged_in',
  sessionGeneration: 1,
  user_id: '@reader:example.test',
  device_id: 'TEST',
  homeserver_url: 'https://example.test',
};

let revision = 1;
const commands: Array<{ command: string; args?: Record<string, unknown> }> = [];
const record = (command: string, args?: Record<string, unknown>) => {
  commands.push({ command, args });
  document.body.dataset.folderCommands = JSON.stringify(commands);
};

const folderSnapshot = () => ({
  sessionGeneration: 1,
  version: readOnly ? 2 : 1,
  readOnly,
  folders: folders.map((folder) => ({ id: folder.id, name: folder.name, roomIds: folder.rooms })),
});

const rejection = (id: string) => ({
  code: 'InvalidRequest',
  message: `fixture ${id}`,
  diagnosticId: `v-room-folders.${id}`,
});

/** Core's name rule (`room_folders.rs`). */
const folderName = (value: unknown): string => {
  const name = String(value ?? '')
    .trim()
    .replace(/\s+/g, ' ');
  const length = [...name].length;
  if (length < 1 || length > 64) throw rejection('invalid-name');
  return name;
};

const fileable = (roomId: string) => {
  const room = rooms.find((r) => r.roomId === roomId);
  if (!room || room.membership !== 'join') throw rejection('room-not-joined');
  if (room.isDirect) throw rejection('room-is-direct');
  if (room.isSpace) throw rejection('room-is-space');
};

let emit: ((payload: unknown) => void) | undefined;
/** Core wakes the room-list owner right after a local write. */
const wake = () => {
  revision += 1;
  emit?.({ sessionGeneration: 1, revision });
};

const folderCommand = (command: string, args: Record<string, unknown> = {}) => {
  if (command === 'matrix_room_folders_snapshot') return folderSnapshot();
  record(command, args);
  if (readOnly) throw rejection('unsupported-version');
  const folderId = typeof args.folderId === 'string' ? args.folderId : null;
  if (command === 'matrix_room_folders_create') {
    const name = folderName(args.name);
    if (folders.length >= maxFolders) throw rejection('too-many-folders');
    if (tooLarge) throw rejection('content-too-large');
    const roomId = typeof args.roomId === 'string' ? args.roomId : undefined;
    if (roomId) fileable(roomId);
    const created = `folder-${folders.length + 1}`;
    folders = [
      ...folders.map((folder) => ({
        ...folder,
        rooms: folder.rooms.filter((r) => r !== roomId),
      })),
      { id: created, name, rooms: roomId ? [roomId] : [] },
    ];
    wake();
    return { folderId: created, snapshot: folderSnapshot() };
  }
  if (command === 'matrix_room_folders_rename') {
    const name = folderName(args.name);
    if (!folders.some((folder) => folder.id === folderId)) throw rejection('folder-not-found');
    folders = folders.map((folder) => (folder.id === folderId ? { ...folder, name } : folder));
  } else if (command === 'matrix_room_folders_delete') {
    folders = folders.filter((folder) => folder.id !== folderId);
  } else if (command === 'matrix_room_folders_move') {
    const from = folders.findIndex((folder) => folder.id === folderId);
    if (from < 0) throw rejection('folder-not-found');
    const next = [...folders];
    const [moved] = next.splice(from, 1);
    next.splice(Math.min(Number(args.toIndex), next.length), 0, moved);
    folders = next;
  } else if (command === 'matrix_room_folders_set_room') {
    const roomId = String(args.roomId);
    if (folderId !== null) {
      fileable(roomId);
      if (!folders.some((folder) => folder.id === folderId)) throw rejection('folder-not-found');
      if (tooLarge) throw rejection('content-too-large');
    }
    folders = folders.map((folder) => {
      if (folder.id === folderId) {
        return folder.rooms.includes(roomId)
          ? folder
          : { ...folder, rooms: [...folder.rooms, roomId] };
      }
      return { ...folder, rooms: folder.rooms.filter((r) => r !== roomId) };
    });
  } else {
    throw new Error(`Unexpected folder command ${command}`);
  }
  wake();
  return folderSnapshot();
};

const unknown: string[] = [];
const invoke = async (command: string, args?: Record<string, unknown>) => {
  if (command === 'matrix_session_snapshot') return session;
  if (command.startsWith('matrix_room_folders_')) return folderCommand(command, args);
  if (command === 'matrix_room_list_snapshot' || command === 'matrix_room_list_snapshot_since') {
    const listed = withFolderIds(rooms, folders);
    const snapshot = {
      sessionGeneration: 1,
      orderedRoomIds: listed.map((r) => r.roomId),
      rooms: listed,
      presentation: presentationFor(listed, folders),
    };
    return command === 'matrix_room_list_snapshot' ? snapshot : { revision, snapshot };
  }
  if (command === 'matrix_mdirect_snapshot') {
    return { sessionGeneration: 1, roomIds: dms.map((d) => d.roomId), userIds: [] };
  }
  if (command === 'matrix_space_parents_snapshot') {
    return {
      sessionGeneration: 1,
      entries: spaceRooms.map((r) => ({ roomId: r.roomId, parentIds: [SPACE] })),
    };
  }
  if (command === 'matrix_space_children_snapshot') {
    return {
      sessionGeneration: 1,
      edges: spaceRooms.map((r, index) => ({
        parentId: SPACE,
        childId: r.roomId,
        order: String(index).padStart(2, '0'),
        suggested: false,
        via: ['example.test'],
        originServerTs: now - index,
      })),
    };
  }
  if (command === 'matrix_agent_approvals_list') {
    return {
      sessionGeneration: 1,
      coverageWindowMs: 300000,
      coverage: 'discovery',
      items: [],
      loading: false,
      incomplete: false,
    };
  }
  if (command === 'matrix_account_data_get') return { eventType: args?.eventType, content: null };
  if (command === 'matrix_room_set_favorite') {
    update(String(args?.roomId), { isFavorite: args?.favorite === true });
    wake();
    return { status: 'ok' };
  }
  unknown.push(command);
  document.body.dataset.unknownCommands = JSON.stringify([...new Set(unknown)]);
  throw new Error(`Unexpected fixture command ${command}`);
};

const fixtureSession = createNativeSession(async (command) => ({
  available: true,
  value: await invoke(command),
}));
// Every harness fixture module loads; only the one on the page installs its bridge.
if (params.has('roomFolders')) {
  window.__SYNARA_DESKTOP__ = {
    platform: 'tauri',
    invoke: async <T,>(cmd: string, args?: Record<string, unknown>) =>
      invoke(cmd, args) as Promise<T>,
  };
  // `listen` goes through the Tauri internals: keep the room-list stream's
  // handler, so a folder write wakes the list as Core does.
  const callbacks = new Map<number, (event: unknown) => void>();
  let next = 0;
  Object.assign(window, {
    __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => undefined },
    __TAURI_INTERNALS__: {
      transformCallback: (handler: (event: unknown) => void) => {
        next += 1;
        callbacks.set(next, handler);
        return next;
      },
      invoke: async (cmd: string, args?: Record<string, unknown>) => {
        if (cmd === 'plugin:event|listen') {
          const handlerId = args?.handler as number;
          if (args?.event === 'matrix-room-list-updated') {
            emit = (payload) =>
              callbacks.get(handlerId)?.({ event: 'matrix-room-list-updated', id: 0, payload });
          }
          return handlerId;
        }
        if (cmd === 'plugin:event|unlisten') return null;
        return invoke(cmd, args);
      },
    },
  });
  setNativeSessionForTests(fixtureSession);
  enableMapSet();
  // The app themes <body> (ThemeManager), which portalled menus and dialogs
  // inherit. Another fixture module darkens <body> as it loads, so this runs
  // once every module has (`RoomFoldersFixture`'s first layout).

  Object.assign(window, {
    __roomFolders: {
      receive: (slug: string, change: Partial<RoomSummary>) => {
        update(id(slug), change);
        wake();
      },
      folders: () => folders,
    },
  });
}

const nav = makeNavToActivePathAtom('@reader:example.test');
const closedCategories = makeClosedNavCategoriesAtom('@reader:example.test');
const openedFolders = makeOpenedSidebarFolderAtom('@reader:example.test');
const notificationPreferences = {
  mute: new Set<string>(),
  specialMessages: new Set<string>(),
  allMessages: new Set<string>(),
};

function ContentPage() {
  const location = useLocation();
  return (
    <Page>
      <PageHeader>
        <output data-testid="current-route">{location.pathname}</output>
      </PageHeader>
      <div style={{ padding: 16 }}>
        <textarea aria-label="Composer" rows={2} />
      </div>
    </Page>
  );
}

function SpaceList() {
  const { spaceIdOrAlias } = useParams();
  const space = getNativeRoom(decodeURIComponent(spaceIdOrAlias ?? ''));
  if (!space) return <p data-testid="space-missing">No space</p>;
  return (
    <SpaceProvider value={space}>
      <Space />
    </SpaceProvider>
  );
}

/**
 * One pane element across the list's routes (the app's pathless layout
 * route), so the selected room's param reaches it and the list stays mounted.
 */
function HomePaneRoutes() {
  const [pane] = useState(() => (
    <MobileFriendlyPageNav path={['/home/', '/direct/']}>
      <Home />
    </MobileFriendlyPageNav>
  ));
  return (
    <Routes>
      <Route element={pane}>
        <Route path="/home/" element={null} />
        <Route path="/home/:roomIdOrAlias/*" element={null} />
        <Route path="/direct/" element={null} />
        <Route path="/direct/:roomIdOrAlias/*" element={null} />
      </Route>
    </Routes>
  );
}

function SpacePaneRoutes() {
  const [pane] = useState(() => <Outlet />);
  return (
    <Routes>
      <Route element={pane}>
        <Route path="/:spaceIdOrAlias/" element={<SpaceList />} />
        <Route path="/:spaceIdOrAlias/:roomIdOrAlias/*" element={<SpaceList />} />
      </Route>
    </Routes>
  );
}

function Panes() {
  return (
    <ClosedNavCategoriesProvider value={closedCategories}>
      <RoomsNotificationPreferencesProvider value={notificationPreferences}>
        <Outlet />
      </RoomsNotificationPreferencesProvider>
    </ClosedNavCategoriesProvider>
  );
}

function Client() {
  useBindAllRoomsAtom(
    useCallback((snapshot) => fixtureSession.applyRoomListSnapshot(snapshot), []),
    useCallback((snapshot) => fixtureSession.applySessionSnapshot(snapshot), [])
  );
  useBindMDirectAtom();
  useBindRoomToParentsAtom();
  useBindRoomToUnreadAtom();
  return (
    <main
      className={`${configClass} ${varsClass} ${light ? lightTheme : `${darkTheme} dark-theme`}`}
      style={{ display: 'flex', height: '100vh' }}
    >
      <Routes>
        <Route element={<Panes />}>
          <Route
            element={
              <PageRoot collapsibleNav nav={<HomePaneRoutes />}>
                <Outlet />
              </PageRoot>
            }
          >
            <Route path="/home/*" element={<ContentPage />} />
            <Route path="/direct/*" element={<ContentPage />} />
          </Route>
          <Route
            element={
              <PageRoot collapsibleNav nav={<SpacePaneRoutes />}>
                <Outlet />
              </PageRoot>
            }
          >
            <Route path="/:spaceIdOrAlias/*" element={<ContentPage />} />
          </Route>
        </Route>
      </Routes>
      <DesktopAppShortcuts />
      <NavBottomBarHost />
    </main>
  );
}

export function RoomFoldersFixture() {
  useLayoutEffect(() => {
    document.body.classList.remove(darkTheme, lightTheme, 'dark-theme');
    document.body.classList.add(
      configClass,
      varsClass,
      light ? lightTheme : darkTheme,
      ...(light ? [] : ['dark-theme'])
    );
  }, []);
  return (
    <Provider>
      <MemoryRouter
        initialEntries={[params.has('space') ? `/${encodeURIComponent(SPACE)}/` : '/home/']}
      >
        <ScreenSizeProvider value={ScreenSize.Desktop}>
          <NavToActivePathProvider value={nav}>
            <OpenedSidebarFolderProvider value={openedFolders}>
              <ApprovalInboxProvider>
                <Client />
              </ApprovalInboxProvider>
            </OpenedSidebarFolderProvider>
          </NavToActivePathProvider>
        </ScreenSizeProvider>
      </MemoryRouter>
    </Provider>
  );
}
