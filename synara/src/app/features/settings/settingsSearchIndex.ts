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
    title: 'Date & Time',
    synonyms: ['clock', 'format'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: '24-Hour Time Format',
    description: 'Show times on a 24-hour clock.',
    synonyms: ['clock', 'am', 'pm', 'military'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Date Format',
    description: 'How dates appear in the timeline.',
    synonyms: ['day', 'month', 'year'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Editor',
    synonyms: ['composer', 'typing', 'compose'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'ENTER for Newline',
    description: 'Use Enter for a new line and Ctrl+Enter to send.',
    synonyms: ['send', 'keyboard', 'return', 'shift'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Markdown Formatting',
    description: 'Format messages with Markdown.',
    synonyms: ['bold', 'italic', 'code', 'formatting'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Hide Typing & Read Receipts',
    description:
      'Hide automatic typing status and read receipts. Choosing Mark as Read still sends a receipt.',
    synonyms: ['privacy', 'typing indicator', 'read receipts', 'seen'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Messages',
    synonyms: ['timeline', 'events'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Hide Membership Change',
    description: 'Do not show join, leave, and invite events in the timeline.',
    synonyms: ['joins', 'leaves', 'invites', 'membership'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Hide Profile Change',
    description: 'Do not show display name and avatar changes in the timeline.',
    synonyms: ['display name', 'avatar', 'profile'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Load Media Automatically',
    description: 'Download and show images and video previews as messages arrive.',
    synonyms: ['images', 'pictures', 'video', 'previews', 'autoload', 'data'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'GIF Search',
    description: 'Search for GIFs from the composer.',
    synonyms: ['gif', 'giphy', 'tenor', 'animated'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Indexed message search',
    description: 'Search encrypted messages with a local index.',
    synonyms: ['search', 'index', 'find', 'encrypted'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Show Hidden Events',
    description: 'Show state and unsupported events that are normally hidden from the timeline.',
    synonyms: ['state events', 'debug', 'unsupported'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Calls',
    synonyms: ['voice', 'video', 'call'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'MatrixRTC',
    description: 'Voice and video calling availability.',
    synonyms: ['calls', 'voice', 'video', 'rtc', 'element call'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Widgets',
    synonyms: ['integrations', 'apps'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Experimental Widgets',
    description: 'Allow room widgets.',
    synonyms: ['widgets', 'integrations', 'apps'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Software Updates',
    synonyms: ['update', 'upgrade', 'version', 'release'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Storage',
    synonyms: ['cache', 'disk', 'data'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Native Session Store',
    description: 'Where your session credentials are kept on this device.',
    synonyms: ['keyring', 'keychain', 'secret store', 'credentials'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Reload Application',
    description: 'Reset navigation and notification caches, then reload the application.',
    synonyms: ['restart', 'refresh', 'reset', 'cache'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Desktop Integration',
    description: 'Current session diagnostics for tray, shortcuts, and portal readiness.',
    synonyms: ['tray', 'portal', 'system tray'],
  },
  {
    page: SettingsPages.GeneralPage,
    title: 'Desktop Shortcuts',
    description: 'Global keyboard shortcuts for the desktop app.',
    synonyms: ['keyboard', 'hotkeys', 'shortcuts', 'keybindings'],
  },

  // Appearance
  {
    page: SettingsPages.AppearancePage,
    title: 'Theme',
    description: 'Theme to use when system theme is not enabled.',
    synonyms: ['dark', 'light', 'mode', 'colors', 'night'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'System Theme',
    description: 'Choose between light and dark theme based on system preference.',
    synonyms: ['dark', 'light', 'automatic', 'os'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Light Theme',
    description: 'Used while your system is in light mode.',
    synonyms: ['light mode', 'day'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Dark Theme',
    description: 'Used while your system is in dark mode.',
    synonyms: ['dark mode', 'night'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Monochrome Mode',
    description: 'Render the interface in greyscale, keeping only the accent color.',
    synonyms: ['greyscale', 'grayscale', 'black and white'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Base Color',
    description: 'Tint the interface surfaces.',
    synonyms: ['background', 'tint', 'colour'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Accent Color',
    description: 'Color for buttons, links and highlights.',
    synonyms: ['highlight', 'primary', 'colour'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Thread Display',
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
    title: 'Message Text',
    description:
      'Adjust message prose brightness without changing navigation, metadata, or code colors.',
    synonyms: ['brightness', 'contrast', 'tone', 'readability'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Page Zoom',
    description: 'Scale the whole interface, from 75% to 150%.',
    synonyms: ['zoom', 'scale', 'size', 'bigger', 'smaller', 'font size'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Messages',
    synonyms: ['timeline', 'chat'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Message Layout',
    description: 'Modern, compact or bubble message layout.',
    synonyms: ['layout', 'bubbles', 'compact', 'irc', 'density'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Message Spacing',
    description: 'Space between messages in the timeline.',
    synonyms: ['spacing', 'density', 'padding', 'compact'],
  },
  {
    page: SettingsPages.AppearancePage,
    title: 'Legacy Username Color',
    description: 'Color sender names with the legacy per-user palette instead of the theme accent.',
    synonyms: ['username', 'name color', 'sender', 'colour'],
  },

  // Notifications
  {
    page: SettingsPages.NotificationPage,
    title: 'System',
    synonyms: ['desktop', 'os', 'alerts'],
  },
  {
    page: SettingsPages.NotificationPage,
    title: 'Desktop Notifications',
    description: 'Show system notifications for new messages.',
    synonyms: ['alerts', 'popups', 'banners', 'notify'],
  },
  {
    page: SettingsPages.NotificationPage,
    title: 'Notification Permission',
    description: 'Allow Synara to show system notifications.',
    synonyms: ['permission', 'allow', 'critical alerts'],
  },
  {
    page: SettingsPages.NotificationPage,
    title: 'Notification Sound',
    description: 'Play sound when new messages arrive.',
    synonyms: ['sound', 'audio', 'ping', 'chime', 'mute'],
  },
  {
    page: SettingsPages.NotificationPage,
    title: 'All Messages',
    synonyms: ['rooms', 'direct messages', 'dm'],
  },
  {
    page: SettingsPages.NotificationPage,
    title: '1-to-1 Chats',
    description: 'Notifications for direct messages.',
    synonyms: ['dm', 'direct messages', 'private'],
  },
  {
    page: SettingsPages.NotificationPage,
    title: '1-to-1 Chats (Encrypted)',
    description: 'Notifications for encrypted direct messages.',
    synonyms: ['dm', 'direct messages', 'encrypted', 'e2ee'],
  },
  {
    page: SettingsPages.NotificationPage,
    title: 'Rooms',
    description: 'Notifications for group rooms.',
    synonyms: ['groups', 'channels'],
  },
  {
    page: SettingsPages.NotificationPage,
    title: 'Rooms (Encrypted)',
    description: 'Notifications for encrypted group rooms.',
    synonyms: ['groups', 'channels', 'encrypted', 'e2ee'],
  },
  {
    page: SettingsPages.NotificationPage,
    title: 'Mentions',
    description: 'Notify when someone mentions you or the room.',
    synonyms: ['mention', 'ping', 'at', '@room', 'display name'],
  },
  {
    page: SettingsPages.NotificationPage,
    title: 'Keyword Messages',
    synonyms: ['keywords', 'words', 'highlights'],
  },
  {
    page: SettingsPages.NotificationPage,
    title: 'Select Keyword',
    description: 'Notify when a message contains this keyword.',
    synonyms: ['keyword', 'add keyword', 'highlight'],
  },
  {
    page: SettingsPages.NotificationPage,
    title: 'Agent Notifications',
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
    title: 'Contact Information',
    synonyms: ['email', 'contact', 'threepid'],
  },
  {
    page: SettingsPages.AccountPage,
    title: 'Email Address',
    description: 'Email address attached to your account.',
    synonyms: ['email', 'mail', 'contact'],
  },
  {
    page: SettingsPages.AccountPage,
    title: 'Select User',
    description: 'Prevent receiving messages or invites from a user by adding their user ID.',
    synonyms: ['ignore', 'ignored users', 'block', 'blocked'],
  },

  // Devices
  {
    page: SettingsPages.DevicesPage,
    title: 'Security',
    synonyms: ['encryption', 'verification', 'keys'],
  },
  {
    page: SettingsPages.DevicesPage,
    title: 'Device Verification',
    description: 'To verify device identity and grant access to encrypted messages.',
    synonyms: ['verify', 'cross-signing', 'recovery key', 'trust'],
  },
  {
    page: SettingsPages.DevicesPage,
    title: 'Message history for new sessions',
    description: 'New sessions read older encrypted messages from your key backup.',
    synonyms: ['key backup', 'backup', 'history', 'recovery'],
  },
  {
    page: SettingsPages.DevicesPage,
    title: 'Current',
    description: 'This device.',
    synonyms: ['this device', 'session'],
  },
  {
    page: SettingsPages.DevicesPage,
    title: 'Others',
    description: 'Your other signed-in devices.',
    synonyms: ['sessions', 'other devices', 'sign out', 'logout'],
  },
  {
    page: SettingsPages.DevicesPage,
    title: 'Device Dashboard',
    description: 'Manage your devices on the account dashboard.',
    synonyms: ['oidc', 'dashboard', 'sessions'],
  },
  {
    page: SettingsPages.DevicesPage,
    title: 'Local Backup',
    synonyms: ['export', 'import', 'room keys', 'backup'],
  },
  {
    page: SettingsPages.DevicesPage,
    title: 'Export Messages Data',
    description: 'Save a password-protected copy of room keys directly to Downloads.',
    synonyms: ['export keys', 'room keys', 'backup'],
  },
  {
    page: SettingsPages.DevicesPage,
    title: 'Import Messages Data',
    description: 'Choose an encrypted room-key file to import.',
    synonyms: ['import keys', 'room keys', 'restore'],
  },
  {
    page: SettingsPages.DevicesPage,
    title: 'Certificate-verified identities',
    synonyms: ['x.509', 'x509', 'certificate', 'ca'],
  },
  {
    page: SettingsPages.DevicesPage,
    title: 'X.509 identity (experimental)',
    description: 'Trust Matrix IDs issued by an imported certificate authority.',
    synonyms: ['x509', 'certificate', 'ca'],
  },

  // Diagnostics
  {
    page: SettingsPages.DiagnosticsPage,
    title: 'Diagnostic Capture',
    synonyms: ['logs', 'debug', 'report'],
  },
  {
    page: SettingsPages.DiagnosticsPage,
    title: 'Enable Diagnostic Capture',
    description: 'Record additional privacy-filtered evidence while reproducing a problem.',
    synonyms: ['logs', 'debug', 'capture'],
  },
  {
    page: SettingsPages.DiagnosticsPage,
    title: 'Performance',
    description: 'Capture frame cadence, long tasks and slow timeline operations.',
    synonyms: ['fps', 'lag', 'slow', 'frames'],
  },
  {
    page: SettingsPages.DiagnosticsPage,
    title: 'Session Persistence',
    description: 'Capture credential-store availability and token-refresh outcomes.',
    synonyms: ['login', 'tokens', 'keyring'],
  },
  {
    page: SettingsPages.DiagnosticsPage,
    title: 'Room State and Positioning',
    description: 'Capture room-open decisions, read-marker outcomes and pagination.',
    synonyms: ['read markers', 'scroll', 'pagination'],
  },
  {
    page: SettingsPages.DiagnosticsPage,
    title: 'Performance Overlay',
    description: 'Show live frame rate, long-task, timeline-row, and memory counters.',
    synonyms: ['fps', 'overlay', 'memory'],
  },
  {
    page: SettingsPages.DiagnosticsPage,
    title: 'Stored Report',
    synonyms: ['report', 'logs', 'copy', 'share', 'support'],
  },
  {
    page: SettingsPages.DiagnosticsPage,
    title: 'Local diagnostics',
    description: 'The diagnostics report stored on this device.',
    synonyms: ['report', 'logs'],
  },

  // Developer Tools
  {
    page: SettingsPages.DeveloperToolsPage,
    title: 'Options',
    synonyms: ['developer', 'devtools'],
  },
  {
    page: SettingsPages.DeveloperToolsPage,
    title: 'Enable Developer Tools',
    description: 'Show developer tools in rooms and settings.',
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
    title: 'Native Session Store',
    description: 'Inspect where session credentials are stored.',
    synonyms: ['keyring', 'keychain', 'secret store'],
  },
  {
    page: SettingsPages.DeveloperToolsPage,
    title: 'Experimental Widgets',
    description: 'Widgets are configured under General.',
    synonyms: ['widgets'],
  },
  {
    page: SettingsPages.DeveloperToolsPage,
    title: 'Account Data',
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
