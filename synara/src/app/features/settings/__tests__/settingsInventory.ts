/**
 * The pinned list of every App Settings row, the page and section it lives on,
 * what it reads or writes, and where it was before the 2026-11 regroup.
 *
 * `settingsInventory.test.ts` holds the search index and the page sources to
 * this list, and `e2e/settings-layout.spec.ts` holds the rendered pages to it,
 * so a setting cannot be dropped or moved without changing this file on
 * purpose. See docs/settings-information-architecture.md.
 */

/**
 * When a row renders. Rows without `when` render on every platform.
 * - desktop: the desktop shell (`isDesktopPlatform()`).
 * - synaraDesktop: the Synara desktop app (`isSynaraDesktop()`).
 * - linuxShell: the borderless Linux shell (`isLinuxDesktopShell()`).
 * - nativeSession: a native Matrix session (always, in the desktop app).
 * - nonNativeSession: never in the desktop app; kept for other builds.
 * - state: depends on account, device or permission state at runtime.
 */
export type InventoryCondition =
  'desktop' | 'synaraDesktop' | 'linuxShell' | 'nativeSession' | 'nonNativeSession' | 'state';

export type InventoryRow = {
  /** The rendered title (`data-setting-title`). */
  title: string;
  /**
   * What the control reads or writes. `settings.<key>` is the shared settings
   * atom, `platformSettings.<key>` the desktop platform settings atom; other
   * values describe an account, homeserver or shell call.
   */
  stores?: string | string[];
  /** Where it was before the regroup, when that differs from the section. */
  was?: string;
  when?: InventoryCondition;
  /** False for rows whose title is computed or not plain text, so not indexed. */
  indexed?: false;
  /** The title is drawn as a small label element, so the row has no data-setting-title. */
  labelTitle?: true;
};

export type InventorySection = {
  title: string;
  /** Where the section's rows were before the regroup, unless a row says otherwise. */
  was?: string;
  when?: InventoryCondition;
  rows: InventoryRow[];
};

export type InventoryPage = {
  page: string;
  group: 'App' | 'Chat' | 'Account' | 'Advanced';
  when?: InventoryCondition;
  sections: InventorySection[];
};

export const SETTINGS_INVENTORY: InventoryPage[] = [
  {
    page: 'General',
    group: 'App',
    sections: [
      {
        title: 'Updates',
        was: 'General › Updates',
        when: 'desktop',
        rows: [{ title: 'Updates', stores: 'desktop updater: check now, last result' }],
      },
      {
        title: 'Date & time',
        was: 'General › Date & time',
        rows: [
          { title: '24-hour time', stores: 'settings.hour24Clock' },
          { title: 'Date format', stores: 'settings.dateFormatString' },
        ],
      },
      {
        title: 'Keyboard shortcuts',
        was: 'General › Keyboard shortcuts',
        rows: [
          {
            title: 'Show Synara',
            stores: ['platformSettings.desktopShortcutShow', 'shell: global shortcuts'],
            indexed: false,
          },
          {
            title: 'Open Later',
            stores: ['platformSettings.desktopShortcutLater', 'shell: global shortcuts'],
            indexed: false,
          },
          {
            title: 'Open Notifications',
            stores: ['platformSettings.desktopShortcutNotifications', 'shell: global shortcuts'],
            indexed: false,
          },
          { title: 'Show or hide the room list', stores: 'read-only shortcut' },
          { title: 'Vim-style list keys', stores: 'platformSettings.vimListKeys' },
        ],
      },
    ],
  },
  {
    page: 'Appearance',
    group: 'App',
    sections: [
      {
        title: 'Theme',
        was: 'Appearance › Theme',
        rows: [
          { title: 'Appearance', stores: ['settings.useSystemTheme', 'settings.themeId'] },
          { title: 'Theme', stores: 'settings.themeId', when: 'state' },
          { title: 'Light theme', stores: 'settings.lightThemeId', when: 'state' },
          { title: 'Dark theme', stores: 'settings.darkThemeId', when: 'state' },
          {
            title: 'Follow system theme colors',
            stores: 'settings.followSystemPalette',
            when: 'state',
          },
          { title: 'Monochrome mode', stores: 'settings.monochromeMode' },
          { title: 'Base color', stores: 'settings.themeBaseColor' },
          { title: 'Accent color', stores: 'settings.customAccentColor' },
        ],
      },
      {
        title: 'Text',
        was: 'Appearance › Text',
        rows: [
          { title: 'Message text', stores: 'settings.messageTextTone' },
          {
            title: 'Interface font',
            stores: ['settings.interfaceFont', 'settings.interfaceFontFamily'],
          },
          {
            title: 'Monospace font',
            stores: ['settings.monospaceFont', 'settings.monospaceFontFamily'],
          },
          { title: 'Page zoom', stores: 'settings.pageZoom' },
        ],
      },
      {
        title: 'Message display',
        was: 'Appearance › Messages',
        rows: [
          { title: 'Message layout', stores: 'settings.messageLayout', when: 'nonNativeSession' },
          { title: 'Message spacing', stores: 'settings.messageSpacing' },
          {
            title: 'Thread display',
            stores: ['settings.threadDisplay', 'settings.threadDisplayChosen'],
            was: 'Appearance › Text',
          },
          { title: 'Legacy username color', stores: 'settings.legacyUsernameColor' },
        ],
      },
      {
        title: 'Window',
        was: 'Appearance › Window',
        when: 'linuxShell',
        rows: [
          { title: 'Tiling mode', stores: 'platformSettings.tilingMode' },
          { title: 'Hardware acceleration', stores: 'shell: rendering mode', when: 'state' },
        ],
      },
    ],
  },
  {
    page: 'Notifications',
    group: 'App',
    sections: [
      {
        title: 'System',
        was: 'Notifications › System',
        rows: [
          {
            title: 'Desktop notifications',
            stores: ['settings.showNotifications', 'system notification permission'],
          },
          {
            title: 'Notification permission',
            stores: 'system notification permission',
            when: 'state',
          },
          { title: 'Notification sound', stores: 'settings.isNotificationSounds' },
        ],
      },
      {
        title: 'Agent notifications',
        was: 'Notifications › Agent notifications',
        rows: [
          { title: 'Agent accounts', stores: 'account data: agent notification preferences' },
          {
            title: 'Notify for tool activity',
            stores: 'account data: agent notification preferences',
            when: 'state',
            indexed: false,
          },
          {
            title: 'Notify for commentary',
            stores: 'account data: agent notification preferences',
            when: 'state',
            indexed: false,
          },
          {
            title: 'Notify for final responses',
            stores: 'account data: agent notification preferences',
            when: 'state',
            indexed: false,
          },
          {
            title: 'Show approval details',
            stores: 'settings.showApprovalNotificationDetails',
            was: 'Notifications › System',
          },
        ],
      },
      {
        title: 'Direct messages',
        was: 'Notifications › Direct messages',
        rows: [
          { title: 'Unencrypted', stores: 'push rules: one-to-one', indexed: false },
          { title: 'Encrypted', stores: 'push rules: encrypted one-to-one', indexed: false },
        ],
      },
      {
        title: 'Rooms',
        was: 'Notifications › Rooms',
        rows: [
          { title: 'Unencrypted', stores: 'push rules: group', indexed: false },
          { title: 'Encrypted', stores: 'push rules: encrypted group', indexed: false },
        ],
      },
      {
        title: 'Mentions',
        was: 'Notifications › Mentions',
        rows: [
          { title: 'Mentions of your user ID', stores: 'push rules: user mention', indexed: false },
          {
            title: 'Contains your display name',
            stores: 'push rules: display name',
            indexed: false,
          },
          { title: 'Contains your username', stores: 'push rules: user name', indexed: false },
          { title: 'Mentions of @room', stores: 'push rules: room mention', indexed: false },
          { title: 'Contains @room', stores: 'push rules: @room', indexed: false },
        ],
      },
      {
        title: 'Keywords',
        was: 'Notifications › Keywords',
        rows: [],
      },
    ],
  },
  {
    page: 'Messages',
    group: 'Chat',
    sections: [
      {
        title: 'Composer',
        was: 'General › Composer',
        rows: [
          { title: 'Enter adds a new line', stores: 'settings.enterForNewline' },
          { title: 'Markdown formatting', stores: 'settings.isMarkdown' },
        ],
      },
      {
        title: 'Timeline',
        was: 'General › Messages',
        rows: [
          { title: 'Hide membership changes', stores: 'settings.hideMembershipEvents' },
          { title: 'Hide profile changes', stores: 'settings.hideNickAvatarEvents' },
        ],
      },
      {
        title: 'Drafts',
        was: 'General › Messages',
        rows: [
          {
            title: 'Sync drafts across devices',
            stores: ['settings.syncDrafts', 'core: drafts key state (recovery hint)'],
          },
        ],
      },
    ],
  },
  {
    page: 'Media & links',
    group: 'Chat',
    sections: [
      {
        title: 'Media',
        was: 'General › Messages',
        rows: [{ title: 'Load media automatically', stores: 'settings.mediaAutoLoad' }],
      },
      {
        title: 'GIFs',
        was: 'General › Messages',
        rows: [
          { title: 'GIF search', stores: 'settings.gifSearchEnabled' },
          { title: 'Play GIF links inline', stores: 'settings.inlineGifLinks' },
        ],
      },
      {
        title: 'Links',
        was: 'General › Messages',
        rows: [
          { title: 'Link previews', stores: 'settings.linkPreviews' },
          { title: 'Show X posts', stores: 'settings.xEmbeds' },
          { title: 'Shorten long links', stores: 'settings.shortenLinks' },
        ],
      },
    ],
  },
  {
    page: 'Privacy',
    group: 'Chat',
    sections: [
      {
        title: 'Activity',
        was: 'General › Composer',
        rows: [{ title: 'Hide typing & read receipts', stores: 'settings.hideActivity' }],
      },
      {
        title: 'Blocked users',
        was: 'Account › Blocked users',
        rows: [{ title: 'Block a user', stores: 'account data: m.ignored_user_list' }],
      },
    ],
  },
  {
    page: 'Account',
    group: 'Account',
    sections: [
      {
        title: 'Profile',
        was: 'Account › Profile',
        rows: [
          {
            title: 'Avatar',
            stores: 'homeserver profile: avatar',
            indexed: false,
            labelTitle: true,
          },
          {
            title: 'Display Name',
            stores: 'homeserver profile: display name',
            indexed: false,
            labelTitle: true,
          },
          { title: 'Presence', stores: 'homeserver presence', indexed: false, labelTitle: true },
          {
            title: 'Status',
            stores: 'homeserver presence status message',
            indexed: false,
            labelTitle: true,
          },
        ],
      },
      { title: 'Matrix ID', was: 'Account › Matrix ID', rows: [] },
      {
        title: 'Password',
        was: 'Account › Password',
        rows: [
          { title: 'Password', stores: 'homeserver account page (OAuth)', when: 'state' },
          { title: 'Change password', stores: 'homeserver password', when: 'state' },
        ],
      },
      {
        title: 'Contact',
        was: 'Account › Contact',
        rows: [{ title: 'Email', stores: 'homeserver 3PIDs' }],
      },
    ],
  },
  {
    page: 'Security & recovery',
    group: 'Account',
    sections: [
      {
        title: 'Verification',
        was: 'Security & recovery › Verification',
        rows: [
          { title: 'Verify this session', stores: 'cross-signing, device verification' },
          {
            title: 'Message history for new sessions',
            stores: 'read-only explanation',
            when: 'nativeSession',
          },
        ],
      },
      {
        title: 'Recovery',
        was: 'Security & recovery › Recovery',
        when: 'nativeSession',
        rows: [
          { title: 'Secure Backup & Recovery', stores: 'secret storage' },
          { title: 'Encryption Backup', stores: 'key backup' },
        ],
      },
      {
        title: 'Certificate identities',
        was: 'Security & recovery › Certificate identities',
        when: 'nativeSession',
        rows: [{ title: 'X.509 identity (experimental)', stores: 'X.509 trust store' }],
      },
      {
        title: 'Certificate-verified identities',
        was: 'Security & recovery › Certificate identities',
        when: 'state',
        rows: [],
      },
      {
        title: 'Danger zone',
        was: 'Security & recovery › Danger zone',
        when: 'state',
        rows: [{ title: 'Reset identity', stores: 'cross-signing identity reset' }],
      },
    ],
  },
  {
    page: 'Sessions',
    group: 'Account',
    sections: [
      { title: 'This session', was: 'Sessions › This session', rows: [] },
      {
        title: 'Other sessions',
        was: 'Sessions › Other sessions',
        when: 'state',
        rows: [{ title: 'Session dashboard', stores: 'homeserver account page', when: 'state' }],
      },
      {
        title: 'Export & import keys',
        was: 'Sessions › Export & import keys',
        rows: [
          { title: 'Export message keys', stores: 'room keys export file' },
          { title: 'Import message keys', stores: 'room keys import file' },
        ],
      },
    ],
  },
  {
    page: 'Advanced',
    group: 'Advanced',
    sections: [
      {
        title: 'Experimental',
        rows: [
          {
            title: 'Indexed message search',
            stores: 'settings.indexedMessageSearch',
            was: 'General › Messages',
          },
          {
            title: 'Show hidden events',
            stores: 'settings.showHiddenEvents',
            was: 'General › Messages',
          },
          {
            title: 'Experimental widgets',
            stores: ['settings.experimentalWidgetsEnabled', 'settings.agentWidgetEntries'],
            was: 'General › Widgets',
            when: 'synaraDesktop',
          },
          {
            title: 'Encrypted state events (experimental)',
            stores: 'settings.encryptedStateEvents',
            was: 'Developer tools › Options',
          },
        ],
      },
      {
        title: 'Calls',
        rows: [
          {
            title: 'Voice and video calls',
            stores: 'read-only: MatrixRTC availability',
            was: 'General › Calls',
            when: 'synaraDesktop',
          },
          {
            title: 'Call transport',
            stores: 'read-only: MatrixRTC transports, Refresh transport',
            was: 'Account › Profile (as "MatrixRTC")',
          },
        ],
      },
      {
        title: 'Developer',
        rows: [
          {
            title: 'Enable developer tools',
            stores: 'settings.developerTools',
            was: 'Developer tools › Options',
          },
          {
            title: 'Account data',
            stores: 'account data editor',
            was: 'Developer tools › Account data (row "Global")',
            when: 'nonNativeSession',
          },
        ],
      },
      {
        title: 'Maintenance',
        rows: [
          {
            title: 'Reload application',
            stores: 'clears navigation and notification caches, reloads',
            was: 'General › Storage',
          },
        ],
      },
    ],
  },
  {
    page: 'Diagnostics',
    group: 'Advanced',
    when: 'desktop',
    sections: [
      {
        title: 'Diagnostic capture',
        was: 'Diagnostics › Diagnostic capture',
        rows: [
          { title: 'Capture diagnostics', stores: 'diagnostics config' },
          { title: 'Performance', stores: 'diagnostics config' },
          { title: 'Session persistence', stores: 'diagnostics config' },
          { title: 'Room state and positioning', stores: 'diagnostics config' },
          { title: 'Performance overlay', stores: 'diagnostics config' },
        ],
      },
      {
        title: 'Stored report',
        was: 'Diagnostics › Stored report',
        rows: [
          { title: 'Local diagnostics', stores: 'diagnostics report', when: 'state' },
          // The same row while the report cannot be read.
          {
            title: 'Diagnostics unavailable',
            stores: 'diagnostics report',
            when: 'state',
            indexed: false,
          },
        ],
      },
      {
        title: 'Desktop integration',
        was: 'Diagnostics › Desktop integration',
        when: 'desktop',
        rows: [
          { title: 'Native session store', stores: 'read-only: session store' },
          { title: 'Tray', stores: 'read-only', indexed: false },
          { title: 'Notifications', stores: 'read-only', indexed: false },
          { title: 'Global shortcuts', stores: 'read-only', indexed: false },
          { title: 'File portal', stores: 'read-only', indexed: false },
          { title: 'Media portal', stores: 'read-only', indexed: false },
        ],
      },
      { title: 'Privacy', was: 'Diagnostics › Privacy', rows: [] },
    ],
  },
  {
    page: 'About',
    group: 'Advanced',
    sections: [
      // New in 3.2: this version's bundled "What's new" notes, read-only.
      { title: "What's new", was: "About › What's new", rows: [] },
      { title: 'Credits', was: 'About › Credits', rows: [] },
    ],
  },
];

/** The page and section names in nav order, for tests that walk them. */
export const INVENTORY_PAGE_NAMES = SETTINGS_INVENTORY.map((page) => page.page);
