import type { SettingsSearchEntry } from '../../components/settings-layout/settingsSearch';
import { SettingsPages } from './settingsPages';

/**
 * Every row and section the App Settings pages render, for settings search.
 *
 * Titles must match the rendered titles exactly; the search result scrolls to
 * the row with that title. `settingsSearchIndex.test.ts` scans the pages and
 * fails when a rendered title is missing here.
 */
export const APP_SETTINGS_SEARCH_INDEX: SettingsSearchEntry<SettingsPages>[] = [
  // General
  {
    page: SettingsPages.GeneralPage,
    title: 'Updates',
    description: 'Check for a new version of Synara.',
    synonyms: ['update', 'upgrade', 'version', 'release', 'software updates'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Date & time',
    synonyms: ['clock', 'format'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: '24-hour time',
    description: 'Show times on a 24-hour clock.',
    synonyms: ['clock', 'am', 'pm', 'military', 'time format'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Date format',
    description: 'How dates appear in the timeline.',
    synonyms: ['day', 'month', 'year'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Composer',
    synonyms: ['editor', 'typing', 'compose', 'message box'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Enter adds a new line',
    description: 'Enter starts a new line and Ctrl+Enter sends the message.',
    synonyms: ['send', 'keyboard', 'return', 'shift', 'newline'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Markdown formatting',
    description: 'Format messages with Markdown.',
    synonyms: ['bold', 'italic', 'code', 'formatting'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Hide typing & read receipts',
    description:
      'Hide automatic typing status and read receipts. Choosing Mark as read still sends a receipt.',
    synonyms: ['privacy', 'typing indicator', 'read receipts', 'seen'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Messages',
    synonyms: ['timeline', 'events'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Hide membership changes',
    description: 'Do not show join, leave, and invite events in the timeline.',
    synonyms: ['joins', 'leaves', 'invites', 'membership'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Hide profile changes',
    description: 'Do not show display name and avatar changes in the timeline.',
    synonyms: ['display name', 'avatar', 'profile'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Load media automatically',
    description: 'Download and show images and video previews as messages arrive.',
    synonyms: ['images', 'pictures', 'video', 'previews', 'autoload', 'data'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'GIF search',
    description: 'Search for GIFs from the composer.',
    synonyms: ['gif', 'giphy', 'klipy', 'animated'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Play GIF links inline',
    description: 'Show Klipy and GIPHY links as GIFs.',
    synonyms: ['gif', 'klipy', 'giphy', 'animated', 'embed'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Show X posts',
    description: 'Show X posts and play their video from x.com links.',
    synonyms: ['x', 'twitter', 'tweet', 'video', 'embed'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Link previews',
    description: 'Show the title and image of a linked page, fetched from this device.',
    synonyms: ['link', 'url', 'preview', 'unfurl', 'embed', 'open graph'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Sync drafts across devices',
    description: 'Keep unsent messages on your other Synara devices, end-to-end encrypted.',
    synonyms: ['draft', 'unsent', 'composer', 'sync', 'devices', 'continue'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Shorten long links',
    description: 'Show a long web address as its site and path.',
    synonyms: ['link', 'url', 'shorten', 'truncate', 'tracking', 'long'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Indexed message search',
    description: 'Search encrypted messages with a local index.',
    synonyms: ['search', 'index', 'find', 'encrypted'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Show hidden events',
    description: 'Show state and unsupported events that are normally hidden from the timeline.',
    synonyms: ['state events', 'debug', 'unsupported'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Keyboard shortcuts',
    description: 'Global keyboard shortcuts for the desktop app.',
    synonyms: ['keyboard', 'hotkeys', 'shortcuts', 'keybindings', 'desktop shortcuts'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Show or hide the room list',
    description: 'Cmd/Ctrl+Shift+D hides the room list pane or brings it back.',
    synonyms: ['sidebar', 'room list', 'collapse', 'hide', 'navigation pane', 'shortcut'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Vim-style list keys',
    description: 'j and k move through the room list, g g goes to the top, G to the bottom.',
    synonyms: ['vim', 'hjkl', 'keyboard', 'room list', 'tiling', 'navigation keys'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Storage',
    synonyms: ['cache', 'disk', 'data'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Reload application',
    description: 'Reset navigation and notification caches, then reload the application.',
    synonyms: ['restart', 'refresh', 'reset', 'cache'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Calls',
    synonyms: ['voice', 'video', 'call'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Voice and video calls',
    description: 'Voice and video calling availability.',
    synonyms: ['calls', 'voice', 'video', 'rtc', 'matrixrtc', 'element call'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Widgets',
    synonyms: ['integrations', 'apps', 'beta'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Experimental widgets',
    description: 'Allow room widgets.',
    synonyms: ['widgets', 'integrations', 'apps'],
  },

  // Appearance
  {
    page: SettingsPages.AppearancePage,
    title: 'Theme',
    description: 'Which light or dark theme to use.',
    synonyms: ['dark', 'light', 'mode', 'colors', 'night', 'system theme'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Appearance',
    description: 'Follow the system setting, or keep Synara light or dark.',
    synonyms: ['dark', 'light', 'automatic', 'os', 'system theme'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Light theme',
    description: 'Used while your system is in light mode.',
    synonyms: ['light mode', 'day'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Dark theme',
    description: 'Used while your system is in dark mode.',
    synonyms: ['dark mode', 'night'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Follow system theme colors',
    description: 'Use the background, text and accent colors of your desktop theme, kept readable.',
    synonyms: [
      'omarchy',
      'desktop theme',
      'system colors',
      'palette',
      'wallpaper',
      'colour scheme',
      'color scheme',
    ],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Monochrome mode',
    description: 'Render the interface in greyscale, keeping only the accent color.',
    synonyms: ['greyscale', 'grayscale', 'black and white'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Base color',
    description: 'Tints the rail, room list and chat.',
    synonyms: ['background', 'tint', 'colour'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Accent color',
    description: 'Color for buttons, links and highlights.',
    synonyms: ['highlight', 'primary', 'colour'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Thread display',
    description:
      'Open threads inline beneath their first message, in a side panel beside the room, or in place of the room.',
    synonyms: ['thread', 'threads', 'replies', 'inline', 'side panel', 'full view', 'pane'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Text',
    synonyms: ['font', 'typography', 'size'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Message text',
    description:
      'Adjust message prose brightness without changing navigation, metadata, or code colors.',
    synonyms: ['brightness', 'contrast', 'tone', 'readability'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Interface font',
    description: 'The font for menus, lists and messages: default, monospace or a custom family.',
    synonyms: ['font', 'typeface', 'typography', 'family', 'ui font', 'sans'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Monospace font',
    description: 'The font for code and other fixed-width text.',
    synonyms: ['font', 'code font', 'fixed width', 'mono', 'typeface', 'programming'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Page zoom',
    description: 'Scale the whole interface, from 75% to 200%.',
    synonyms: ['zoom', 'scale', 'size', 'bigger', 'smaller', 'font size'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Messages',
    synonyms: ['timeline', 'chat'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Message layout',
    description: 'Modern, compact or bubble message layout.',
    synonyms: ['layout', 'bubbles', 'compact', 'irc', 'density'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Message spacing',
    description: 'Space between messages in the timeline.',
    synonyms: ['spacing', 'density', 'padding', 'compact'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Legacy username color',
    description: 'Color sender names with the legacy per-user palette instead of the theme accent.',
    synonyms: ['username', 'name color', 'sender', 'colour'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Window',
    synonyms: ['title bar', 'window manager', 'linux'],
  },
  {
    page: SettingsPages.AppearancePage,
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
  },

  // Notifications
  {
    page: SettingsPages.NotificationPage,
    title: 'System',
    synonyms: ['desktop', 'os', 'alerts'],
  },
  {
    page: SettingsPages.NotificationPage,
    title: 'Desktop notifications',
    description: 'Show system notifications for new messages. Approval requests always alert.',
    synonyms: ['alerts', 'popups', 'banners', 'notify'],
  },
  {
    page: SettingsPages.NotificationPage,
    title: 'Notification permission',
    description: 'Allow Synara to show system notifications.',
    synonyms: ['permission', 'allow', 'critical alerts'],
  },
  {
    page: SettingsPages.NotificationPage,
    title: 'Notification sound',
    description: 'Play a sound when a message arrives.',
    synonyms: ['sound', 'audio', 'ping', 'chime', 'mute'],
  },
  {
    page: SettingsPages.NotificationPage,
    title: 'Show approval details',
    description: 'Show the reason and requested command in agent approval notifications.',
    synonyms: ['approval', 'agent', 'hermes', 'command', 'privacy', 'preview'],
  },
  {
    page: SettingsPages.NotificationPage,
    title: 'Direct messages',
    description: 'Notifications for one-to-one conversations, encrypted or not.',
    synonyms: ['dm', '1-to-1', 'private', 'all messages'],
  },
  {
    page: SettingsPages.NotificationPage,
    title: 'Rooms',
    description: 'Notifications for group rooms, encrypted or not.',
    synonyms: ['groups', 'channels', 'encrypted', 'e2ee', 'all messages'],
  },
  {
    page: SettingsPages.NotificationPage,
    title: 'Mentions',
    description: 'Notify when someone mentions you or the room.',
    synonyms: ['mention', 'ping', 'at', '@room', 'display name'],
  },
  {
    page: SettingsPages.NotificationPage,
    title: 'Keywords',
    description: 'Notify when a message contains one of these words.',
    synonyms: ['keyword', 'words', 'highlights', 'add keyword'],
  },
  {
    page: SettingsPages.NotificationPage,
    title: 'Agent notifications',
    synonyms: ['agents', 'bots', 'approvals', 'tool activity', 'commentary'],
  },
  {
    page: SettingsPages.NotificationPage,
    title: 'Agent accounts',
    description: 'The agent Matrix user IDs these notification rules apply to.',
    synonyms: ['agents', 'bots'],
  },

  // Account
  {
    page: SettingsPages.AccountPage,
    title: 'Profile',
    description: 'Your display name and avatar.',
    synonyms: ['display name', 'name', 'avatar', 'picture', 'photo'],
  },
  {
    page: SettingsPages.AccountPage,
    title: 'Matrix ID',
    description: 'Your full Matrix user ID.',
    synonyms: ['user id', 'mxid', 'username', 'handle'],
  },
  {
    page: SettingsPages.AccountPage,
    title: 'Password',
    synonyms: ['password', 'passphrase', 'credentials', 'security'],
  },
  {
    page: SettingsPages.AccountPage,
    title: 'Change password',
    description: 'Confirm with your current password.',
    synonyms: ['password', 'reset password', 'new password', 'credentials'],
  },
  {
    page: SettingsPages.AccountPage,
    title: 'Contact',
    synonyms: ['email', 'contact', 'threepid', 'contact information'],
  },
  {
    page: SettingsPages.AccountPage,
    title: 'Email',
    description: 'Email addresses attached to your account.',
    synonyms: ['email', 'mail', 'contact', 'address'],
  },
  {
    page: SettingsPages.AccountPage,
    title: 'Blocked users',
    description: "Blocked users can't message or invite you.",
    synonyms: ['ignore', 'ignored users', 'block', 'blocked'],
  },
  {
    page: SettingsPages.AccountPage,
    title: 'Block a user',
    description: 'Prevent messages and invites from a user by adding their user ID.',
    synonyms: ['ignore', 'ignored users', 'block', 'blocked'],
  },

  // Security & recovery
  {
    page: SettingsPages.SecurityPage,
    title: 'Verification',
    synonyms: ['encryption', 'verify', 'keys', 'cross-signing', 'security'],
  },
  {
    page: SettingsPages.SecurityPage,
    title: 'Verify this session',
    description:
      'Verify this session so other devices trust it and it can read your encrypted history.',
    synonyms: ['verify', 'cross-signing', 'recovery key', 'trust', 'device verification'],
  },
  {
    page: SettingsPages.SecurityPage,
    title: 'Message history for new sessions',
    description: 'New sessions read older encrypted messages from your key backup.',
    synonyms: ['key backup', 'backup', 'history', 'recovery'],
  },
  {
    page: SettingsPages.SecurityPage,
    title: 'Recovery',
    description: 'Key backup and a recovery key let a new session read your history.',
    synonyms: ['key backup', 'recovery key', 'secret storage', 'restore'],
  },
  {
    page: SettingsPages.SecurityPage,
    title: 'Certificate identities',
    synonyms: ['x.509', 'x509', 'certificate', 'ca'],
  },
  {
    page: SettingsPages.SecurityPage,
    title: 'X.509 identity (experimental)',
    description: 'Trust Matrix IDs issued by an imported certificate authority.',
    synonyms: ['x509', 'certificate', 'ca'],
  },
  {
    page: SettingsPages.SecurityPage,
    title: 'Certificate-verified identities',
    synonyms: ['x.509', 'x509', 'certificate', 'ca'],
  },
  {
    page: SettingsPages.SecurityPage,
    title: 'Danger zone',
    description: 'Reset your cryptographic identity if you have lost every other way back in.',
    synonyms: ['reset identity', 'reset', 'start over', 'lost recovery key'],
  },

  // Sessions
  {
    page: SettingsPages.DevicesPage,
    title: 'This session',
    description: 'This device.',
    synonyms: ['this device', 'current', 'session', 'log out'],
  },
  {
    page: SettingsPages.DevicesPage,
    title: 'Other sessions',
    description: 'Your other logged-in sessions.',
    synonyms: ['sessions', 'other devices', 'devices', 'sign out', 'log out', 'logout'],
  },
  {
    page: SettingsPages.DevicesPage,
    title: 'Session dashboard',
    description: 'Manage your sessions on your account page.',
    synonyms: ['oidc', 'dashboard', 'devices'],
  },
  {
    page: SettingsPages.DevicesPage,
    title: 'Export & import keys',
    synonyms: ['export', 'import', 'room keys', 'backup', 'local backup'],
  },
  {
    page: SettingsPages.DevicesPage,
    title: 'Export message keys',
    description: 'Save a password-protected copy of your room keys to Downloads.',
    synonyms: ['export keys', 'room keys', 'backup'],
  },
  {
    page: SettingsPages.DevicesPage,
    title: 'Import message keys',
    description: 'Choose an encrypted room-key file to import.',
    synonyms: ['import keys', 'room keys', 'restore'],
  },

  // Diagnostics
  {
    page: SettingsPages.DiagnosticsPage,
    title: 'Diagnostic capture',
    synonyms: ['logs', 'debug', 'report'],
  },
  {
    page: SettingsPages.DiagnosticsPage,
    title: 'Capture diagnostics',
    description: 'Record additional privacy-filtered evidence while reproducing a problem.',
    synonyms: ['logs', 'debug', 'capture', 'enable'],
  },
  {
    page: SettingsPages.DiagnosticsPage,
    title: 'Performance',
    description: 'Capture frame cadence, long tasks and slow timeline operations.',
    synonyms: ['fps', 'lag', 'slow', 'frames'],
  },
  {
    page: SettingsPages.DiagnosticsPage,
    title: 'Session persistence',
    description: 'Capture credential-store availability and token-refresh outcomes.',
    synonyms: ['login', 'tokens', 'keyring'],
  },
  {
    page: SettingsPages.DiagnosticsPage,
    title: 'Room state and positioning',
    description: 'Capture room-open decisions, read-marker outcomes and pagination.',
    synonyms: ['read markers', 'scroll', 'pagination'],
  },
  {
    page: SettingsPages.DiagnosticsPage,
    title: 'Performance overlay',
    description: 'Show live frame rate, long-task, timeline-row, and memory counters.',
    synonyms: ['fps', 'overlay', 'memory'],
  },
  {
    page: SettingsPages.DiagnosticsPage,
    title: 'Stored report',
    synonyms: ['report', 'logs', 'copy', 'share', 'support'],
  },
  {
    page: SettingsPages.DiagnosticsPage,
    title: 'Local diagnostics',
    description: 'The diagnostics report stored on this device.',
    synonyms: ['report', 'logs', 'clear'],
  },
  {
    page: SettingsPages.DiagnosticsPage,
    title: 'Desktop integration',
    description: 'Readiness of the tray, shortcuts and portals for this session.',
    synonyms: ['tray', 'portal', 'system tray', 'wayland', 'x11'],
  },
  {
    page: SettingsPages.DiagnosticsPage,
    title: 'Native session store',
    description: 'Where your session credentials are kept on this device.',
    synonyms: ['keyring', 'keychain', 'secret store', 'credentials'],
  },
  {
    page: SettingsPages.DiagnosticsPage,
    title: 'Privacy',
    description: 'What diagnostic reports leave out.',
    synonyms: ['report', 'exclude', 'tokens'],
  },

  // Developer tools
  {
    page: SettingsPages.DeveloperToolsPage,
    title: 'Options',
    synonyms: ['developer', 'devtools'],
  },
  {
    page: SettingsPages.DeveloperToolsPage,
    title: 'Enable developer tools',
    description: 'Show event sources, state editors and raw account data in rooms and settings.',
    synonyms: ['developer', 'devtools', 'debug'],
  },
  {
    page: SettingsPages.DeveloperToolsPage,
    title: 'Encrypted state events (experimental)',
    description: 'MSC4362. Older clients will not see room name, topic, or avatar.',
    synonyms: ['msc4362', 'state encryption'],
  },
  {
    page: SettingsPages.DeveloperToolsPage,
    title: 'Account data',
    description: 'Browse and edit raw account data.',
    synonyms: ['json', 'raw', 'debug'],
  },
  {
    page: SettingsPages.DeveloperToolsPage,
    title: 'Global',
    description: 'Data stored in your global account data.',
    synonyms: ['account data', 'json'],
  },

  // About
  {
    page: SettingsPages.AboutPage,
    title: 'Credits',
    synonyms: ['licenses', 'open source', 'thanks', 'attribution'],
  },
];
