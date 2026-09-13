// Production Approvals, navigation hooks, back handler, and route patterns.
// Only Matrix/inbox data and the destination room's content are fixtures.
import React from 'react';
import { createRoot } from 'react-dom/client';
import { createStore, Provider } from 'jotai';
import { MemoryRouter, Routes, Route, useLocation, useParams } from 'react-router-dom';
import { configClass, varsClass } from 'folds';
import 'folds/dist/style.css';
import '@fontsource/inter/variable.css';
import '../../src/index.css';
import { darkTheme } from '../../src/colors.css';
import { Approvals } from '../../src/app/features/approvals/Approvals';
import {
  ApprovalInboxContext,
  type ApprovalInboxContextValue,
} from '../../src/app/features/approvals/ApprovalInboxProvider';
import { MatrixClientProvider } from '../../src/app/hooks/useMatrixClient';
import { ScreenSize, ScreenSizeProvider } from '../../src/app/hooks/useScreenSize';
import { useNavigateToApprovals } from '../../src/app/hooks/useNavigateToApprovals';
import { BackRouteHandler } from '../../src/app/components/BackRouteHandler';
import { roomToParentsAtom } from '../../src/app/state/room/roomToParents';
import { mDirectAtom } from '../../src/app/state/mDirectList';
import type { MatrixClient } from '../../src/client/initMatrix';
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
const mx = {
  getRoom: (id: string) => rooms.find((item) => item.roomId === id) ?? null,
  getRooms: () => rooms,
  getSafeUserId: () => '@reader:example.test',
  getUserId: () => '@reader:example.test',
} as unknown as MatrixClient;
const store = createStore();
store.set(mDirectAtom, { type: 'INITIALIZE', rooms: new Set(kind === 'direct' ? [roomId] : []) });
store.set(roomToParentsAtom, {
  type: 'INITIALIZE',
  roomToParents: new Map(kind === 'space' ? [[roomId, new Set([firstParent, originParent])]] : []),
});
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
    <MatrixClientProvider value={mx}>
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
    </MatrixClientProvider>
  </Provider>
);
