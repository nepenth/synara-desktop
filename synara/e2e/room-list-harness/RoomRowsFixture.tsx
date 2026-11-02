// Production room rows and section headers with a 56-room list: unread,
// mentions, muted, private, favourites, DMs and long names. Only native IPC
// data is synthetic. Visual proof for NAV-2/3/4/7/8/9.
import React, { useCallback, useState } from 'react';
import { Provider } from 'jotai';
import { MemoryRouter } from 'react-router-dom';
import { configClass, varsClass } from 'folds';
import { darkTheme, synaraLightTheme as lightTheme } from '../../src/colors.css';
import { presentationFor } from './presentation';
import {
  createNativeSession,
  getNativeRoom,
  setNativeSessionForTests,
} from '../../src/app/native/nativeSession';
import {
  useBindAllRoomsAtom,
  useNativeRoomListSnapshot,
} from '../../src/app/state/room-list/roomList';
import { useBindRoomToUnreadAtom } from '../../src/app/state/room/roomToUnread';
import { RoomNavItem } from '../../src/app/features/room-nav';
import { NavCategory, NavSearchField, NavSectionHeader } from '../../src/app/components/nav';
import { RoomNotificationMode } from '../../src/app/hooks/useRoomsNotificationPreferences';
import type { RoomSummary } from '../../src/app/features/matrix-dto/room';

const room = (roomId: string, name: string, change: Partial<RoomSummary>): RoomSummary => ({
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

const LONG = 'A very long room name that keeps going past the width of the navigation pane';
const special: RoomSummary[] = [
  room('!mention:example.test', 'design-reviews', { unreadCount: 7, highlightCount: 2 }),
  room('!unread:example.test', 'release-train', { unreadCount: 5 }),
  room('!muted:example.test', 'random-noise', { unreadCount: 9, notificationMode: 'mute' }),
  room('!private:example.test', 'leadership', { joinRule: 'invite' }),
  room('!private-unread:example.test', 'incident-bridge', { joinRule: 'invite', unreadCount: 1 }),
  room('!fav:example.test', 'ops', { isFavorite: true }),
  room('!long:example.test', LONG, { unreadCount: 3, highlightCount: 1 }),
  room('!dm:example.test', 'Alice Liddell', {
    isDirect: true,
    directUserId: '@alice:example.test',
    unreadCount: 4,
  }),
  room('!dm-read:example.test', 'Bob', { isDirect: true, directUserId: '@bob:example.test' }),
  room('!many:example.test', 'firehose', { unreadCount: 250, highlightCount: 143 }),
];
const filler: RoomSummary[] = Array.from({ length: 46 }, (_, index) =>
  room(`!room-${index}:example.test`, `project-${String(index + 1).padStart(2, '0')}`, {
    lastActivityTs: 1_700_000_000_000 - index * 60_000,
  })
);
const rooms = [...special, ...filler];
const session = {
  status: 'logged_in',
  sessionGeneration: 1,
  user_id: '@reader:example.test',
  device_id: 'TEST',
  homeserver_url: 'https://example.test',
};
const invoke = async (command: string) => {
  if (command === 'matrix_session_snapshot') return session;
  if (command === 'matrix_room_list_snapshot') {
    return {
      sessionGeneration: 1,
      orderedRoomIds: rooms.map((r) => r.roomId),
      rooms,
      presentation: presentationFor(rooms),
    };
  }
  if (command === 'matrix_space_parents_snapshot') return { sessionGeneration: 1, entries: [] };
  throw new Error(`Unexpected fixture command ${command}`);
};
const installFixtureSession = () => {
  window.__SYNARA_DESKTOP__ = {
    platform: 'tauri',
    invoke: async <T,>(cmd: string) => invoke(cmd) as Promise<T>,
  };
  const fixtureSession = createNativeSession(async (command) => ({
    available: true,
    value: await invoke(command),
  }));
  setNativeSessionForTests(fixtureSession);
  return fixtureSession;
};

const modeFor = (summary: RoomSummary | undefined): RoomNotificationMode =>
  summary?.notificationMode === 'mute' ? RoomNotificationMode.Mute : RoomNotificationMode.Unset;

function BoundRows({
  fixtureSession,
  light,
}: {
  fixtureSession: ReturnType<typeof installFixtureSession>;
  light: boolean;
}) {
  useBindAllRoomsAtom(
    useCallback((snapshot) => fixtureSession.applyRoomListSnapshot(snapshot), [fixtureSession]),
    useCallback((snapshot) => fixtureSession.applySessionSnapshot(snapshot), [fixtureSession])
  );
  useBindRoomToUnreadAtom();
  const { orderedRoomIds, rooms: summaries } = useNativeRoomListSnapshot();
  const [closed, setClosed] = useState(false);
  const [selected, setSelected] = useState('!fav:example.test');
  const favorites = orderedRoomIds.filter(
    (roomId) => summaries.find((s) => s.roomId === roomId)?.isFavorite
  );
  const rest = orderedRoomIds.filter((roomId) => !favorites.includes(roomId));
  const row = (roomId: string, favoriteMarker?: boolean) => {
    const nativeRoom = getNativeRoom(roomId);
    if (!nativeRoom) return null;
    const summary = summaries.find((s) => s.roomId === roomId);
    return (
      <div
        key={roomId}
        data-testid="room-row"
        data-room-id={roomId}
        onClick={() => setSelected(roomId)}
      >
        <RoomNavItem
          room={nativeRoom}
          selected={selected === roomId}
          linkPath={`/home/${roomId}/`}
          notificationMode={modeFor(summary)}
          favoriteMarker={favoriteMarker}
        />
      </div>
    );
  };
  return (
    <main
      className={`${configClass} ${varsClass} ${light ? lightTheme : darkTheme}`}
      style={{ padding: 16 }}
    >
      <nav
        data-testid="room-rows"
        aria-label="Rooms"
        style={{
          width: 256,
          display: 'flex',
          flexDirection: 'column',
          background: 'var(--synara-room-list-background, #1e1f22)',
        }}
      >
        <NavSearchField onOpen={() => undefined} />
        <div style={{ padding: 8, display: 'flex', flexDirection: 'column', gap: 12 }}>
          <NavCategory>
            <NavSectionHeader label="Favorites" closed={false} onToggle={() => undefined} />
            {favorites.map((roomId) => row(roomId))}
          </NavCategory>
          <NavCategory>
            <NavSectionHeader
              label="Rooms"
              closed={closed}
              categoryId="rooms"
              onToggle={() => setClosed((value) => !value)}
              actions={
                <button type="button" aria-label="Sort Rooms" aria-expanded={false}>
                  sort
                </button>
              }
            />
            {(closed
              ? rest.filter((roomId) => {
                  const s = summaries.find((item) => item.roomId === roomId);
                  return (
                    roomId === selected ||
                    (s &&
                      s.notificationMode !== 'mute' &&
                      (s.unreadCount > 0 || s.highlightCount > 0))
                  );
                })
              : rest
            ).map((roomId) => row(roomId, true))}
          </NavCategory>
        </div>
      </nav>
    </main>
  );
}

export function RoomRowsFixture() {
  const [fixtureSession] = useState(installFixtureSession);
  const light = new URLSearchParams(window.location.search).has('light');
  return (
    <Provider>
      <MemoryRouter initialEntries={['/home/']}>
        <BoundRows fixtureSession={fixtureSession} light={light} />
      </MemoryRouter>
    </Provider>
  );
}
