import type { SettingsSearchEntry } from '../../components/settings-layout/settingsSearch';
import { SettingsPages } from './settingsPages';

/**
 * When an indexed row renders, matching the condition on the row itself, so
 * search never offers a row the page will not show. See `Settings.tsx`.
 */
export type SettingsIndexCondition =
  'desktop' | 'synaraDesktop' | 'linuxShell' | 'nativeSession' | 'nonNativeSession';

export type AppSettingsSearchEntry = SettingsSearchEntry<SettingsPages> & {
  section: string;
  when?: SettingsIndexCondition;
};

const P = SettingsPages;

/**
 * Every row and section the App Settings pages render, for settings search.
 *
 * Titles and descriptions must match what the page renders; the search result
 * scrolls to the row with that title. `settingsSearchIndex.test.ts` scans the
 * pages and fails when a rendered title is missing here or a description
 * differs, and `settingsInventory.test.ts` pins every entry's page and section.
 */
export const APP_SETTINGS_SEARCH_INDEX: AppSettingsSearchEntry[] = [
  // General
  {
    page: P.GeneralPage,
    section: 'Updates',
    title: 'Updates',
    description: 'Check for a new version of Synara.',
    synonyms: ['update', 'upgrade', 'version', 'release', 'software updates'],
    when: 'desktop',
  },
  {
    page: P.GeneralPage,
    section: 'Date & time',
    title: 'Date & time',
    synonyms: ['clock', 'format'],
  },
  {
    page: P.GeneralPage,
    section: 'Date & time',
    title: '24-hour time',
    description: 'Show times on a 24-hour clock.',
    synonyms: ['clock', 'am', 'pm', 'military', 'time format'],
  },
  {
    page: P.GeneralPage,
    section: 'Date & time',
    title: 'Date format',
    description: 'How dates appear in the timeline.',
    synonyms: ['day', 'month', 'year'],
  },
  {
    page: P.GeneralPage,
    section: 'Keyboard shortcuts',
    title: 'Keyboard shortcuts',
    description:
      'Open Synara screens from anywhere. These keys stop working in other apps while Synara runs; clear a field to turn that shortcut off.',
    synonyms: ['keyboard', 'hotkeys', 'shortcuts', 'keybindings', 'desktop shortcuts', 'global'],
  },
  {
    page: P.GeneralPage,
    section: 'Keyboard shortcuts',
    title: 'Show or hide the room list',
    description:
      'Works in the Synara window, also while typing. On macOS it is in the View menu too.',
    synonyms: ['sidebar', 'room list', 'collapse', 'hide', 'navigation pane', 'shortcut'],
  },
  {
    page: P.GeneralPage,
    section: 'Keyboard shortcuts',
    title: 'Vim-style list keys',
    description: 'j and k move through the room list, g g goes to the top, G to the bottom.',
    synonyms: ['vim', 'hjkl', 'keyboard', 'room list', 'tiling', 'navigation keys'],
  },

  // Appearance
  {
    page: P.AppearancePage,
    section: 'Theme',
    title: 'Theme',
    description: 'Which light theme to use.',
    synonyms: ['colors', 'dark theme', 'night', 'system theme'],
  },
  {
    page: P.AppearancePage,
    section: 'Theme',
    title: 'Appearance',
    description: 'Follow the system setting, or keep Synara light or dark.',
    synonyms: ['dark mode', 'light mode', 'night mode', 'automatic', 'os', 'system theme'],
  },
  {
    page: P.AppearancePage,
    section: 'Theme',
    title: 'Light theme',
    description: 'Used while your system is in light mode.',
    synonyms: ['day'],
  },
  {
    page: P.AppearancePage,
    section: 'Theme',
    title: 'Dark theme',
    description: 'Used while your system is in dark mode.',
    synonyms: ['night'],
  },
  {
    page: P.AppearancePage,
    section: 'Theme',
    title: 'Follow system theme colors',
    description: 'Use the colors of your desktop theme, or your macOS accent color, kept readable.',
    synonyms: [
      'omarchy',
      'macos accent',
      'system accent',
      'desktop theme',
      'system colors',
      'palette',
      'wallpaper',
      'colour scheme',
      'color scheme',
    ],
  },
  {
    page: P.AppearancePage,
    section: 'Theme',
    title: 'Monochrome mode',
    description: 'Render the interface in greyscale, keeping only the accent color.',
    synonyms: ['greyscale', 'grayscale', 'black and white'],
  },
  {
    page: P.AppearancePage,
    section: 'Theme',
    title: 'Base color',
    description:
      'Tints the side panels, room list and chat. Lightness is mapped to the stacked greys, not used as a fill.',
    synonyms: ['background', 'tint', 'colour'],
  },
  {
    page: P.AppearancePage,
    section: 'Theme',
    title: 'Accent color',
    description: 'Buttons, links and selected items. Theme default until you pick a color.',
    synonyms: ['highlight', 'primary', 'colour'],
  },
  {
    page: P.AppearancePage,
    section: 'Text',
    title: 'Text',
    synonyms: ['font', 'typography', 'size'],
  },
  {
    page: P.AppearancePage,
    section: 'Text',
    title: 'Message text',
    description:
      'Adjust message prose brightness without changing navigation, metadata, or code colors.',
    synonyms: ['brightness', 'contrast', 'tone', 'readability'],
  },
  {
    page: P.AppearancePage,
    section: 'Text',
    title: 'Interface font',
    description:
      'The font for menus, lists and messages. Custom takes font families installed on this device, separated by commas; the default font covers anything they lack.',
    synonyms: ['font', 'typeface', 'typography', 'family', 'ui font', 'sans'],
  },
  {
    page: P.AppearancePage,
    section: 'Text',
    title: 'Monospace font',
    description:
      'The font for code and other fixed-width text. Custom takes font families installed on this device, separated by commas.',
    synonyms: ['font', 'code font', 'fixed width', 'mono', 'typeface', 'programming'],
  },
  {
    page: P.AppearancePage,
    section: 'Text',
    title: 'Page zoom',
    description: 'Scale the whole interface, from 75% to 200%. Press Enter to apply.',
    synonyms: ['zoom', 'scale', 'size', 'bigger', 'smaller', 'font size'],
  },
  {
    page: P.AppearancePage,
    section: 'Message display',
    title: 'Message display',
    synonyms: ['timeline', 'chat', 'messages'],
  },
  {
    page: P.AppearancePage,
    section: 'Message display',
    title: 'Message layout',
    description: 'Modern, compact or bubble message layout.',
    synonyms: ['layout', 'bubbles', 'compact', 'irc', 'density'],
    when: 'nonNativeSession',
  },
  {
    page: P.AppearancePage,
    section: 'Message display',
    title: 'Message spacing',
    description: 'Space between messages in the timeline.',
    synonyms: ['spacing', 'density', 'padding', 'compact'],
  },
  {
    page: P.AppearancePage,
    section: 'Message display',
    title: 'Thread display',
    description:
      'Open threads inline beneath their first message, in a side panel beside the room, or in place of the room.',
    synonyms: ['thread', 'threads', 'replies', 'inline', 'side panel', 'full view', 'pane'],
  },
  {
    page: P.AppearancePage,
    section: 'Message display',
    title: 'Legacy username color',
    description: 'Color sender names with the legacy per-user palette instead of the theme accent.',
    synonyms: ['username', 'name color', 'sender', 'colour'],
  },
  {
    page: P.AppearancePage,
    section: 'Window',
    title: 'Window',
    synonyms: ['title bar', 'window manager', 'linux'],
    when: 'linuxShell',
  },
  {
    page: P.AppearancePage,
    section: 'Window',
    title: 'Tiling mode',
    description:
      'For tiling window managers such as Hyprland or Sway: no in-app title bar. Linux only.',
    synonyms: [
      'hyprland',
      'sway',
      'i3',
      'niri',
      'omarchy',
      'tiling',
      'title bar',
      'titlebar',
      'window manager',
      'decorations',
    ],
    when: 'linuxShell',
  },
  {
    page: P.AppearancePage,
    section: 'Window',
    title: 'Hardware acceleration',
    description:
      'Draw the interface with your graphics card. Full is the fastest. Balanced avoids the display path that most often fails on NVIDIA. Off draws on the CPU only and is the slowest: use it if the window flickers, goes blank or the app closes on start. Applies after you restart Synara.',
    synonyms: [
      'gpu',
      'graphics card',
      'rendering',
      'nvidia',
      'wayland',
      'flicker',
      'blank window',
      'performance',
      'software rendering',
    ],
    when: 'linuxShell',
  },

  // Notifications
  {
    page: P.NotificationPage,
    section: 'System',
    title: 'System',
    synonyms: ['desktop', 'os', 'alerts'],
  },
  {
    page: P.NotificationPage,
    section: 'System',
    title: 'Desktop notifications',
    description:
      'Show desktop notifications when messages arrive. Approval requests alert even when this is off.',
    synonyms: ['alerts', 'popups', 'banners', 'notify'],
  },
  {
    page: P.NotificationPage,
    section: 'System',
    title: 'Notification permission',
    description:
      'On supported macOS builds, requesting permission also lets you opt into Critical Alerts for verified agent approvals. Your macOS notification settings control which alerts can be delivered.',
    synonyms: ['permission', 'allow', 'critical alerts'],
  },
  {
    page: P.NotificationPage,
    section: 'System',
    title: 'Notification sound',
    description: 'Play a sound when a message arrives.',
    synonyms: ['sound', 'audio', 'ping', 'chime', 'mute'],
  },
  {
    page: P.NotificationPage,
    section: 'Agent notifications',
    title: 'Agent notifications',
    description:
      'Agent accounts and categories save to your account and apply on every Synara client. Show approval details applies to this device only.',
    synonyms: ['agents', 'bots', 'approvals', 'tool activity', 'commentary'],
  },
  {
    page: P.NotificationPage,
    section: 'Agent notifications',
    title: 'Agent accounts',
    description:
      'Add each agent’s full Matrix user ID. These rules apply only to the listed agents. Once you add an agent, only listed agents can raise urgent approval alerts, even in muted rooms; with no agents listed, approval alerts follow each room’s notification settings.',
    synonyms: ['agents', 'bots'],
  },
  {
    page: P.NotificationPage,
    section: 'Agent notifications',
    title: 'Show approval details',
    description:
      'Show the reason and requested command in agent approval notifications. When off, approval notifications only name the room.',
    synonyms: ['approval', 'agent', 'hermes', 'command', 'privacy', 'preview'],
  },
  {
    page: P.NotificationPage,
    section: 'Direct messages',
    title: 'Direct messages',
    description: 'How much to notify for one-to-one conversations.',
    synonyms: ['dm', '1-to-1', 'private', 'all messages', 'encrypted'],
  },
  {
    page: P.NotificationPage,
    section: 'Rooms',
    title: 'Rooms',
    description: 'How much to notify for group rooms.',
    synonyms: ['groups', 'channels', 'encrypted', 'e2ee', 'all messages'],
  },
  {
    page: P.NotificationPage,
    section: 'Mentions',
    title: 'Mentions',
    description: 'Always notify when a message mentions you.',
    synonyms: ['mention', 'ping', 'at', '@room', 'display name', 'username'],
  },
  {
    page: P.NotificationPage,
    section: 'Keywords',
    title: 'Keywords',
    description: 'Notify when a message contains one of these words.',
    synonyms: ['keyword', 'words', 'highlights', 'add keyword'],
  },

  // Messages
  {
    page: P.MessagesPage,
    section: 'Composer',
    title: 'Composer',
    synonyms: ['editor', 'typing', 'compose', 'message box'],
  },
  {
    page: P.MessagesPage,
    section: 'Composer',
    title: 'Enter adds a new line',
    description: 'Enter starts a new line and Ctrl+Enter sends the message.',
    synonyms: ['send', 'keyboard', 'return', 'shift', 'newline'],
  },
  {
    page: P.MessagesPage,
    section: 'Composer',
    title: 'Markdown formatting',
    description: 'Format messages with Markdown.',
    synonyms: ['bold', 'italic', 'code', 'formatting'],
  },
  {
    page: P.MessagesPage,
    section: 'Timeline',
    title: 'Timeline',
    synonyms: ['events', 'history'],
  },
  {
    page: P.MessagesPage,
    section: 'Timeline',
    title: 'Hide membership changes',
    description: 'Do not show join, leave, and invite events in the timeline.',
    synonyms: ['joins', 'leaves', 'invites', 'membership'],
  },
  {
    page: P.MessagesPage,
    section: 'Timeline',
    title: 'Hide profile changes',
    description: 'Do not show display name and avatar changes in the timeline.',
    synonyms: ['display name', 'avatar', 'profile'],
  },
  {
    page: P.MessagesPage,
    section: 'Drafts',
    title: 'Drafts',
    synonyms: ['unsent', 'draft'],
  },
  {
    page: P.MessagesPage,
    section: 'Drafts',
    title: 'Sync drafts across devices',
    description:
      'Unsent messages follow you to your other Synara devices, end-to-end encrypted. Devices need your recovery key or another verified device to read them.',
    synonyms: ['draft', 'unsent', 'composer', 'sync', 'devices', 'continue'],
  },

  // Media & links
  {
    page: P.MediaPage,
    section: 'Media',
    title: 'Media',
    synonyms: ['images', 'pictures', 'video'],
  },
  {
    page: P.MediaPage,
    section: 'Media',
    title: 'Load media automatically',
    description:
      'Download and show images and video previews as messages arrive. When off, media loads on request.',
    synonyms: ['images', 'pictures', 'video', 'previews', 'autoload', 'data'],
  },
  {
    page: P.MediaPage,
    section: 'GIFs',
    title: 'GIFs',
    synonyms: ['animated', 'giphy', 'klipy'],
  },
  {
    page: P.MediaPage,
    section: 'GIFs',
    title: 'GIF search',
    description: 'Show GIF search in the message composer.',
    synonyms: ['gif', 'giphy', 'klipy', 'animated', 'gif picker', 'composer'],
  },
  {
    page: P.MediaPage,
    section: 'GIFs',
    title: 'Play GIF links inline',
    description:
      'Show Klipy and GIPHY links as GIFs. Your device loads them from the provider, which can see your IP address.',
    synonyms: ['gif', 'klipy', 'giphy', 'animated', 'embed'],
  },
  {
    page: P.MediaPage,
    section: 'Links',
    title: 'Links',
    synonyms: ['url', 'web'],
  },
  {
    page: P.MediaPage,
    section: 'Links',
    title: 'Link previews',
    description:
      'When a message has a link, Synara fetches the page from this device to show a preview. The site can see your IP address.',
    synonyms: ['link', 'url', 'preview', 'unfurl', 'embed', 'open graph'],
  },
  {
    page: P.MediaPage,
    section: 'Links',
    title: 'Show X posts',
    description:
      'Show the post and play its video for x.com links. Your device contacts X, which can see your IP address.',
    synonyms: ['x', 'twitter', 'tweet', 'video', 'embed'],
  },
  {
    page: P.MediaPage,
    section: 'Links',
    title: 'Shorten long links',
    description:
      'Show a long web address as its site and path. The link still opens the full address, shown when you point at it.',
    synonyms: ['link', 'url', 'shorten', 'truncate', 'tracking', 'long'],
  },

  // Privacy
  {
    page: P.PrivacyPage,
    section: 'Activity',
    title: 'Activity',
    synonyms: ['typing', 'receipts', 'presence'],
  },
  {
    page: P.PrivacyPage,
    section: 'Activity',
    title: 'Hide typing & read receipts',
    description:
      "Don't send typing status, and hide who is typing and who has read a room. Reading still marks rooms as read privately, so badges clear and your other devices stay in sync.",
    synonyms: ['privacy', 'typing indicator', 'read receipts', 'seen', 'read status'],
  },
  {
    page: P.PrivacyPage,
    section: 'Blocked users',
    title: 'Blocked users',
    description: "Blocked users can't message or invite you.",
    synonyms: ['ignore', 'ignored users', 'block', 'blocked'],
  },
  {
    page: P.PrivacyPage,
    section: 'Blocked users',
    title: 'Block a user',
    description: 'Prevent messages and invites from a user by adding their user ID.',
    synonyms: ['ignore', 'ignored users', 'blocked', 'someone', 'person', 'unblock'],
  },

  // Account
  {
    page: P.AccountPage,
    section: 'Profile',
    title: 'Profile',
    description: 'Your display name, avatar, presence and status.',
    synonyms: ['display name', 'name', 'avatar', 'picture', 'photo', 'presence', 'status'],
  },
  {
    page: P.AccountPage,
    section: 'Matrix ID',
    title: 'Matrix ID',
    description: 'Your full Matrix user ID.',
    synonyms: ['user id', 'mxid', 'username', 'handle'],
  },
  {
    page: P.AccountPage,
    section: 'Password',
    title: 'Password',
    description: 'Your homeserver manages your password on its account page.',
    synonyms: ['password', 'passphrase', 'credentials', 'security'],
  },
  {
    page: P.AccountPage,
    section: 'Password',
    title: 'Change password',
    description: 'Confirm with your current password.',
    synonyms: ['password', 'reset password', 'new password', 'credentials'],
  },
  {
    page: P.AccountPage,
    section: 'Contact',
    title: 'Contact',
    synonyms: ['email', 'contact', 'threepid', 'contact information'],
  },
  {
    page: P.AccountPage,
    section: 'Contact',
    title: 'Email',
    description: 'Email addresses attached to your account.',
    synonyms: ['email', 'mail', 'contact', 'address'],
  },

  // Security & recovery
  {
    page: P.SecurityPage,
    section: 'Verification',
    title: 'Verification',
    synonyms: ['encryption', 'verify', 'keys', 'cross-signing', 'security'],
  },
  {
    page: P.SecurityPage,
    section: 'Verification',
    title: 'Verify this session',
    description:
      'Verify this session so other devices trust it and it can read your encrypted history.',
    synonyms: ['verify', 'cross-signing', 'recovery key', 'trust', 'device verification'],
  },
  {
    page: P.SecurityPage,
    section: 'Verification',
    title: 'Message history for new sessions',
    description:
      "New sessions read older encrypted messages from your key backup. Sessions don't hand room keys to each other, because a forwarded key can't prove who sent the message. Keep key backup on and verify new logins.",
    synonyms: ['key backup', 'backup', 'history', 'recovery'],
    when: 'nativeSession',
  },
  {
    page: P.SecurityPage,
    section: 'Recovery',
    title: 'Recovery',
    description: 'Key backup and a recovery key let a new session read your history.',
    synonyms: ['key backup', 'recovery key', 'secret storage', 'restore'],
    when: 'nativeSession',
  },
  {
    page: P.SecurityPage,
    section: 'Recovery',
    title: 'Secure Backup & Recovery',
    description: 'Set up or unlock secret storage to recover verification and encryption backup.',
    synonyms: ['secret storage', 'recovery key', 'ssss', 'unlock'],
    when: 'nativeSession',
  },
  {
    page: P.SecurityPage,
    section: 'Recovery',
    title: 'Encryption Backup',
    description: 'Connect this device to restore encrypted message history.',
    synonyms: ['key backup', 'restore', 'backup', 'history'],
    when: 'nativeSession',
  },
  {
    page: P.SecurityPage,
    section: 'Certificate identities',
    title: 'Certificate identities',
    synonyms: ['x.509', 'x509', 'certificate', 'ca'],
    when: 'nativeSession',
  },
  {
    page: P.SecurityPage,
    section: 'Certificate identities',
    title: 'X.509 identity (experimental)',
    description:
      'Sit beside emoji verification. Trusts whoever the imported CA issued a Matrix ID for — including bots — via matrix:u/… or email SAN. Enabling this shares encrypted room keys with those users’ devices without an emoji prompt. A CA-re-signed identity reset is accepted without the usual pin warning; that is not the same as comparing emoji. Changes apply after a session reload.',
    synonyms: ['x509', 'certificate', 'ca'],
    when: 'nativeSession',
  },
  {
    page: P.SecurityPage,
    section: 'Certificate-verified identities',
    title: 'Certificate-verified identities',
    synonyms: ['x.509', 'x509', 'certificate', 'ca'],
    when: 'nativeSession',
  },
  {
    page: P.SecurityPage,
    section: 'Danger zone',
    title: 'Danger zone',
    description: 'Changes here cannot be undone.',
    synonyms: ['reset identity', 'reset', 'start over', 'lost recovery key'],
    when: 'nativeSession',
  },
  {
    page: P.SecurityPage,
    section: 'Danger zone',
    title: 'Reset identity',
    description:
      'If you have lost your recovery key and all your other devices, start over with a new identity.',
    synonyms: ['reset', 'start over', 'lost recovery key', 'cross-signing'],
    when: 'nativeSession',
  },

  // Sessions
  {
    page: P.DevicesPage,
    section: 'This session',
    title: 'This session',
    description: 'This device.',
    synonyms: ['this device', 'current', 'session', 'log out'],
  },
  {
    page: P.DevicesPage,
    section: 'Other sessions',
    title: 'Other sessions',
    description: 'Your other logged-in sessions.',
    synonyms: ['sessions', 'other devices', 'devices', 'sign out', 'log out', 'logout'],
  },
  {
    page: P.DevicesPage,
    section: 'Other sessions',
    title: 'Session dashboard',
    description: 'Manage your sessions on your account page.',
    synonyms: ['oidc', 'dashboard', 'devices'],
  },
  {
    page: P.DevicesPage,
    section: 'Export & import keys',
    title: 'Export & import keys',
    description:
      'A password-protected file of your room keys, for moving encrypted history by hand.',
    synonyms: ['export', 'import', 'room keys', 'backup', 'local backup'],
  },
  {
    page: P.DevicesPage,
    section: 'Export & import keys',
    title: 'Export message keys',
    description: 'Save a password-protected copy of your room keys to Downloads.',
    synonyms: ['export keys', 'room keys', 'backup'],
  },
  {
    page: P.DevicesPage,
    section: 'Export & import keys',
    title: 'Import message keys',
    description:
      'Choose an encrypted room-key file. Synara decrypts and imports it on this device.',
    synonyms: ['import keys', 'room keys', 'restore'],
  },

  // Advanced
  {
    page: P.AdvancedPage,
    section: 'Experimental',
    title: 'Experimental',
    description: 'Features still being tested. They can change or go away.',
    synonyms: ['beta', 'labs', 'preview features'],
  },
  {
    page: P.AdvancedPage,
    section: 'Experimental',
    title: 'Indexed message search',
    description:
      'Local experimental index of messages this device has already seen, including encrypted rooms. Not complete homeserver history. Changing this requires a session reload.',
    synonyms: ['search', 'index', 'find', 'encrypted', 'message search'],
  },
  {
    page: P.AdvancedPage,
    section: 'Experimental',
    title: 'Show hidden events',
    description: 'Show state and unsupported events that are normally hidden from the timeline.',
    synonyms: ['state events', 'debug', 'unsupported'],
  },
  {
    page: P.AdvancedPage,
    section: 'Experimental',
    title: 'Experimental widgets',
    description:
      'Default off. This device only — not synced to the account or other devices. Widgets run third-party or local web code as you. Room-state widgets cannot load localhost; agent URLs may use loopback.',
    synonyms: ['widgets', 'integrations', 'apps', 'beta', 'agent widget'],
    when: 'synaraDesktop',
  },
  {
    page: P.AdvancedPage,
    section: 'Experimental',
    title: 'Encrypted state events (experimental)',
    description:
      'MSC4362. Older clients will not see room name, topic, or avatar. This cannot be turned off for rooms that already opted in; this device will still decrypt those rooms.',
    synonyms: ['msc4362', 'state encryption'],
  },
  {
    page: P.AdvancedPage,
    section: 'Calls',
    title: 'Calls',
    synonyms: ['voice', 'video', 'call'],
  },
  {
    page: P.AdvancedPage,
    section: 'Calls',
    title: 'Voice and video calls',
    description: 'Voice and video calling availability.',
    synonyms: ['calls', 'voice', 'video', 'rtc', 'matrixrtc', 'element call'],
    when: 'synaraDesktop',
  },
  {
    page: P.AdvancedPage,
    section: 'Calls',
    title: 'Call transport',
    description: 'The MatrixRTC call transports your homeserver offers.',
    synonyms: ['matrixrtc', 'rtc', 'livekit', 'sfu', 'transport', 'calls'],
  },
  {
    page: P.AdvancedPage,
    section: 'Developer',
    title: 'Developer',
    description: 'Low-level account data and protocol tools. Changes here take effect immediately.',
    synonyms: ['developer tools', 'devtools', 'debug'],
  },
  {
    page: P.AdvancedPage,
    section: 'Developer',
    title: 'Enable developer tools',
    description: 'Show event sources, state editors and raw account data in rooms and settings.',
    synonyms: ['developer', 'devtools', 'debug', 'view source'],
  },
  {
    page: P.AdvancedPage,
    section: 'Developer',
    title: 'Account data',
    description: 'Data stored in your global account data.',
    synonyms: ['json', 'raw', 'debug', 'global'],
    when: 'nonNativeSession',
  },
  {
    page: P.AdvancedPage,
    section: 'Maintenance',
    title: 'Maintenance',
    synonyms: ['storage', 'cache', 'reload'],
  },
  {
    page: P.AdvancedPage,
    section: 'Maintenance',
    title: 'Reload application',
    description:
      'Reset navigation and notification caches, then reload the application. Your native message store and login are kept.',
    synonyms: ['restart', 'refresh', 'reset', 'cache', 'clear cache'],
  },

  // Diagnostics (the page itself is desktop only)
  {
    page: P.DiagnosticsPage,
    section: 'Diagnostic capture',
    title: 'Diagnostic capture',
    synonyms: ['logs', 'debug', 'report'],
  },
  {
    page: P.DiagnosticsPage,
    section: 'Diagnostic capture',
    title: 'Capture diagnostics',
    description:
      'Record additional privacy-filtered evidence while reproducing a problem. Normal client behavior is unchanged.',
    synonyms: ['logs', 'debug', 'capture'],
  },
  {
    page: P.DiagnosticsPage,
    section: 'Diagnostic capture',
    title: 'Performance',
    description:
      'Capture frame cadence, long tasks, rendered timeline size, and slow timeline operations.',
    synonyms: ['fps', 'lag', 'slow', 'frames'],
  },
  {
    page: P.DiagnosticsPage,
    section: 'Diagnostic capture',
    title: 'Session persistence',
    description:
      'Capture credential-store availability, bootstrap source, token-refresh outcomes, and sync lifecycle without recording credentials.',
    synonyms: ['login', 'tokens', 'keyring'],
  },
  {
    page: P.DiagnosticsPage,
    section: 'Diagnostic capture',
    title: 'Room state and positioning',
    description:
      'Capture room-open decisions, read-marker outcomes, recent-room updates, pagination, anchoring, and unexpected scroll movement.',
    synonyms: ['read markers', 'scroll', 'pagination'],
  },
  {
    page: P.DiagnosticsPage,
    section: 'Diagnostic capture',
    title: 'Performance overlay',
    description:
      'Show live frame rate, long-task, timeline-row, and memory counters over the client.',
    synonyms: ['fps', 'overlay', 'memory'],
  },
  {
    page: P.DiagnosticsPage,
    section: 'Stored report',
    title: 'Stored report',
    synonyms: ['report', 'logs', 'copy', 'share', 'support'],
  },
  {
    page: P.DiagnosticsPage,
    section: 'Stored report',
    title: 'Local diagnostics',
    description: 'The diagnostics report stored on this device.',
    synonyms: ['report', 'logs', 'clear'],
  },
  {
    page: P.DiagnosticsPage,
    section: 'Desktop integration',
    title: 'Desktop integration',
    description: 'Readiness of the tray, shortcuts and portals for this session.',
    synonyms: ['tray', 'portal', 'system tray', 'wayland', 'x11'],
    when: 'desktop',
  },
  {
    page: P.DiagnosticsPage,
    section: 'Desktop integration',
    title: 'Native session store',
    description: 'Where your session credentials are kept on this device.',
    synonyms: ['keyring', 'keychain', 'secret store', 'credentials'],
    when: 'desktop',
  },
  {
    page: P.DiagnosticsPage,
    section: 'Privacy',
    title: 'Privacy',
    description: 'What diagnostic reports leave out.',
    synonyms: ['report', 'exclude', 'tokens'],
  },

  // About
  {
    page: P.AboutPage,
    section: 'Credits',
    title: 'Credits',
    synonyms: ['licenses', 'open source', 'thanks', 'attribution'],
  },
];

/** The index entries whose rows render where `holds` says their condition is met. */
export const appSettingsSearchIndex = (
  holds: (condition: SettingsIndexCondition) => boolean
): AppSettingsSearchEntry[] =>
  APP_SETTINGS_SEARCH_INDEX.filter((entry) => entry.when === undefined || holds(entry.when));
