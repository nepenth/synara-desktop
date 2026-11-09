// Production room rows and section headers with a 56-room list: unread,
// mentions, muted, private, favourites, DMs and long names. Only native IPC
// data is synthetic. Visual proof for NAV-2/3/4/7/8/9, for NAV-12 (the one
// list's sections in order Favorites, Rooms, Direct messages) and for wave 3:
// the reading hierarchy, the selected fill, the unread gutter marker, middle
// truncation of names and the pane title, and DM presence dots fed through
// the real presence subscription (Alice online, Bob and Dave offline, Carol
// unknown).
import React, { useCallback, useState } from 'react';
import { Provider } from 'jotai';
import { MemoryRouter } from 'react-router-dom';
import { color, configClass, varsClass } from 'folds';
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
import {
  NavCategory,
  NavPaneTitle,
  NavSearchField,
  NavSectionHeader,
} from '../../src/app/components/nav';
import { setSessionBootstrapResult } from '../../src/app/state/sessionBootstrap';
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
  room('!home-mgmt:example.test', 'Project - Home Management', {}),
  room('!hosting:example.test', 'Project - Hosting Migration', { unreadCount: 2 }),
  room('!dm:example.test', 'Alice Liddell', {
    isDirect: true,
    directUserId: '@alice:example.test',
    unreadCount: 4,
  }),
  room('!dm-read:example.test', 'Bob', { isDirect: true, directUserId: '@bob:example.test' }),
  room('!many:example.test', 'firehose', { unreadCount: 250, highlightCount: 143 }),
  room('!dm-mention:example.test', 'Carol', {
    isDirect: true,
    directUserId: '@carol:example.test',
    unreadCount: 1,
    highlightCount: 1,
  }),
  room('!dm-muted:example.test', 'Dave', {
    isDirect: true,
    directUserId: '@dave:example.test',
    unreadCount: 6,
    notificationMode: 'mute',
  }),
];
const filler: RoomSummary[] = Array.from({ length: 44 }, (_, index) =>
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
// Core presence snapshots for the DM peers; anyone else is unknown.
const presence: Record<string, 'online' | 'offline'> = {
  '@alice:example.test': 'online',
  '@bob:example.test': 'offline',
  '@dave:example.test': 'offline',
};
const invoke = async (command: string, args?: Record<string, unknown>) => {
  if (command === 'matrix_session_snapshot') return session;
  const userId = typeof args?.userId === 'string' ? args.userId : '';
  if (command === 'matrix_presence_snapshot') {
    const state = presence[userId];
    return state
      ? {
          status: 'ready',
          sessionGeneration: 1,
          userId,
          snapshot: { userId, state, currentlyActive: state === 'online' },
        }
      : { status: 'unknown', sessionGeneration: 1, userId };
  }
  if (command === 'matrix_presence_subscribe') {
    return { subscriptionId: `presence-${userId}`, userId, sessionGeneration: 1 };
  }
  if (command === 'matrix_presence_unsubscribe') return { status: 'ok' };
  if (command === 'matrix_user_status_snapshot') return { sessionGeneration: 1, userId };
  if (command === 'matrix_room_list_snapshot' || command === 'matrix_room_list_snapshot_since') {
    const snapshot = {
      sessionGeneration: 1,
      orderedRoomIds: rooms.map((r) => r.roomId),
      rooms,
      presentation: presentationFor(rooms),
    };
    return command === 'matrix_room_list_snapshot' ? snapshot : { revision: 0, snapshot };
  }
  if (command === 'matrix_space_parents_snapshot') return { sessionGeneration: 1, entries: [] };
  throw new Error(`Unexpected fixture command ${command}`);
};
const installFixtureSession = () => {
  window.__SYNARA_DESKTOP__ = {
    platform: 'tauri',
    invoke: async <T,>(cmd: string, args?: Record<string, unknown>) =>
      invoke(cmd, args) as Promise<T>,
  };
  // `@tauri-apps/api/event` listens through the internals; presence updates
  // never arrive here, the snapshot is the whole story.
  let callbacks = 0;
  Object.assign(window, {
    __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => undefined },
    __TAURI_INTERNALS__: {
      transformCallback: () => {
        callbacks += 1;
        return callbacks;
      },
      invoke: async (cmd: string, args?: Record<string, unknown>) => {
        if (cmd === 'plugin:event|listen') return callbacks;
        if (cmd === 'plugin:event|unlisten') return null;
        return invoke(cmd, args);
      },
    },
  });
  setSessionBootstrapResult({
    source: 'native',
    session: {
      userId: session.user_id,
      deviceId: session.device_id,
      baseUrl: session.homeserver_url,
      sessionGeneration: '1',
    },
  });
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
  const [directsClosed, setDirectsClosed] = useState(false);
  const [selected, setSelected] = useState('!fav:example.test');
  const favorites = orderedRoomIds.filter(
    (roomId) => summaries.find((s) => s.roomId === roomId)?.isFavorite
  );
  const isDirect = (roomId: string) => !!summaries.find((s) => s.roomId === roomId)?.isDirect;
  const rest = orderedRoomIds.filter((roomId) => !favorites.includes(roomId) && !isDirect(roomId));
  const directs = orderedRoomIds.filter(
    (roomId) => !favorites.includes(roomId) && isDirect(roomId)
  );
  // A collapsed section keeps its unread and selected rows (Slack).
  const keptWhenClosed = (roomId: string) => {
    const s = summaries.find((item) => item.roomId === roomId);
    return (
      roomId === selected ||
      (!!s && s.notificationMode !== 'mute' && (s.unreadCount > 0 || s.highlightCount > 0))
    );
  };
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
          linkPath={isDirect(roomId) ? `/direct/${roomId}/` : `/home/${roomId}/`}
          notificationMode={modeFor(summary)}
          favoriteMarker={favoriteMarker}
        />
      </div>
    );
  };
  return (
    <main
      // index.css scopes the reading-role variables under `.dark-theme`, as the app's body has.
      className={`${configClass} ${varsClass} ${light ? lightTheme : `${darkTheme} dark-theme`}`}
      style={{ padding: 16 }}
    >
      <nav
        data-testid="room-rows"
        aria-label="Rooms"
        style={{
          width: 256,
          display: 'flex',
          flexDirection: 'column',
          // The room list's own chrome role (`THEME_CHROME_ROLES.roomList`).
          background: color.Surface.Container,
        }}
      >
        {/* The Home pane header's geometry: logo, title, "+" and options. */}
        <header
          style={{ display: 'flex', alignItems: 'center', gap: 12, height: 48, padding: '0 12px' }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexGrow: 1, minWidth: 0 }}>
            <span
              style={{
                width: 22,
                height: 22,
                borderRadius: 6,
                flex: 'none',
                background: color.Primary.Main,
              }}
            />
            <NavPaneTitle text="matrix.whyland.net" />
          </div>
          <span style={{ width: 68, height: 32, flex: 'none' }} />
        </header>
        <NavSearchField />
        <div style={{ padding: 8, display: 'flex', flexDirection: 'column' }}>
          <NavCategory>
            <NavSectionHeader label="Favorites" closed={false} onToggle={() => undefined} />
            {favorites.map((roomId) => row(roomId))}
          </NavCategory>
          <NavCategory>
            <NavSectionHeader
              label="Rooms"
              spaced
              closed={closed}
              categoryId="rooms"
              onToggle={() => setClosed((value) => !value)}
              actions={
                <button type="button" aria-label="Sort Rooms" aria-expanded={false}>
                  sort
                </button>
              }
            />
            {(closed ? rest.filter(keptWhenClosed) : rest).map((roomId) => row(roomId, true))}
          </NavCategory>
          <NavCategory>
            <NavSectionHeader
              label="Direct messages"
              spaced
              closed={directsClosed}
              categoryId="direct|direct"
              onToggle={() => setDirectsClosed((value) => !value)}
              actions={
                <button type="button" aria-label="Sort Direct messages" aria-expanded={false}>
                  sort
                </button>
              }
            />
            {(directsClosed ? directs.filter(keptWhenClosed) : directs).map((roomId) =>
              row(roomId)
            )}
          </NavCategory>
        </div>
      </nav>
    </main>
  );
}

export function RoomRowsFixture() {
  const [fixtureSession] = useState(installFixtureSession);
  const search = new URLSearchParams(window.location.search);
  const light = search.has('light') || search.get('theme') === 'light';
  return (
    <Provider>
      <MemoryRouter initialEntries={['/home/']}>
        <BoundRows fixtureSession={fixtureSession} light={light} />
      </MemoryRouter>
    </Provider>
  );
}
