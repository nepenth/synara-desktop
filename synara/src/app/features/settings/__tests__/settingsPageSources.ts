import { SettingsPages } from '../settingsPages';

/**
 * The source files whose rows each App Settings page renders, including the
 * shared components it renders from `components/` and other features, and the
 * components scanned in each file. Used by the search-index and inventory tests.
 */
export type PageSource = {
  file: string;
  /** InfoCard is scanned only where a page uses one as a settings row. */
  infoCards?: boolean;
};

const settings = (path: string): PageSource => ({ file: `src/app/features/settings/${path}` });

export const APP_SETTINGS_PAGE_SOURCES: Record<SettingsPages, PageSource[]> = {
  [SettingsPages.GeneralPage]: [
    settings('general/General.tsx'),
    { file: 'src/app/features/desktop-updater/DesktopUpdaterProvider.tsx' },
  ],
  [SettingsPages.AppearancePage]: [settings('appearance/Appearance.tsx')],
  [SettingsPages.NotificationPage]: [
    settings('notifications/Notifications.tsx'),
    settings('notifications/SystemNotification.tsx'),
    settings('notifications/AgentNotifications.tsx'),
  ],
  [SettingsPages.MessagesPage]: [settings('messages/Messages.tsx')],
  [SettingsPages.MediaPage]: [settings('media/MediaLinks.tsx')],
  [SettingsPages.PrivacyPage]: [
    settings('privacy/Privacy.tsx'),
    settings('account/IgnoredUserList.tsx'),
  ],
  [SettingsPages.AccountPage]: [
    settings('account/Account.tsx'),
    settings('account/Profile.tsx'),
    settings('account/MatrixId.tsx'),
    settings('account/Password.tsx'),
    settings('account/ContactInfo.tsx'),
  ],
  [SettingsPages.SecurityPage]: [
    settings('devices/Devices.tsx'),
    settings('devices/Verification.tsx'),
    settings('devices/X509IdentityCard.tsx'),
    { file: 'src/app/components/SecretStorage.tsx', infoCards: true },
    { file: 'src/app/components/BackupRestore.tsx', infoCards: true },
  ],
  [SettingsPages.DevicesPage]: [
    settings('devices/Devices.tsx'),
    settings('devices/DeviceTile.tsx'),
    settings('devices/OtherDevices.tsx'),
    settings('devices/LocalBackup.tsx'),
  ],
  [SettingsPages.AdvancedPage]: [
    settings('advanced/Advanced.tsx'),
    settings('advanced/AccountData.tsx'),
  ],
  [SettingsPages.DiagnosticsPage]: [
    settings('diagnostics/Diagnostics.tsx'),
    settings('diagnostics/DesktopIntegration.tsx'),
  ],
  [SettingsPages.AboutPage]: [settings('about/About.tsx')],
};

/**
 * Settings page files that render both Security & recovery and Sessions:
 * a title from one of these may belong to either page.
 */
export const SHARED_PAGE_FILES: Record<string, SettingsPages[]> = {
  'src/app/features/settings/devices/Devices.tsx': [
    SettingsPages.SecurityPage,
    SettingsPages.DevicesPage,
  ],
};
