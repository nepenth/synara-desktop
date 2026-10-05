import {
  invokeDesktopWithAvailability,
  listen,
  type DesktopInvokeResult,
} from '../../../utils/desktop';

export type AgentNotificationPreferences = {
  schemaVersion: 1;
  agentUserIds: string[];
  notifyToolActivity: boolean;
  notifyCommentary: boolean;
  notifyFinalResponses: boolean;
};

type PreferencesInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

const defaultInvoke: PreferencesInvoke = (command, args) =>
  invokeDesktopWithAvailability<unknown>(command, args);

export function acceptsAgentNotificationPreferences(
  value: unknown
): value is AgentNotificationPreferences {
  if (!value || typeof value !== 'object') return false;
  const body = value as AgentNotificationPreferences;
  return (
    body.schemaVersion === 1 &&
    Array.isArray(body.agentUserIds) &&
    body.agentUserIds.every((id) => typeof id === 'string') &&
    typeof body.notifyToolActivity === 'boolean' &&
    typeof body.notifyCommentary === 'boolean' &&
    typeof body.notifyFinalResponses === 'boolean'
  );
}

export async function loadAgentNotificationPreferences(
  invoke: PreferencesInvoke = defaultInvoke
): Promise<AgentNotificationPreferences> {
  const result = await invoke('matrix_agent_notification_preferences_snapshot');
  if (!result.available || !acceptsAgentNotificationPreferences(result.value)) {
    throw new Error('Agent notification settings could not be loaded.');
  }
  return result.value;
}

export async function saveAgentNotificationPreferences(
  preferences: AgentNotificationPreferences,
  invoke: PreferencesInvoke = defaultInvoke
): Promise<AgentNotificationPreferences> {
  const result = await invoke('matrix_agent_notification_preferences_set', { preferences });
  if (!result.available || !acceptsAgentNotificationPreferences(result.value)) {
    throw new Error(
      'Agent notification settings were not saved. Check your connection and agent IDs, then try again.'
    );
  }
  return result.value;
}

export async function subscribeAgentNotificationPreferences(
  onUpdate: () => void
): Promise<() => void> {
  const unlisten = await listen('matrix-agent-notification-preferences-updated', onUpdate);
  return () => {
    void unlisten?.();
  };
}
