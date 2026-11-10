// The production Home rail tab and snapshot bindings; only native IPC data is
// synthetic. Since wave 3 the rail has no Direct messages tab: DMs are a
// section of the one Home list (NAV-12) and count in the Home badge. `homePane`
// adds the production Home pane, mounted across `/home/` and `/direct/` routes
// as in the app router. `collapsible` lays out the desktop client: the rail with
// the room list toggle, the collapsible Home pane in `PageRoot`, a room page with
// a composer, and the window shortcuts.
import React, { useCallback, useState } from 'react';
import { presentationFor } from './presentation';
import { Provider } from 'jotai';
import { enableMapSet } from 'immer';
import { MemoryRouter, Outlet, Route, Routes, useLocation } from 'react-router-dom';
import { configClass, varsClass } from 'folds';
import { darkTheme, synaraLightTheme as lightTheme } from '../../src/colors.css';
import { ScreenSize, ScreenSizeProvider } from '../../src/app/hooks/useScreenSize';
import { NavToActivePathProvider } from '../../src/app/state/hooks/navToActivePath';
import { makeNavToActivePathAtom } from '../../src/app/state/navToActivePath';
import { createNativeSession, setNativeSessionForTests } from '../../src/app/native/nativeSession';
import { useBindAllRoomsAtom } from '../../src/app/state/room-list/roomList';
import { useBindMDirectAtom } from '../../src/app/state/mDirectList';
import { useBindRoomToUnreadAtom } from '../../src/app/state/room/roomToUnread';
import { useBindRoomToParentsAtom } from '../../src/app/state/room/roomToParents';
import { HomeTab } from '../../src/app/pages/client/sidebar/HomeTab';
import { useDirectRooms } from '../../src/app/pages/client/direct/useDirectRooms';
import { useHomeRooms } from '../../src/app/pages/client/home/useHomeRooms';
import { useRoomNavigate } from '../../src/app/hooks/useRoomNavigate';
import type { RoomSummary } from '../../src/app/features/matrix-dto/room';
import { Home } from '../../src/app/pages/client/home/Home';
import { ClosedNavCategoriesProvider } from '../../src/app/state/hooks/closedNavCategories';
import { makeClosedNavCategoriesAtom } from '../../src/app/state/closedNavCategories';
import { RoomsNotificationPreferencesProvider } from '../../src/app/hooks/useRoomsNotificationPreferences';
import { Page, PageRoot } from '../../src/app/components/page';
import { Sidebar, SidebarStack } from '../../src/app/components/sidebar';
import { RoomListTab } from '../../src/app/pages/client/sidebar/RoomListTab';
import { DesktopAppShortcuts } from '../../src/app/features/desktop-keyboard/DesktopAppShortcuts';

const room = (roomId: string, isDirect: boolean): RoomSummary => ({
  roomId,
  name: roomId,
  membership: 'join',
  isDirect,
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
});
const dm = '!dm:example.test';
const otherDM = '!other-dm:example.test';
const home = '!home:example.test';
const params = new URLSearchParams(location.search);
const collapsible = params.has('collapsible');
const homePane = params.has('homePane') || collapsible;
// `manyRooms`: 400 Home rooms (named in list order), far more than the
// virtual list renders, with one unread room deep in the list.
const manyRooms = params.has('manyRooms');
const fillerName = (index: number) =>
  manyRooms
    ? `!filler-${String(index).padStart(3, '0')}:example.test`
    : `!filler-${index}:example.test`;
// Enough Home rooms that the Direct messages section starts below the fold.
const filler = homePane
  ? Array.from({ length: manyRooms ? 400 : 30 }, (_, index) => ({
      ...room(fillerName(index), false),
      unreadCount: manyRooms && index === 350 ? 3 : 0,
    }))
  : [];
let rooms = [room(dm, true), room(otherDM, true), room(home, false), ...filler];
const mdirectLagging = params.has('laggingMDirect');
const session = {
  status: 'logged_in',
  sessionGeneration: 1,
  user_id: '@reader:example.test',
  device_id: 'TEST',
  homeserver_url: 'https://example.test',
};
const readCommands: string[] = [];
const invoke = async (command: string, args?: Record<string, unknown>) => {
  if (command === 'matrix_room_set_read_state' && args?.action === 'mark_read') {
    const roomId = String(args.roomId);
    readCommands.push(roomId);
    document.body.dataset.readRooms = JSON.stringify(readCommands);
    update(roomId, { unreadCount: 0, highlightCount: 0, markedUnread: false });
    // The native readback shape: menus now surface a mark-read failure, so the
    // fixture must report the receipt Core wrote.
    return { receiptSent: true, acknowledgedEventId: '$latest', unreadFlagCleared: false };
  }
  if (command === 'matrix_session_snapshot') return session;
  if (command === 'matrix_room_list_snapshot' || command === 'matrix_room_list_snapshot_since') {
    const snapshot = {
      sessionGeneration: 1,
      orderedRoomIds: rooms.map((r) => r.roomId),
      rooms,
      presentation: presentationFor(rooms),
    };
    return command === 'matrix_room_list_snapshot' ? snapshot : { revision: 0, snapshot };
  }
  if (command === 'matrix_mdirect_snapshot') {
    return {
      sessionGeneration: 1,
      roomIds: mdirectLagging ? [] : [dm, otherDM],
      userIds: [],
    };
  }
  if (command === 'matrix_space_parents_snapshot') return { sessionGeneration: 1, entries: [] };
  throw new Error(`Unexpected fixture command ${command}`);
};
window.__SYNARA_DESKTOP__ = {
  platform: 'tauri',
  invoke: async <T,>(cmd: string, args?: Record<string, unknown>) =>
    invoke(cmd, args) as Promise<T>,
};
const fixtureSession = createNativeSession(async (command) => ({
  available: true,
  value: await invoke(command),
}));
setNativeSessionForTests(fixtureSession);
// The app enables this at startup; route-path and category stores use Map/Set.
enableMapSet();
const nav = makeNavToActivePathAtom('@reader:example.test');
const closedCategories = makeClosedNavCategoriesAtom('@reader:example.test');
const notificationPreferences = {
  mute: new Set<string>(),
  specialMessages: new Set<string>(),
  allMessages: new Set<string>(),
};

function HomePane() {
  return (
    <div
      data-testid="home-pane"
      // A row flex box stretches the pane to this height, as `PageRoot` does.
      style={{ width: 280, height: 420, display: 'flex', overflow: 'hidden' }}
    >
      <ClosedNavCategoriesProvider value={closedCategories}>
        <RoomsNotificationPreferencesProvider value={notificationPreferences}>
          <Home />
        </RoomsNotificationPreferencesProvider>
      </ClosedNavCategoriesProvider>
    </div>
  );
}

/** One pane element for both route trees, like the app's pathless layout route. */
function HomePaneRoutes() {
  const [pane] = useState(() => <HomePane />);
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
const update = (roomId: string, change: Partial<RoomSummary>) => {
  rooms = rooms.map((r) => (r.roomId === roomId ? { ...r, ...change } : r));
};

function DestinationMembership() {
  const directRooms = useDirectRooms();
  const homeRooms = useHomeRooms();
  const location = useLocation();
  const { navigateRoom, navigateThread } = useRoomNavigate();
  const visibleRooms = location.pathname.startsWith('/direct') ? directRooms : homeRooms;
  return (
    <>
      <output data-testid="current-route">{location.pathname}</output>
      <button type="button" onClick={() => navigateRoom(dm)}>
        Open DM via navigation
      </button>
      <button type="button" onClick={() => navigateThread(dm, '$thread')}>
        Open DM thread via navigation
      </button>
      <button type="button" onClick={() => navigateRoom(home)}>
        Open Home via navigation
      </button>
      <ul data-testid="destination-rooms">
        {visibleRooms.map((id) => (
          <li key={id}>{id}</li>
        ))}
      </ul>
    </>
  );
}

/** The room page beside the pane: a composer and the fixture's unread controls. */
function RoomPage() {
  return (
    <Page style={{ padding: 16, gap: 8 }} data-file-drop-region="room">
      {/* The app's timeline scroller and Slate composer, for F6. */}
      <div id="native-timeline-history" role="region" aria-label="Messages" style={{ height: 80 }}>
        Messages
      </div>
      <textarea aria-label="Composer" />
      <div
        contentEditable
        suppressContentEditableWarning
        aria-label="Rich composer"
        role="textbox"
        data-slate-editor="true"
      >
        Draft
      </div>
      <button type="button" onClick={() => update(home, { unreadCount: 4 })}>
        Receive Home
      </button>
      <button type="button" onClick={() => update(dm, { unreadCount: 2 })}>
        Receive DM
      </button>
      <button
        type="button"
        onClick={() => {
          update(home, { unreadCount: 0, highlightCount: 0, markedUnread: false });
          update(dm, { unreadCount: 0, highlightCount: 0, markedUnread: false });
        }}
      >
        Mark all read
      </button>
    </Page>
  );
}

function CollapsibleClient() {
  const light = params.get('theme') === 'light';
  const pane = (
    <ClosedNavCategoriesProvider value={closedCategories}>
      <RoomsNotificationPreferencesProvider value={notificationPreferences}>
        <Home />
      </RoomsNotificationPreferencesProvider>
    </ClosedNavCategoriesProvider>
  );
  return (
    <main
      // index.css scopes the reading-role variables under `.dark-theme`, as the app's body has.
      className={`${configClass} ${varsClass} ${light ? lightTheme : `${darkTheme} dark-theme`}`}
      style={{ display: 'flex', height: '100vh' }}
    >
      <Sidebar as="nav" aria-label="Spaces">
        <SidebarStack>
          <RoomListTab />
        </SidebarStack>
        <SidebarStack>
          <div data-testid="home-rail">
            <HomeTab />
          </div>
        </SidebarStack>
      </Sidebar>
      <Routes>
        <Route
          element={
            <PageRoot collapsibleNav nav={pane}>
              <Outlet />
            </PageRoot>
          }
        >
          <Route path="/home/" element={<RoomPage />} />
          <Route path="/home/:roomIdOrAlias/*" element={<RoomPage />} />
          <Route path="/direct/:roomIdOrAlias/*" element={<RoomPage />} />
        </Route>
      </Routes>
      <DesktopAppShortcuts />
    </main>
  );
}

function BoundRail() {
  useBindAllRoomsAtom(
    useCallback((snapshot) => fixtureSession.applyRoomListSnapshot(snapshot), []),
    useCallback((snapshot) => fixtureSession.applySessionSnapshot(snapshot), [])
  );
  useBindMDirectAtom();
  useBindRoomToParentsAtom();
  useBindRoomToUnreadAtom();
  if (collapsible) return <CollapsibleClient />;
  return (
    <main className={`${configClass} ${varsClass} ${darkTheme}`} style={{ padding: 24 }}>
      <nav style={{ width: 66, display: 'flex', flexDirection: 'column', gap: 24 }}>
        <div data-testid="home-rail">
          <HomeTab />
        </div>
      </nav>
      {homePane && <HomePaneRoutes />}
      <DestinationMembership />
      <button type="button" onClick={() => update(dm, { unreadCount: 2 })}>
        Receive DM
      </button>
      <button type="button" onClick={() => update(otherDM, { unreadCount: 3 })}>
        Receive second DM
      </button>
      <button type="button" onClick={() => update(home, { unreadCount: 4 })}>
        Receive Home
      </button>
      <button type="button" onClick={() => update(home, { unreadCount: 4, highlightCount: 1 })}>
        Mention Home
      </button>
      <button type="button" onClick={() => update(dm, { unreadCount: 0, highlightCount: 1 })}>
        Mention only DM
      </button>
      <button type="button" onClick={() => update(dm, { notificationMode: 'mute' })}>
        Mute DM
      </button>
      <button
        type="button"
        onClick={() => update(dm, { unreadCount: 0, highlightCount: 0, markedUnread: false })}
      >
        Mark DM read
      </button>
      <button
        type="button"
        onClick={() => update(otherDM, { unreadCount: 0, highlightCount: 0, markedUnread: false })}
      >
        Mark second DM read
      </button>
      <button
        type="button"
        onClick={() => update(home, { unreadCount: 0, highlightCount: 0, markedUnread: false })}
      >
        Mark Home read
      </button>
      <button type="button" onClick={() => update(dm, { markedUnread: true })}>
        Mark DM unread
      </button>
    </main>
  );
}

export function NavigationUnreadFixture() {
  return (
    <Provider>
      <MemoryRouter initialEntries={[params.has('directSelected') ? '/direct/' : '/home/']}>
        <ScreenSizeProvider value={ScreenSize.Desktop}>
          <NavToActivePathProvider value={nav}>
            <BoundRail />
          </NavToActivePathProvider>
        </ScreenSizeProvider>
      </MemoryRouter>
    </Provider>
  );
}
