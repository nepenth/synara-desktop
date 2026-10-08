// The production app shell (router, ClientRoot, Settings, room header) with
// only native IPC answered by fixtures, so settings layout and menus render
// exactly as they ship.
import React from 'react';
import { createRoot } from 'react-dom/client';
import { enableMapSet } from 'immer';
import { configClass, varsClass } from 'folds';
import 'folds/dist/style.css';
import '@fontsource-variable/inter';
import '../../src/index.css';
import { darkTheme, synaraLightTheme } from '../../src/colors.css';
import App from '../../src/app/pages/App';
import '../../src/app/i18n';
import { setSessionBootstrapResult } from '../../src/app/state/sessionBootstrap';
import type { RoomSummary } from '../../src/app/features/matrix-dto/room';
import { presentationFor } from '../room-list-harness/presentation';

enableMapSet();

const query = new URLSearchParams(location.search);
const ROOM_ID = '!review:example.test';
const identity = {
  userId: '@reviewer:example.test',
  deviceId: 'REVIEW',
  homeserverUrl: `${location.origin}/mock-homeserver`,
  sessionGeneration: 1,
};
const room: RoomSummary = {
  roomId: ROOM_ID,
  name: 'Design review',
  canonicalAlias: '#design:example.test',
  membership: 'join',
  isDirect: false,
  isSpace: false,
  isEncrypted: true,
  encryptionStatus: 'encrypted',
  joinRule: 'invite',
  unreadCount: 0,
  highlightCount: 0,
  markedUnread: false,
  lastMessageIsAgentApproval: false,
  lastActivityTs: Date.now() - 60000,
};
const roomState = (eventType: string, content: unknown) => ({
  status: 'ok',
  roomId: ROOM_ID,
  eventType,
  stateKey: '',
  sessionGeneration: 1,
  content,
});
type Fixture = unknown | ((args?: Record<string, unknown>) => unknown);
const fixtures: Record<string, Fixture> = {
  matrix_room_power_levels_snapshot: roomState('m.room.power_levels', {
    users_default: 0,
    users: { '@reviewer:example.test': 100 },
    events_default: 0,
    state_default: 50,
    invite: 0,
  }),
  matrix_room_power_level_tags_snapshot: roomState('in.synara.room.power_level_tags', {}),
  matrix_room_creators_snapshot: {
    status: 'ok',
    roomId: ROOM_ID,
    eventType: 'm.room.create',
    stateKey: '',
    sessionGeneration: 1,
    creators: ['@reviewer:example.test'],
  },
  matrix_restore_session: identity,
  // Core's wire shape: snake_case identity plus camelCase sessionGeneration.
  matrix_session_snapshot: {
    status: 'logged_in',
    user_id: identity.userId,
    device_id: identity.deviceId,
    homeserver_url: identity.homeserverUrl,
    sessionGeneration: identity.sessionGeneration,
  },
  matrix_sync_status: {
    readiness: 'running',
    sessionGeneration: 1,
    offlineModeEnabled: true,
    failureDiagnosticId: null,
    slidingSyncCapable: true,
    commandGate: 'open',
  },
  matrix_room_list_snapshot: {
    sessionGeneration: 1,
    orderedRoomIds: [ROOM_ID],
    rooms: [room],
    presentation: presentationFor([room]),
  },
  desktop_append_log: true,
  desktop_set_badge_count: true,
  desktop_set_shortcuts: true,
  desktop_dismiss_notifications: true,
  matrix_notification_focus_set: true,
  'plugin:window|is_maximized': false,
  matrix_get_own_profile: { displayName: 'Reviewer', avatarUrl: null },
};
const unknown = new Set<string>();
let callbackId = 0;
Object.assign(window, {
  __SYNARA_DESKTOP__: { platform: 'tauri', os: 'linux' },
  __TAURI_INTERNALS__: {
    metadata: {
      currentWindow: { label: 'main' },
      currentWebview: { label: 'main', windowLabel: 'main' },
    },
    transformCallback: () => {
      callbackId += 1;
      return callbackId;
    },
    invoke: async (command: string, args?: Record<string, unknown>) => {
      if (command in fixtures) {
        const fixture = fixtures[command];
        return typeof fixture === 'function' ? fixture(args) : fixture;
      }
      if (command.startsWith('plugin:event|')) return callbackId;
      unknown.add(command);
      throw new Error(`fixture: ${command} unavailable`);
    },
  },
  __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => undefined },
  synaraUnknownCommands: unknown,
});
setSessionBootstrapResult({
  source: 'native',
  session: {
    userId: identity.userId,
    deviceId: identity.deviceId,
    baseUrl: identity.homeserverUrl,
    sessionGeneration: String(identity.sessionGeneration),
  },
});
const theme = query.get('theme') === 'light' ? [synaraLightTheme] : [darkTheme, 'dark-theme'];
document.body.classList.add(configClass, varsClass, ...theme);
createRoot(document.getElementById('root')!).render(<App />);
