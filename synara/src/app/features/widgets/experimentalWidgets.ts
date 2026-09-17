import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import type { AgentWidgetEntry } from '../../state/settings';

export { isSafeWidgetUrl } from './widgetUrl';

export type ListedWidget = {
  widgetId: string;
  name: string;
  origin: string;
  url: string;
  kind: 'roomState' | 'agent';
  initOnContentLoad: boolean;
};

export type WidgetListSnapshot = {
  sessionGeneration: number;
  roomId: string;
  widgets: ListedWidget[];
  openSessions: Array<{
    sessionId: string;
    widgetId: string;
    roomId: string;
    name: string;
    origin: string;
    kind: 'roomState' | 'agent';
  }>;
};

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

const isListedWidget = (value: unknown): value is ListedWidget => {
  if (!isRecord(value)) return false;
  return (
    typeof value.widgetId === 'string' &&
    typeof value.name === 'string' &&
    typeof value.origin === 'string' &&
    typeof value.url === 'string' &&
    (value.kind === 'roomState' || value.kind === 'agent') &&
    typeof value.initOnContentLoad === 'boolean'
  );
};

const parseListSnapshot = (value: unknown): WidgetListSnapshot | undefined => {
  if (!isRecord(value) || !Array.isArray(value.widgets)) return undefined;
  if (typeof value.sessionGeneration !== 'number' || typeof value.roomId !== 'string') {
    return undefined;
  }
  const widgets = value.widgets.filter(isListedWidget);
  return {
    sessionGeneration: value.sessionGeneration,
    roomId: value.roomId,
    widgets,
    openSessions: [],
  };
};

export const listExperimentalWidgets = async (
  roomId: string,
  agentWidgets: AgentWidgetEntry[],
  experimentalWidgetsEnabled: boolean
): Promise<WidgetListSnapshot | undefined> => {
  if (!isSynaraDesktop() || !experimentalWidgetsEnabled) return undefined;
  try {
    const result = await invokeDesktopWithAvailability<WidgetListSnapshot>(
      'matrix_widgets_list',
      {
        experimentalWidgetsEnabled,
        roomId,
        agentWidgets,
      },
      { suppressErrorDiagnostic: true }
    );
    if (!result.available) return undefined;
    return parseListSnapshot(result.value);
  } catch {
    return undefined;
  }
};

export const openExperimentalWidget = async (args: {
  experimentalWidgetsEnabled: boolean;
  roomId: string;
  widget: ListedWidget;
  receiveRoom: boolean;
  sendRoomMessage: boolean;
}): Promise<boolean> => {
  if (!isSynaraDesktop() || !args.experimentalWidgetsEnabled) return false;
  try {
    const result = await invokeDesktopWithAvailability('matrix_widget_open', {
      experimentalWidgetsEnabled: args.experimentalWidgetsEnabled,
      roomId: args.roomId,
      widgetId: args.widget.widgetId,
      name: args.widget.name,
      url: args.widget.url,
      kind: args.widget.kind,
      initOnContentLoad: args.widget.initOnContentLoad,
      receiveRoom: args.receiveRoom,
      sendRoomMessage: args.sendRoomMessage,
    });
    return result.available;
  } catch {
    return false;
  }
};

export const closeExperimentalWidgets = async (sessionId?: string): Promise<void> => {
  if (!isSynaraDesktop()) return;
  try {
    await invokeDesktopWithAvailability(
      'matrix_widget_close',
      { sessionId: sessionId ?? null },
      { suppressErrorDiagnostic: true }
    );
  } catch {
    // Teardown is best-effort and idempotent.
  }
};

export const WIDGETS_SETTINGS_PATH = 'Settings → General → Widgets';

export const widgetsSettingsDescription =
  'Default off. This device only — not synced to the account or other devices. Widgets run third-party or local web code as you. Room-state widgets cannot load localhost; agent URLs may use loopback.';

export function widgetsPanelEmptyCopy(experimentalWidgetsEnabled: boolean): string {
  if (!experimentalWidgetsEnabled) {
    return `Experimental Widgets are off. Enable them in ${WIDGETS_SETTINGS_PATH}.`;
  }
  return `No room widgets or agent URLs yet. Add an agent URL in ${WIDGETS_SETTINGS_PATH}, or wait for a room-state widget (m.widget).`;
}

export function widgetsAgentHowToCopy(): string {
  return 'Agents: add a name and an https or http://127.0.0.1 URL in Settings, Open from this panel, then optionally grant send m.room.message.';
}
