// The list column's frame with production parts: the Home pane, its header
// with the space switcher, and the bottom bar, on an account with 60 Home
// rooms, three DMs and twelve spaces (two in a "Work" folder). Native IPC is
// synthetic: the room list, the space order in `in.synara.spaces` (whose save
// can be made to fail), mark-read and one pending approval. A space route
// shows a minimal pane with the same header and bar.
import React, { useCallback, useState } from 'react';
import { Provider } from 'jotai';
import { enableMapSet } from 'immer';
import { MemoryRouter, Outlet, Route, Routes, useLocation, useParams } from 'react-router-dom';
import { configClass, varsClass } from 'folds';
import { darkTheme } from '../../src/colors.css';
import { presentationFor } from './presentation';
import { ScreenSize, ScreenSizeProvider } from '../../src/app/hooks/useScreenSize';
import { NavToActivePathProvider } from '../../src/app/state/hooks/navToActivePath';
import { makeNavToActivePathAtom } from '../../src/app/state/navToActivePath';
import { createNativeSession, setNativeSessionForTests } from '../../src/app/native/nativeSession';
import { getNativeRoom } from '../../src/app/native/nativeSession';
import { useBindAllRoomsAtom } from '../../src/app/state/room-list/roomList';
import { useBindMDirectAtom } from '../../src/app/state/mDirectList';
import { useBindRoomToUnreadAtom } from '../../src/app/state/room/roomToUnread';
import { useBindRoomToParentsAtom } from '../../src/app/state/room/roomToParents';
import type { RoomSummary } from '../../src/app/features/matrix-dto/room';
import { Home } from '../../src/app/pages/client/home/Home';
import { ClosedNavCategoriesProvider } from '../../src/app/state/hooks/closedNavCategories';
import { makeClosedNavCategoriesAtom } from '../../src/app/state/closedNavCategories';
import { RoomsNotificationPreferencesProvider } from '../../src/app/hooks/useRoomsNotificationPreferences';
import { Page, PageHeader, PageNav, PageRoot } from '../../src/app/components/page';
import { ShowRoomListButton } from '../../src/app/features/room-list-pane/RoomListToggleButton';
import { ApprovalInboxProvider } from '../../src/app/features/approvals/ApprovalInboxProvider';
import { OpenedSidebarFolderProvider } from '../../src/app/state/hooks/openedSidebarFolder';
import { makeOpenedSidebarFolderAtom } from '../../src/app/state/openedSidebarFolder';
import { NavPaneHeader } from '../../src/app/features/nav-pane/NavPaneHeader';
import { NavBottomBar } from '../../src/app/features/nav-pane/NavBottomBar';
import { DesktopAppShortcuts } from '../../src/app/features/desktop-keyboard/DesktopAppShortcuts';

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
  ...change,
});

const SPACE_NAMES = [
  'Design',
  'Engineering',
  'Finance',
  'Garden',
  'Hiking',
  'Infra',
  'Jazz',
  'Kitchen',
  'Library',
  'Music',
  'Café club',
  'Zeta',
];
const spaceId = (name: string) =>
  `!space-${name
    .toLowerCase()
    .normalize('NFD')
    .replace(/[^a-z]/g, '')}:example.test`;
const spaces = SPACE_NAMES.map((name) => summary(spaceId(name), name, { isSpace: true }));
const homeRooms = Array.from({ length: 60 }, (_, index) =>
  summary(`!room-${String(index).padStart(2, '0')}:example.test`, `Room ${index}`)
);
const dms = ['!dm-a:example.test', '!dm-b:example.test', '!dm-c:example.test'].map((id) =>
  summary(id, id.slice(1, 5), { isDirect: true })
);
// One room inside Design, so it is not a Home room.
const designRoom = summary('!design-room:example.test', 'Design chat');
let rooms: RoomSummary[] = [...homeRooms, ...dms, designRoom, ...spaces];

/** Core's space rollups: the fixture's buttons set them directly. */
let spaceRollups: Record<string, { total: number; highlight: number }> = {};

const params = new URLSearchParams(location.search);
let spacesContent: Record<string, unknown> | null = {
  sidebar: [
    spaceId('Design'),
    { id: 'folder-work', name: 'Work', content: [spaceId('Engineering'), spaceId('Infra')] },
    ...SPACE_NAMES.filter((name) => !['Design', 'Engineering', 'Infra'].includes(name)).map(
      spaceId
    ),
  ],
};
let failSaves = params.has('failSaves');
const saves: unknown[] = [];
const readCommands: string[] = [];

const update = (roomId: string, change: Partial<RoomSummary>) => {
  rooms = rooms.map((r) => (r.roomId === roomId ? { ...r, ...change } : r));
};

const presentation = () => {
  const base = presentationFor(rooms);
  const rollups = Object.entries(spaceRollups).map(([roomId, unread]) => ({ roomId, ...unread }));
  return { ...base, unread: [...base.unread, ...rollups] };
};

const session = {
  status: 'logged_in',
  sessionGeneration: 1,
  user_id: '@reader:example.test',
  device_id: 'TEST',
  homeserver_url: 'https://example.test',
};

const approval = {
  roomId: '!room-01:example.test',
  eventId: '$approval',
  sender: '@agent:example.test',
  body: '⚠️ Dangerous command requires approval\n```sh\nls\n```\nReply !approve to execute or !deny to cancel.',
  originServerTs: Date.now() - 1000,
  expiresAt: Date.now() + 300000,
  status: 'pending',
  canSendReaction: true,
  bodyTruncated: false,
};

const invoke = async (command: string, args?: Record<string, unknown>) => {
  if (command === 'matrix_session_snapshot') return session;
  if (command === 'matrix_room_list_snapshot' || command === 'matrix_room_list_snapshot_since') {
    const snapshot = {
      sessionGeneration: 1,
      orderedRoomIds: rooms.map((r) => r.roomId),
      rooms,
      presentation: presentation(),
    };
    return command === 'matrix_room_list_snapshot' ? snapshot : { revision: 0, snapshot };
  }
  if (command === 'matrix_mdirect_snapshot') {
    return { sessionGeneration: 1, roomIds: dms.map((d) => d.roomId), userIds: [] };
  }
  if (command === 'matrix_space_parents_snapshot') {
    return {
      sessionGeneration: 1,
      entries: [{ roomId: designRoom.roomId, parentIds: [spaceId('Design')] }],
    };
  }
  if (command === 'matrix_account_data_get') {
    return {
      eventType: args?.eventType,
      content: args?.eventType === 'in.synara.spaces' ? spacesContent : null,
    };
  }
  if (command === 'matrix_account_data_set') {
    if (failSaves) throw new Error('Fixture save failed');
    spacesContent = args?.content as Record<string, unknown>;
    saves.push(spacesContent);
    document.body.dataset.savedSpaces = JSON.stringify(
      (spacesContent as { sidebar?: unknown }).sidebar
    );
    return { eventType: args?.eventType, content: spacesContent };
  }
  if (command === 'matrix_room_set_read_state' && args?.action === 'mark_read') {
    const roomId = String(args.roomId);
    readCommands.push(roomId);
    document.body.dataset.readRooms = JSON.stringify(readCommands);
    update(roomId, { unreadCount: 0, highlightCount: 0, markedUnread: false });
    return { receiptSent: true, acknowledgedEventId: '$latest', unreadFlagCleared: false };
  }
  if (command === 'matrix_agent_approvals_list') {
    return {
      sessionGeneration: 1,
      coverageWindowMs: 300000,
      coverage: 'discovery',
      items: params.has('noApprovals') ? [] : [approval],
      loading: false,
      incomplete: false,
    };
  }
  throw new Error(`Unexpected fixture command ${command}`);
};
const fixtureSession = createNativeSession(async (command) => ({
  available: true,
  value: await invoke(command),
}));
// Every harness fixture module loads; only the one on the page may install
// its native bridge and session.
if (params.has('navPane')) {
  window.__SYNARA_DESKTOP__ = {
    platform: 'tauri',
    invoke: async <T,>(cmd: string, args?: Record<string, unknown>) =>
      invoke(cmd, args) as Promise<T>,
  };
  setNativeSessionForTests(fixtureSession);
  enableMapSet();
  document.body.dataset.failSaves = String(failSaves);
  // The app themes <body> (ThemeManager), which portalled menus inherit.
  document.body.classList.add(darkTheme, 'dark-theme');
}
const nav = makeNavToActivePathAtom('@reader:example.test');
const closedCategories = makeClosedNavCategoriesAtom('@reader:example.test');
const openedFolders = makeOpenedSidebarFolderAtom('@reader:example.test');
const notificationPreferences = {
  mute: new Set<string>(),
  specialMessages: new Set<string>(),
  allMessages: new Set<string>(),
};

/** The page beside the list: its header's "Show room list" and the fixture controls. */
function ContentPage() {
  const location = useLocation();
  const [, rerender] = useState(0);
  const control = (label: string, action: () => void) => (
    <button
      type="button"
      onClick={() => {
        action();
        rerender((value) => value + 1);
      }}
    >
      {label}
    </button>
  );
  return (
    <Page style={{ gap: 8 }}>
      <PageHeader>
        <ShowRoomListButton />
        <output data-testid="current-route">{location.pathname}</output>
      </PageHeader>
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8, padding: 16 }}>
        {control('Mention in Design', () => {
          spaceRollups = { ...spaceRollups, [spaceId('Design')]: { total: 2, highlight: 1 } };
        })}
        {control('Unread in Jazz', () => {
          spaceRollups = { ...spaceRollups, [spaceId('Jazz')]: { total: 3, highlight: 0 } };
        })}
        {control('Clear spaces', () => {
          spaceRollups = {};
        })}
        {control('Receive Home', () => update('!room-00:example.test', { unreadCount: 4 }))}
        {control('Mention Home', () =>
          update('!room-00:example.test', { unreadCount: 4, highlightCount: 2 })
        )}
        {control('Fail saves', () => {
          failSaves = true;
          document.body.dataset.failSaves = 'true';
        })}
      </div>
    </Page>
  );
}

/** A space route's pane: the production header (switcher) and bar around a stub list. */
function SpacePane() {
  const { spaceIdOrAlias } = useParams();
  const name = getNativeRoom(spaceIdOrAlias ?? '')?.name ?? spaceIdOrAlias ?? '';
  return (
    <PageNav label="Rooms" header={<NavPaneHeader title={name} />} footer={<NavBottomBar />}>
      <p style={{ padding: 16 }} data-testid="space-pane">
        {name}
      </p>
    </PageNav>
  );
}

function HomePane() {
  return (
    <ClosedNavCategoriesProvider value={closedCategories}>
      <RoomsNotificationPreferencesProvider value={notificationPreferences}>
        <Home />
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
  const homeLayout = (
    <PageRoot collapsibleNav nav={<HomePane />}>
      <Outlet />
    </PageRoot>
  );
  return (
    <main
      className={`${configClass} ${varsClass} ${darkTheme} dark-theme`}
      style={{ display: 'flex', height: '100vh' }}
    >
      <Routes>
        <Route element={homeLayout}>
          <Route path="/home/*" element={<ContentPage />} />
          <Route path="/direct/*" element={<ContentPage />} />
          <Route path="/approvals/*" element={<ContentPage />} />
          <Route path="/inbox/*" element={<ContentPage />} />
        </Route>
        <Route
          path="/:spaceIdOrAlias/*"
          element={
            <PageRoot collapsibleNav nav={<SpacePane />}>
              <ContentPage />
            </PageRoot>
          }
        />
      </Routes>
      <DesktopAppShortcuts />
    </main>
  );
}

export function NavPaneFixture() {
  return (
    <Provider>
      <MemoryRouter initialEntries={['/home/']}>
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
