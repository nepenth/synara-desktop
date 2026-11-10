import React from 'react';
import {
  Outlet,
  Route,
  createBrowserRouter,
  createHashRouter,
  createRoutesFromElements,
  redirect,
  useNavigate,
} from 'react-router-dom';

import { ClientConfig } from '../hooks/useClientConfig';
import { AuthLayout, Login, Register, ResetPassword } from './auth';
import {
  DIRECT_PATH,
  EXPLORE_PATH,
  HOME_PATH,
  LOGIN_PATH,
  INBOX_PATH,
  REGISTER_PATH,
  RESET_PASSWORD_PATH,
  SETTINGS_PATH,
  SPACE_PATH,
  _CREATE_PATH,
  _FEATURED_PATH,
  _INVITES_PATH,
  _JOIN_PATH,
  _LATER_PATH,
  _LOBBY_PATH,
  _NOTIFICATIONS_PATH,
  _ROOM_PATH,
  _ROOM_THREAD_PATH,
  _SEARCH_PATH,
  _SERVER_PATH,
  CREATE_PATH,
  APPROVALS_PATH,
} from './paths';
import {
  getAppPathFromHref,
  getExploreFeaturedPath,
  getHomePath,
  getInboxNotificationsPath,
  getLoginPath,
  getOriginBaseUrl,
  getSpaceLobbyPath,
} from './pathUtils';
import { ClientBindAtoms, ClientLayout, ClientRoot } from './client';
import { Home, HomeJoin, HomeRouteRoomProvider, HomeSearch } from './client/home';
import { DirectCreate, DirectRouteRoomProvider } from './client/direct';
import { RouteSpaceProvider, Space, SpaceRouteRoomProvider, SpaceSearch } from './client/space';
import { Explore, FeaturedRooms, PublicRooms } from './client/explore';
import { Notifications, Inbox, Invites, Later } from './client/inbox';
import { setAfterLoginRedirectPath } from './afterLoginRedirectPath';
import { Room } from '../features/room';
import { Lobby } from '../features/lobby';
import { WelcomePage } from './client/WelcomePage';
import { PageRoot } from '../components/page';
import { ScreenSize } from '../hooks/useScreenSize';
import { MobileFriendlyPageNav } from './MobileFriendly';
import { ClientInitStorageAtom } from './client/ClientInitStorageAtom';
import { ClientNonUIFeatures } from './client/ClientNonUIFeatures';
import { AuthRouteThemeManager, UnAuthRouteThemeManager } from './ThemeManager';
import { NativeVerificationInboxRenderer } from '../features/verification/NativeDeviceVerification';
import { DeviceVerificationPromptRenderer } from '../features/cross-signing/DeviceVerificationPrompt';
import { RoomSettingsRenderer } from '../features/room-settings';
import { ClientRoomsNotificationPreferences } from './client/ClientRoomsNotificationPreferences';
import { SpaceSettingsRenderer } from '../features/space-settings';
import { UserRoomProfileRenderer } from '../components/UserRoomProfileRenderer';
import { CreateRoomModalRenderer } from '../features/create-room';
import { HomeCreateRoom } from './client/home/CreateRoom';
import { Create } from './client/create';
import { CreateSpaceModalRenderer } from '../features/create-space';
import { SearchModalRenderer } from '../features/search';
import { getActiveSession } from '../state/sessionBootstrap';
import { Settings } from '../features/settings';
import { SettingsModal } from '../components/settings-layout';
import { RouteError } from './RouteError';
import { ApprovalInboxProvider } from '../features/approvals/ApprovalInboxProvider';
import { Approvals } from '../features/approvals/Approvals';
import { lastActiveRoomPath, readStoredActivePaths } from './client/lastActiveRoomPath';

/**
 * Home and Direct index routes reopen the scope's last room (NAV-1). Only a
 * brand-new account, or a scope without a remembered room, sees Welcome.
 * Home is one list of rooms and DMs (NAV-12), so its last place may be a DM.
 */
const lastRoomLoader =
  (navId: 'home' | 'direct', basePath: string | string[], utility: string[]) => () => {
    const session = getActiveSession();
    if (!session) return null;
    const target = lastActiveRoomPath(
      readStoredActivePaths(session.userId),
      navId,
      basePath,
      utility
    );
    return target ? redirect(target) : null;
  };

function SettingsRoute() {
  const navigate = useNavigate();
  const closeSettings = () => navigate(getHomePath());

  return (
    <SettingsModal requestClose={closeSettings}>
      <Settings requestClose={closeSettings} />
    </SettingsModal>
  );
}

export const createRouter = (clientConfig: ClientConfig, screenSize: ScreenSize) => {
  const { hashRouter } = clientConfig;
  const mobile = screenSize === ScreenSize.Mobile;

  const routes = createRoutesFromElements(
    <Route errorElement={<RouteError />}>
      <Route
        index
        loader={() => {
          if (getActiveSession()) return redirect(getHomePath());
          const afterLoginPath = getAppPathFromHref(getOriginBaseUrl(), window.location.href);
          if (afterLoginPath) setAfterLoginRedirectPath(afterLoginPath);
          return redirect(getLoginPath());
        }}
      />
      <Route
        loader={() => {
          if (getActiveSession()) {
            return redirect(getHomePath());
          }

          return null;
        }}
        element={
          <>
            <AuthLayout />
            <UnAuthRouteThemeManager />
          </>
        }
      >
        <Route path={LOGIN_PATH} element={<Login />} />
        <Route path={REGISTER_PATH} element={<Register />} />
        <Route path={RESET_PASSWORD_PATH} element={<ResetPassword />} />
      </Route>

      <Route
        loader={() => {
          const session = getActiveSession();
          if (!session) {
            const afterLoginPath = getAppPathFromHref(
              getOriginBaseUrl(hashRouter),
              window.location.href
            );
            if (afterLoginPath) setAfterLoginRedirectPath(afterLoginPath);
            return redirect(getLoginPath());
          }
          return null;
        }}
        element={
          <AuthRouteThemeManager>
            <ClientRoot>
              <ClientInitStorageAtom>
                <ClientRoomsNotificationPreferences>
                  <ClientBindAtoms>
                    <ApprovalInboxProvider>
                      <ClientNonUIFeatures>
                        <ClientLayout>
                          <Outlet />
                        </ClientLayout>
                        <SearchModalRenderer />
                        <UserRoomProfileRenderer />
                        <CreateRoomModalRenderer />
                        <CreateSpaceModalRenderer />
                        <RoomSettingsRenderer />
                        <SpaceSettingsRenderer />
                        <NativeVerificationInboxRenderer />
                        <DeviceVerificationPromptRenderer />
                      </ClientNonUIFeatures>
                    </ApprovalInboxProvider>
                  </ClientBindAtoms>
                </ClientRoomsNotificationPreferences>
              </ClientInitStorageAtom>
            </ClientRoot>
          </AuthRouteThemeManager>
        }
      >
        <Route path={SETTINGS_PATH} element={<SettingsRoute />} />
        {/*
          One Home list for rooms and DMs (NAV-12): the pane stays mounted
          across `/home/` and `/direct/` routes; DMs keep their own route.
        */}
        <Route
          element={
            <PageRoot
              collapsibleNav
              nav={
                <MobileFriendlyPageNav path={[HOME_PATH, DIRECT_PATH]}>
                  <Home />
                </MobileFriendlyPageNav>
              }
            >
              <Outlet />
            </PageRoot>
          }
        >
          <Route path={HOME_PATH}>
            {mobile ? null : (
              <Route
                index
                loader={lastRoomLoader(
                  'home',
                  [HOME_PATH, DIRECT_PATH],
                  [_CREATE_PATH, _JOIN_PATH, _SEARCH_PATH]
                )}
                element={<WelcomePage />}
              />
            )}
            <Route path={_CREATE_PATH} element={<HomeCreateRoom />} />
            <Route path={_JOIN_PATH} element={<HomeJoin />} />
            <Route path={_SEARCH_PATH} element={<HomeSearch />} />
            <Route
              path={_ROOM_THREAD_PATH}
              element={
                <HomeRouteRoomProvider>
                  <Room />
                </HomeRouteRoomProvider>
              }
            />
            <Route
              path={_ROOM_PATH}
              element={
                <HomeRouteRoomProvider>
                  <Room />
                </HomeRouteRoomProvider>
              }
            />
          </Route>
          <Route path={DIRECT_PATH}>
            {mobile ? null : (
              <Route
                index
                loader={lastRoomLoader('direct', DIRECT_PATH, [_CREATE_PATH])}
                element={<WelcomePage />}
              />
            )}
            <Route path={_CREATE_PATH} element={<DirectCreate />} />
            <Route
              path={_ROOM_THREAD_PATH}
              element={
                <DirectRouteRoomProvider>
                  <Room />
                </DirectRouteRoomProvider>
              }
            />
            <Route
              path={_ROOM_PATH}
              element={
                <DirectRouteRoomProvider>
                  <Room />
                </DirectRouteRoomProvider>
              }
            />
          </Route>
        </Route>
        <Route
          path={SPACE_PATH}
          element={
            <RouteSpaceProvider>
              <PageRoot
                collapsibleNav
                nav={
                  <MobileFriendlyPageNav path={SPACE_PATH}>
                    <Space />
                  </MobileFriendlyPageNav>
                }
              >
                <Outlet />
              </PageRoot>
            </RouteSpaceProvider>
          }
        >
          {mobile ? null : (
            <Route
              index
              loader={({ params }) => {
                const { spaceIdOrAlias } = params;
                if (spaceIdOrAlias) {
                  return redirect(getSpaceLobbyPath(spaceIdOrAlias));
                }
                return null;
              }}
              element={<WelcomePage />}
            />
          )}
          <Route path={_LOBBY_PATH} element={<Lobby />} />
          <Route path={_SEARCH_PATH} element={<SpaceSearch />} />
          <Route
            path={_ROOM_THREAD_PATH}
            element={
              <SpaceRouteRoomProvider>
                <Room />
              </SpaceRouteRoomProvider>
            }
          />
          <Route
            path={_ROOM_PATH}
            element={
              <SpaceRouteRoomProvider>
                <Room />
              </SpaceRouteRoomProvider>
            }
          />
        </Route>
        <Route
          path={EXPLORE_PATH}
          element={
            <PageRoot
              collapsibleNav
              nav={
                <MobileFriendlyPageNav path={EXPLORE_PATH}>
                  <Explore />
                </MobileFriendlyPageNav>
              }
            >
              <Outlet />
            </PageRoot>
          }
        >
          {mobile ? null : (
            <Route
              index
              loader={() => redirect(getExploreFeaturedPath())}
              element={<WelcomePage />}
            />
          )}
          <Route path={_FEATURED_PATH} element={<FeaturedRooms />} />
          <Route path={_SERVER_PATH} element={<PublicRooms />} />
        </Route>
        <Route
          path={CREATE_PATH}
          element={
            <PageRoot
              collapsibleNav
              nav={
                <MobileFriendlyPageNav path={[]}>
                  <Home />
                </MobileFriendlyPageNav>
              }
            >
              <Create />
            </PageRoot>
          }
        />
        {/*
          Approvals has no list of its own; it borrows the Home list as its
          column (D2) so the bottom bar never disappears. On Mobile the page
          stands alone with its Back button, as a room does.
        */}
        <Route
          path={APPROVALS_PATH}
          element={
            <PageRoot
              collapsibleNav
              nav={
                <MobileFriendlyPageNav path={[]}>
                  <Home />
                </MobileFriendlyPageNav>
              }
            >
              <Approvals />
            </PageRoot>
          }
        />
        <Route
          path={INBOX_PATH}
          element={
            <PageRoot
              collapsibleNav
              nav={
                <MobileFriendlyPageNav path={INBOX_PATH}>
                  <Inbox />
                </MobileFriendlyPageNav>
              }
            >
              <Outlet />
            </PageRoot>
          }
        >
          {mobile ? null : (
            <Route
              index
              loader={() => redirect(getInboxNotificationsPath())}
              element={<WelcomePage />}
            />
          )}
          <Route path={_NOTIFICATIONS_PATH} element={<Notifications />} />
          <Route path={_INVITES_PATH} element={<Invites />} />
          <Route path={_LATER_PATH} element={<Later />} />
        </Route>
      </Route>
      <Route path="/*" element={<p>Page not found</p>} />
    </Route>
  );

  if (hashRouter?.enabled) {
    return createHashRouter(routes, { basename: hashRouter.basename });
  }
  return createBrowserRouter(routes, {
    basename: import.meta.env.BASE_URL,
  });
};
