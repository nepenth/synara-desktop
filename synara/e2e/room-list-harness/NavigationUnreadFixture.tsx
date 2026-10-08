// Production rail tabs and snapshot bindings; only native IPC data is synthetic.
import React, { useCallback } from 'react';
import { presentationFor } from './presentation';
import { Provider } from 'jotai';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { configClass, varsClass } from 'folds';
import { darkTheme } from '../../src/colors.css';
import { ScreenSize, ScreenSizeProvider } from '../../src/app/hooks/useScreenSize';
import { NavToActivePathProvider } from '../../src/app/state/hooks/navToActivePath';
import { makeNavToActivePathAtom } from '../../src/app/state/navToActivePath';
import { createNativeSession, setNativeSessionForTests } from '../../src/app/native/nativeSession';
import { useBindAllRoomsAtom } from '../../src/app/state/room-list/roomList';
import { useBindMDirectAtom } from '../../src/app/state/mDirectList';
import { useBindRoomToUnreadAtom } from '../../src/app/state/room/roomToUnread';
import { useBindRoomToParentsAtom } from '../../src/app/state/room/roomToParents';
import { HomeTab } from '../../src/app/pages/client/sidebar/HomeTab';
import { DirectTab } from '../../src/app/pages/client/sidebar/DirectTab';
import { useDirectRooms } from '../../src/app/pages/client/direct/useDirectRooms';
import { useHomeRooms } from '../../src/app/pages/client/home/useHomeRooms';
import { useRoomNavigate } from '../../src/app/hooks/useRoomNavigate';
import type { RoomSummary } from '../../src/app/features/matrix-dto/room';

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
let rooms = [room(dm, true), room(otherDM, true), room(home, false)];
const params = new URLSearchParams(location.search);
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
    return { status: 'updated' };
  }
  if (command === 'matrix_session_snapshot') return session;
  if (command === 'matrix_room_list_snapshot') {
    return {
      sessionGeneration: 1,
      orderedRoomIds: rooms.map((r) => r.roomId),
      rooms,
      presentation: presentationFor(rooms),
    };
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
const nav = makeNavToActivePathAtom('@reader:example.test');
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

function BoundRail() {
  useBindAllRoomsAtom(
    useCallback((snapshot) => fixtureSession.applyRoomListSnapshot(snapshot), []),
    useCallback((snapshot) => fixtureSession.applySessionSnapshot(snapshot), [])
  );
  useBindMDirectAtom();
  useBindRoomToParentsAtom();
  useBindRoomToUnreadAtom();
  return (
    <main className={`${configClass} ${varsClass} ${darkTheme}`} style={{ padding: 24 }}>
      <nav style={{ width: 66, display: 'flex', flexDirection: 'column', gap: 24 }}>
        <div data-testid="home-rail">
          <HomeTab />
        </div>
        <div data-testid="dm-rail">
          <DirectTab />
        </div>
      </nav>
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
