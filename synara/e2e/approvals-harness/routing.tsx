// Production Approvals, navigation hooks, back handler, and route patterns.
// Only Matrix/inbox data and the destination room's content are fixtures.
import React, { ReactNode } from 'react';
import { createRoot } from 'react-dom/client';
import { createStore, Provider } from 'jotai';
import { MemoryRouter, Routes, Route, useLocation, useParams } from 'react-router-dom';
import { configClass, varsClass } from 'folds';
import 'folds/dist/style.css';
import '@fontsource-variable/inter';
import '../../src/index.css';
import { darkTheme } from '../../src/colors.css';
import { Approvals } from '../../src/app/features/approvals/Approvals';
import {
  ApprovalInboxContext,
  type ApprovalInboxContextValue,
} from '../../src/app/features/approvals/ApprovalInboxProvider';
import { setNativeSessionForTests, type NativeSession } from '../../src/app/native/nativeSession';
import { setNativeIdentity } from '../../src/app/state/nativeIdentity';
import { ScreenSize, ScreenSizeProvider } from '../../src/app/hooks/useScreenSize';
import { useNavigateToApprovals } from '../../src/app/hooks/useNavigateToApprovals';
import { BackRouteHandler } from '../../src/app/components/BackRouteHandler';
import { roomToParentsAtom } from '../../src/app/state/room/roomToParents';
import {
  useBindAllRoomsAtom,
  useNativeRoomListSnapshot,
} from '../../src/app/state/room-list/roomList';
import type { RoomSummary } from '../../src/app/features/matrix-dto/room';
import {
  APPROVALS_PATH,
  HOME_PATH,
  DIRECT_PATH,
  INBOX_PATH,
  SPACE_PATH,
  HOME_ROOM_PATH,
  DIRECT_ROOM_PATH,
  SPACE_ROOM_PATH,
} from '../../src/app/pages/paths';
import { presentationFor } from '../room-list-harness/presentation';

const query = new URLSearchParams(location.search);
const kind = query.get('kind') ?? 'home';
const roomId = '!agent-room:example.test';
const roomAlias = query.has('rawId') ? undefined : '#agent-room:example.test';
const eventId = '$approval/+?%25:example.test';
const firstParent = '!first-parent:example.test';
const originParent = '!origin-parent:example.test';
const originAlias = '#origin-parent:example.test';
const room = (id: string, alias?: string) => ({
  roomId: id,
  name: id,
  getCanonicalAlias: () => alias,
  currentState: { getStateEvents: () => undefined },
  getMember: () => ({ name: 'Hermes' }),
});
const rooms = [room(roomId, roomAlias), room(firstParent), room(originParent, originAlias)];
setNativeSessionForTests({
  getRoom: (id: string) => rooms.find((item) => item.roomId === id) ?? null,
  getRooms: () => rooms,
} as unknown as NativeSession);
setNativeIdentity({ userId: '@reader:example.test' });
const store = createStore();
store.set(roomToParentsAtom, {
  type: 'INITIALIZE',
  roomToParents: new Map(kind === 'space' ? [[roomId, new Set([firstParent, originParent])]] : []),
});
// Classify the route from the same authoritative native projection as shipped
// navigation. The separate m.direct renderer projection is deliberately absent.
const nativeRooms: RoomSummary[] = rooms.map((item) => ({
  roomId: item.roomId,
  name: item.name,
  membership: 'join',
  isDirect: item.roomId === roomId && kind === 'direct',
  isSpace: item.roomId !== roomId,
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
}));
window.__SYNARA_DESKTOP__ = {
  platform: 'tauri',
  invoke: async <T,>(command: string) => {
    if (command === 'matrix_session_snapshot')
      return {
        status: 'logged_in',
        sessionGeneration: 1,
        user_id: '@reader:example.test',
        device_id: 'TEST',
        homeserver_url: 'https://example.test',
      } as T;
    if (command === 'matrix_room_list_snapshot')
      return {
        sessionGeneration: 1,
        orderedRoomIds: nativeRooms.map((item) => item.roomId),
        rooms: nativeRooms,
        presentation: presentationFor(nativeRooms),
      } as T;
    throw new Error(`Unexpected routing fixture command ${command}`);
  },
};
function NativeRoutingOwner({ children }: { children: ReactNode }) {
  useBindAllRoomsAtom();
  const snapshot = useNativeRoomListSnapshot();
  return snapshot.sessionGeneration === 1 ? children : null;
}
const now = Date.now();
const inbox: ApprovalInboxContextValue = {
  sessionGeneration: 1,
  coverage: 'discovery',
  pendingCount: 1,
  loading: false,
  incomplete: false,
  now,
  refresh: () => undefined,
  decided: () => false,
  items: [
    {
      roomId,
      eventId,
      sender: '@hermes:example.test',
      body: 'Approval required\n```sh\npwd\n```',
      originServerTs: now,
      expiresAt: now + 300000,
      status: 'pending',
      canSendReaction: true,
      bodyTruncated: false,
    },
  ],
  recentItems: [],
};

function Entry() {
  const navigateToApprovals = useNavigateToApprovals();
  return (
    <>
      <h1>Origin section</h1>
      <button type="button" onClick={navigateToApprovals}>
        Open approvals
      </button>
      <BackRouteHandler>
        {(onBack) => (
          <button type="button" onClick={onBack}>
            Section Back
          </button>
        )}
      </BackRouteHandler>
    </>
  );
}
function LocationOutput() {
  const current = useLocation();
  return (
    <output data-testid="location">
      {current.pathname}
      {current.search}
      {current.hash}
    </output>
  );
}
function Destination() {
  const params = useParams();
  return (
    <>
      <h1>Room destination</h1>
      <output data-testid="room-param">{params.roomIdOrAlias}</output>
      <output data-testid="event-param">{params.eventId}</output>
      <output data-testid="space-param">{params.spaceIdOrAlias ?? ''}</output>
    </>
  );
}
const origin =
  query.get('origin') ?? (kind === 'space' ? '/%23origin-parent%3Aexample.test/' : `/${kind}/`);
document.body.classList.add(configClass, varsClass, darkTheme, 'dark-theme');
createRoot(document.getElementById('root')!).render(
  <Provider store={store}>
    <NativeRoutingOwner>
      <ScreenSizeProvider value={ScreenSize.Mobile}>
        <ApprovalInboxContext.Provider value={inbox}>
          <MemoryRouter initialEntries={[origin]}>
            <LocationOutput />
            <Routes>
              <Route path={APPROVALS_PATH} element={<Approvals />} />
              <Route path={HOME_ROOM_PATH} element={<Destination />} />
              <Route path={DIRECT_ROOM_PATH} element={<Destination />} />
              <Route path={SPACE_ROOM_PATH} element={<Destination />} />
              <Route path={HOME_PATH} element={<Entry />} />
              <Route path={DIRECT_PATH} element={<Entry />} />
              <Route path={`${INBOX_PATH}*`} element={<Entry />} />
              <Route path={SPACE_PATH} element={<Entry />} />
            </Routes>
          </MemoryRouter>
        </ApprovalInboxContext.Provider>
      </ScreenSizeProvider>
    </NativeRoutingOwner>
  </Provider>
);
