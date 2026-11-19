import React, { useEffect, useMemo, useState } from 'react';
import { Avatar, Button, Icon, IconButton, Icons, Text } from 'folds';
import { General } from './general';
import { AppearancePage as AppearanceSettings } from './appearance';
import { PageRoot } from '../../components/page';
import { revealSetting, SettingsNav, SettingsNavGroup } from '../../components/settings-layout';
import { ScreenSize, useScreenSizeContext } from '../../hooks/useScreenSize';
import { Account } from './account';
import { useUserProfile } from '../../hooks/useUserProfile';
import { getMxIdLocalPart } from '../../utils/matrix';
import { UserAvatar } from '../../components/user-avatar';
import { nameInitials } from '../../utils/common';
import { Notifications } from './notifications';
import { Devices, SecurityRecovery } from './devices';
import { Messages } from './messages/Messages';
import { MediaLinks } from './media/MediaLinks';
import { Privacy } from './privacy/Privacy';
import { Advanced } from './advanced/Advanced';
import { Diagnostics } from './diagnostics';
import { About } from './about';
import { UseStateProvider } from '../../components/UseStateProvider';
import { LogoutDialog } from '../../components/LogoutDialog';
import * as depthCss from '../../styles/Depth.css';
import { isDesktopPlatform } from '../../platform';
import { isLinuxDesktopShell } from '../../hooks/useTilingMode';
import { isSynaraDesktop } from '../../utils/desktop';
import { isNativeMatrixSession } from '../verification/nativeVerification';
import { getSafeMyUserId } from '../../state/nativeIdentity';
import { lastViewedSettingsPage, rememberSettingsPage, SettingsPages } from './settingsPages';
import { appSettingsSearchIndex, SettingsIndexCondition } from './settingsSearchIndex';

export { SettingsPages };

/** Whether rows indexed with a condition render here, as the pages decide. */
const indexConditionHolds = (condition: SettingsIndexCondition): boolean => {
  switch (condition) {
    case 'desktop':
      return isDesktopPlatform();
    case 'synaraDesktop':
      return isSynaraDesktop();
    case 'linuxShell':
      return isLinuxDesktopShell();
    case 'nativeSession':
      return isNativeMatrixSession();
    case 'nonNativeSession':
      return !isNativeMatrixSession();
    default:
      return true;
  }
};

/**
 * Four groups, by task: the app itself, chatting, your account, and the
 * advanced and support pages. Everyday pages come before advanced ones.
 */
const useSettingsNavGroups = (): SettingsNavGroup<SettingsPages>[] =>
  useMemo(
    () => [
      {
        label: 'App',
        items: [
          {
            id: SettingsPages.GeneralPage,
            name: 'General',
            icon: Icons.Setting,
            keywords: ['date', 'time', 'clock', 'shortcuts', 'keyboard', 'updates', 'version'],
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
              'threads',
              'layout',
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
        label: 'Chat',
        items: [
          {
            id: SettingsPages.MessagesPage,
            name: 'Messages',
            icon: Icons.Message,
            keywords: ['composer', 'editor', 'enter', 'markdown', 'timeline', 'drafts'],
          },
          {
            id: SettingsPages.MediaPage,
            name: 'Media & links',
            icon: Icons.Photo,
            keywords: ['gif', 'images', 'video', 'previews', 'url', 'embeds'],
          },
          {
            id: SettingsPages.PrivacyPage,
            name: 'Privacy',
            icon: Icons.EyeBlind,
            keywords: ['typing', 'read receipts', 'block', 'ignore'],
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
            keywords: ['profile', 'name', 'avatar', 'email', 'contact', 'password', 'matrix id'],
          },
          {
            id: SettingsPages.SecurityPage,
            name: 'Security & recovery',
            icon: Icons.ShieldLock,
            keywords: [
              'verification',
              'verify',
              'security',
              'backup',
              'recovery key',
              'keys',
              'encryption',
              'reset identity',
            ],
          },
          {
            id: SettingsPages.DevicesPage,
            name: 'Sessions',
            icon: Icons.Monitor,
            keywords: ['devices', 'sessions', 'log out', 'export keys', 'import keys'],
          },
        ],
      },
      {
        label: 'Advanced',
        items: [
          {
            id: SettingsPages.AdvancedPage,
            name: 'Advanced',
            icon: Icons.Terminal,
            keywords: [
              'experimental',
              'beta',
              'widgets',
              'calls',
              'developer',
              'devtools',
              'reload',
              'cache',
            ],
          },
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
  /** The page to open on; otherwise the last page viewed, then General. */
  initialPage?: SettingsPages;
  /** A row or section title on that page to scroll to and highlight. */
  initialSetting?: string;
  requestClose: () => void;
};
export function Settings({ initialPage, initialSetting, requestClose }: SettingsProps) {
  const userId = getSafeMyUserId();
  const profile = useUserProfile(userId);
  const displayName = profile.displayName ?? getMxIdLocalPart(userId) ?? userId;
  const avatarUrl = profile.avatarUrl;

  const screenSize = useScreenSizeContext();
  const navGroups = useSettingsNavGroups();
  const searchIndex = useMemo(() => appSettingsSearchIndex(indexConditionHolds), []);
  const [activePage, setActivePage] = useState<SettingsPages | undefined>(() => {
    const available = (page: SettingsPages | undefined) =>
      page !== undefined && navGroups.some((group) => group.items.some((item) => item.id === page));
    if (available(initialPage)) return initialPage;
    // On a phone-sized window the page list comes first.
    if (screenSize === ScreenSize.Mobile) return undefined;
    const last = lastViewedSettingsPage();
    return available(last) ? last : SettingsPages.GeneralPage;
  });

  useEffect(() => {
    rememberSettingsPage(activePage);
  }, [activePage]);

  useEffect(() => {
    if (initialSetting) revealSetting(initialSetting);
  }, [initialSetting]);

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
            searchIndex={searchIndex}
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
                      <Text size="B400">Log out</Text>
                    </Button>
                    {logout && <LogoutDialog onClose={() => setLogout(false)} />}
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
      {activePage === SettingsPages.MessagesPage && (
        <Messages requestClose={handlePageRequestClose} />
      )}
      {activePage === SettingsPages.MediaPage && (
        <MediaLinks requestClose={handlePageRequestClose} />
      )}
      {activePage === SettingsPages.PrivacyPage && (
        <Privacy requestClose={handlePageRequestClose} />
      )}
      {activePage === SettingsPages.SecurityPage && (
        <SecurityRecovery requestClose={handlePageRequestClose} />
      )}
      {activePage === SettingsPages.DevicesPage && (
        <Devices requestClose={handlePageRequestClose} />
      )}
      {activePage === SettingsPages.DiagnosticsPage && (
        <Diagnostics requestClose={handlePageRequestClose} />
      )}
      {activePage === SettingsPages.AdvancedPage && (
        <Advanced requestClose={handlePageRequestClose} />
      )}
      {activePage === SettingsPages.AboutPage && <About requestClose={handlePageRequestClose} />}
    </PageRoot>
  );
}
