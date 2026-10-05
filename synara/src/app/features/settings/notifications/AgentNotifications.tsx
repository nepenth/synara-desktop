import React, { useCallback, useEffect, useRef, useState } from 'react';
import { Box, Button, Input, Spinner, Switch, Text } from 'folds';
import { SequenceCard } from '../../../components/sequence-card';
import { SettingTile } from '../../../components/setting-tile';
import { SequenceCardStyle, SettingsQuietControl } from '../styles.css';
import {
  loadAgentNotificationPreferences,
  saveAgentNotificationPreferences,
  subscribeAgentNotificationPreferences,
  type AgentNotificationPreferences,
} from './nativeAgentNotificationPreferences';

const categories = [
  ['notifyToolActivity', 'Notify for tool activity', 'Messages starting with “🛠 Tool activity”.'],
  ['notifyCommentary', 'Notify for commentary', 'Messages starting with “💬 Commentary”.'],
  [
    'notifyFinalResponses',
    'Notify for final responses',
    'Other messages from the agents you select.',
  ],
] as const;

export function AgentNotifications() {
  const [draft, setDraft] = useState<AgentNotificationPreferences | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [dirty, setDirty] = useState(false);
  const [agentId, setAgentId] = useState('');
  const [message, setMessage] = useState<string | null>(null);
  const dirtyRef = useRef(false);
  const savingRef = useRef(false);

  const refresh = useCallback(async () => {
    if (dirtyRef.current || savingRef.current) return;
    setLoading(true);
    try {
      const next = await loadAgentNotificationPreferences();
      if (!dirtyRef.current && !savingRef.current) setDraft(next);
      setMessage(null);
    } catch (error) {
      setMessage(
        error instanceof Error ? error.message : 'Agent notification settings could not be loaded.'
      );
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
    const onFocus = () => {
      void refresh();
    };
    window.addEventListener('focus', onFocus);
    let disposed = false;
    let unsubscribe: (() => void) | undefined;
    subscribeAgentNotificationPreferences(onFocus)
      .then((stop) => {
        if (disposed) stop();
        else unsubscribe = stop;
      })
      .catch(() => {
        /* Explicit refresh remains available. */
      });
    return () => {
      disposed = true;
      unsubscribe?.();
      window.removeEventListener('focus', onFocus);
    };
  }, [refresh]);

  const edit = (next: AgentNotificationPreferences) => {
    dirtyRef.current = true;
    setDirty(true);
    setMessage(null);
    setDraft(next);
  };

  const save = async () => {
    if (!draft || savingRef.current) return;
    savingRef.current = true;
    setSaving(true);
    setMessage(null);
    try {
      setDraft(await saveAgentNotificationPreferences(draft));
      dirtyRef.current = false;
      setDirty(false);
      setMessage('Saved to your account.');
    } catch (error) {
      setMessage(
        error instanceof Error ? error.message : 'Agent notification settings were not saved.'
      );
    } finally {
      savingRef.current = false;
      setSaving(false);
    }
  };

  return (
    <Box direction="Column" gap="100">
      <Text size="L400">Agent Notifications</Text>
      <SequenceCard
        className={SequenceCardStyle}
        variant="SurfaceVariant"
        direction="Column"
        gap="400"
      >
        <SettingTile
          title="Agent accounts"
          description="Add each agent’s full Matrix user ID. These rules apply only to the listed agents and sync with your account across Synara clients."
        >
          {loading && <Spinner size="100" />}
          {draft && (
            <Box direction="Column" gap="200">
              {draft.agentUserIds.map((id) => (
                <Box key={id} gap="200" alignItems="Center">
                  <Box grow="Yes">
                    <Text size="T300">{id}</Text>
                  </Box>
                  <Button
                    className={SettingsQuietControl}
                    size="300"
                    variant="Secondary"
                    fill="None"
                    disabled={saving || loading}
                    onClick={() =>
                      edit({
                        ...draft,
                        agentUserIds: draft.agentUserIds.filter((item) => item !== id),
                      })
                    }
                    aria-label={`Remove ${id}`}
                  >
                    <Text size="T300">Remove</Text>
                  </Button>
                </Box>
              ))}
              {draft.agentUserIds.length === 0 && (
                <Text size="T200">
                  No agents selected. All message categories continue to notify.
                </Text>
              )}
              <Box
                as="form"
                gap="200"
                onSubmit={(event) => {
                  event.preventDefault();
                  const id = agentId.trim();
                  if (id && !draft.agentUserIds.includes(id))
                    edit({ ...draft, agentUserIds: [...draft.agentUserIds, id] });
                  setAgentId('');
                }}
              >
                <Box grow="Yes">
                  <Input
                    aria-label="Agent Matrix user ID"
                    placeholder="@hermes:example.org"
                    value={agentId}
                    onChange={(event) => setAgentId(event.currentTarget.value)}
                    readOnly={saving || loading}
                  />
                </Box>
                <Button
                  className={SettingsQuietControl}
                  size="400"
                  variant="Secondary"
                  fill="Soft"
                  type="submit"
                  disabled={saving || loading || !agentId.trim()}
                >
                  <Text size="T300">Add</Text>
                </Button>
              </Box>
            </Box>
          )}
        </SettingTile>
        {draft &&
          categories.map(([key, title, description]) => (
            <SettingTile
              key={key}
              title={title}
              description={description}
              after={
                <Switch
                  aria-label={title}
                  value={draft[key]}
                  disabled={saving || loading}
                  onChange={(value) => edit({ ...draft, [key]: value })}
                />
              }
            />
          ))}
        <Text size="T200">
          Approval requests remain eligible for urgent alerts. Turning a category off keeps its
          messages in the conversation. On iOS, encrypted background alerts may still appear as
          generic activity unless the installed build supports Apple’s Notification Filtering
          capability.
        </Text>
        <Box gap="200">
          <Button
            className={SettingsQuietControl}
            size="400"
            variant="Secondary"
            fill="Soft"
            disabled={!dirty || saving || loading}
            onClick={() => {
              void save();
            }}
          >
            <Text size="T300">{saving ? 'Saving…' : 'Save to account'}</Text>
          </Button>
          <Button
            className={SettingsQuietControl}
            size="400"
            variant="Secondary"
            fill="None"
            disabled={dirty || saving || loading}
            onClick={() => {
              void refresh();
            }}
          >
            <Text size="T300">Refresh</Text>
          </Button>
        </Box>
        {dirty && <Text size="T200">Unsaved changes.</Text>}
        {message && (
          <Text size="T200" role="status">
            {message}
          </Text>
        )}
      </SequenceCard>
    </Box>
  );
}
