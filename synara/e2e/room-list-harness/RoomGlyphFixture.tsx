// Production room rows; only native IPC data is synthetic. A DM shows its peer
// in the `#` slot, and every other room keeps `#`.
import React, { useCallback, useState } from 'react';
import { Provider } from 'jotai';
import { MemoryRouter } from 'react-router-dom';
import { configClass, varsClass } from 'folds';
import { darkTheme } from '../../src/colors.css';
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
import { RoomNavItem } from '../../src/app/features/room-nav';
import { RoomNotificationMode } from '../../src/app/hooks/useRoomsNotificationPreferences';
import type { RoomSummary } from '../../src/app/features/matrix-dto/room';

// Real summaries carry an mxc URI that loads through synara-media, which a
// plain browser cannot open. A data URI stands in for the resolved image.
const ALICE_AVATAR = `data:image/svg+xml,${encodeURIComponent(
  '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 36 36"><rect width="36" height="36" fill="#d9822b"/><circle cx="18" cy="14" r="7" fill="#fff4e6"/><rect x="7" y="24" width="22" height="14" rx="7" fill="#fff4e6"/></svg>'
)}`;

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
  ...change,
});
const rooms = [
  room('!alice:example.test', 'Alice Liddell', {
    isDirect: true,
    directUserId: '@alice:example.test',
    avatarUrl: ALICE_AVATAR,
  }),
  room('!bob:example.test', 'bob', { isDirect: true, directUserId: '@bob:example.test' }),
  room('!group:example.test', 'Carol and Dan', { isDirect: true }),
  room('!ops:example.test', 'Ops', {}),
];
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
  throw new Error(`Unexpected fixture command ${command}`);
};
// Installed when this fixture renders, not on import: main.tsx imports every
// fixture, and a module-level install would replace the others' sessions.
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

function BoundRooms({
  fixtureSession,
}: {
  fixtureSession: ReturnType<typeof installFixtureSession>;
}) {
  useBindAllRoomsAtom(
    useCallback((snapshot) => fixtureSession.applyRoomListSnapshot(snapshot), [fixtureSession]),
    useCallback((snapshot) => fixtureSession.applySessionSnapshot(snapshot), [fixtureSession])
  );
  const { orderedRoomIds } = useNativeRoomListSnapshot();
  return (
    <main className={`${configClass} ${varsClass} ${darkTheme}`} style={{ padding: 16 }}>
      <nav
        data-testid="room-glyph-list"
        style={{
          width: 280,
          padding: 8,
          display: 'flex',
          flexDirection: 'column',
          gap: 2,
          background: 'var(--synara-room-list-background, #1e1f22)',
        }}
      >
        {orderedRoomIds.map((roomId) => {
          const nativeRoom = getNativeRoom(roomId);
          if (!nativeRoom) return null;
          return (
            <div key={roomId} data-testid="room-glyph-row" data-room-id={roomId}>
              <RoomNavItem
                room={nativeRoom}
                selected={false}
                linkPath={`/home/${roomId}/`}
                notificationMode={RoomNotificationMode.Unset}
              />
            </div>
          );
        })}
      </nav>
    </main>
  );
}

export function RoomGlyphFixture() {
  const [fixtureSession] = useState(installFixtureSession);
  return (
    <Provider>
      <MemoryRouter initialEntries={['/home/']}>
        <BoundRooms fixtureSession={fixtureSession} />
      </MemoryRouter>
    </Provider>
  );
}
