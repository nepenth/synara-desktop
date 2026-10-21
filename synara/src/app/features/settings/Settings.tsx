import React, { useMemo, useState } from 'react';
import {
  Avatar,
  Button,
  Icon,
  IconButton,
  Icons,
  Overlay,
  OverlayBackdrop,
  OverlayCenter,
  Text,
} from 'folds';
import FocusTrap from '../../components/FocusTrap';
import { General } from './general';
import { AppearancePage as AppearanceSettings } from './appearance';
import { PageRoot } from '../../components/page';
import { SettingsNav, SettingsNavGroup } from '../../components/settings-layout';
import { ScreenSize, useScreenSizeContext } from '../../hooks/useScreenSize';
import { Account } from './account';
import { useUserProfile } from '../../hooks/useUserProfile';
import { getMxIdLocalPart } from '../../utils/matrix';
import { UserAvatar } from '../../components/user-avatar';
import { nameInitials } from '../../utils/common';
import { Notifications } from './notifications';
import { Devices } from './devices';
import { DeveloperTools } from './developer-tools';
import { Diagnostics } from './diagnostics';
import { About } from './about';
import { UseStateProvider } from '../../components/UseStateProvider';
import { stopPropagation } from '../../utils/keyboard';
import { LogoutDialog } from '../../components/LogoutDialog';
import * as depthCss from '../../styles/Depth.css';
import { isDesktopPlatform } from '../../platform';
import { getSafeMyUserId } from '../../state/nativeIdentity';
import { SettingsPages } from './settingsPages';
import { APP_SETTINGS_SEARCH_INDEX } from './settingsSearchIndex';

export { SettingsPages };

const useSettingsNavGroups = (): SettingsNavGroup<SettingsPages>[] =>
  useMemo(
    () => [
      {
        label: 'Preferences',
        items: [
          {
            id: SettingsPages.GeneralPage,
            name: 'General',
            icon: Icons.Setting,
            keywords: [
              'widgets',
              'calls',
              'date',
              'time',
              'editor',
              'enter',
              'markdown',
              'storage',
              'updates',
            ],
          },
          {
            id: SettingsPages.AppearancePage,
            name: 'Appearance',
            icon: Icons.Sun,
            keywords: [
              'theme',
              'dark',
              'light',
              'color',
              'accent',
              'font',
              'text',
              'density',
              'zoom',
            ],
          },
          {
            id: SettingsPages.NotificationPage,
            name: 'Notifications',
            icon: Icons.Bell,
            keywords: ['sound', 'mentions', 'keywords', 'agent', 'approvals', 'system', 'badge'],
          },
        ],
      },
      {
        label: 'Account',
        items: [
          {
            id: SettingsPages.AccountPage,
            name: 'Account',
            icon: Icons.User,
            keywords: ['profile', 'name', 'avatar', 'email', 'contact', 'ignored', 'matrix id'],
          },
          {
            id: SettingsPages.DevicesPage,
            name: 'Devices',
            icon: Icons.Monitor,
            keywords: [
              'sessions',
              'verification',
              'security',
              'backup',
              'recovery',
              'keys',
              'encryption',
            ],
          },
        ],
      },
      {
        label: 'Advanced',
        items: [
          ...(isDesktopPlatform()
            ? [
                {
                  id: SettingsPages.DiagnosticsPage,
                  name: 'Diagnostics',
                  icon: Icons.File,
                  keywords: ['logs', 'report', 'capture', 'support'],
                },
              ]
            : []),
          {
            id: SettingsPages.DeveloperToolsPage,
            name: 'Developer Tools',
            icon: Icons.Terminal,
            keywords: ['account data', 'debug', 'json'],
          },
          {
            id: SettingsPages.AboutPage,
            name: 'About',
            icon: Icons.Info,
            keywords: ['version', 'build', 'credits', 'license'],
          },
        ],
      },
    ],
    []
  );

type SettingsProps = {
  initialPage?: SettingsPages;
  requestClose: () => void;
};
export function Settings({ initialPage, requestClose }: SettingsProps) {
  const userId = getSafeMyUserId();
  const profile = useUserProfile(userId);
  const displayName = profile.displayName ?? getMxIdLocalPart(userId) ?? userId;
  const avatarUrl = profile.avatarUrl;

  const screenSize = useScreenSizeContext();
  const [activePage, setActivePage] = useState<SettingsPages | undefined>(() => {
    if (initialPage) return initialPage;
    return screenSize === ScreenSize.Mobile ? undefined : SettingsPages.GeneralPage;
  });
  const navGroups = useSettingsNavGroups();

  const handlePageRequestClose = () => {
    if (screenSize === ScreenSize.Mobile) {
      setActivePage(undefined);
      return;
    }
    requestClose();
  };

  return (
    <PageRoot
      nav={
        screenSize === ScreenSize.Mobile && activePage !== undefined ? undefined : (
          <SettingsNav
            header={
              <>
                <Avatar size="200" radii="300">
                  <UserAvatar
                    userId={userId}
                    src={avatarUrl}
                    renderFallback={() => <Text size="H6">{nameInitials(displayName)}</Text>}
                  />
                </Avatar>
                <Text size="H4" truncate>
                  Settings
                </Text>
              </>
            }
            headerAfter={
              screenSize === ScreenSize.Mobile ? (
                <IconButton
                  className={depthCss.quietInteractiveSurface}
                  onClick={requestClose}
                  variant="Surface"
                  fill="None"
                  aria-label="Close"
                >
                  <Icon src={Icons.Cross} />
                </IconButton>
              ) : undefined
            }
            groups={navGroups}
            searchIndex={APP_SETTINGS_SEARCH_INDEX}
            active={activePage}
            onSelect={setActivePage}
            footer={
              <UseStateProvider initial={false}>
                {(logout, setLogout) => (
                  <>
                    <Button
                      className={depthCss.quietInteractiveSurface}
                      size="300"
                      variant="Critical"
                      fill="None"
                      radii="300"
                      style={{ width: '100%', justifyContent: 'flex-start' }}
                      before={<Icon src={Icons.Power} size="100" />}
                      onClick={() => setLogout(true)}
                    >
                      <Text size="B400">Logout</Text>
                    </Button>
                    {logout && (
                      <Overlay open backdrop={<OverlayBackdrop />}>
                        <OverlayCenter>
                          <FocusTrap
                            focusTrapOptions={{
                              onDeactivate: () => setLogout(false),
                              clickOutsideDeactivates: true,
                              escapeDeactivates: stopPropagation,
                            }}
                          >
                            <LogoutDialog handleClose={() => setLogout(false)} />
                          </FocusTrap>
                        </OverlayCenter>
                      </Overlay>
                    )}
                  </>
                )}
              </UseStateProvider>
            }
          />
        )
      }
    >
      {activePage === SettingsPages.GeneralPage && (
        <General requestClose={handlePageRequestClose} />
      )}
      {activePage === SettingsPages.AppearancePage && (
        <AppearanceSettings requestClose={handlePageRequestClose} />
      )}
      {activePage === SettingsPages.AccountPage && (
        <Account requestClose={handlePageRequestClose} />
      )}
      {activePage === SettingsPages.NotificationPage && (
        <Notifications requestClose={handlePageRequestClose} />
      )}
      {activePage === SettingsPages.DevicesPage && (
        <Devices requestClose={handlePageRequestClose} />
      )}
      {activePage === SettingsPages.DiagnosticsPage && (
        <Diagnostics requestClose={handlePageRequestClose} />
      )}
      {activePage === SettingsPages.DeveloperToolsPage && (
        <DeveloperTools requestClose={handlePageRequestClose} />
      )}
      {activePage === SettingsPages.AboutPage && <About requestClose={handlePageRequestClose} />}
    </PageRoot>
  );
}
