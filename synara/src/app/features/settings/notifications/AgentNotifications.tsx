import React, { useCallback, useEffect, useId, useRef, useState } from 'react';
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
import { SettingsSection } from '../../../components/settings-layout';
import { composerErrorMessage } from '../../room/composerSendError';
import { InlineMessage } from '../../../components/inline-message';

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
  const [agentId, setAgentId] = useState('');
  const [message, setMessage] = useState<{ tone: 'success' | 'critical'; text: string }>();
  const savingRef = useRef(false);
  const agentIdInput = useId();

  const refresh = useCallback(async () => {
    if (savingRef.current) return;
    setLoading(true);
    try {
      const next = await loadAgentNotificationPreferences();
      if (!savingRef.current) setDraft(next);
    } catch (error) {
      setMessage({
        tone: 'critical',
        text: composerErrorMessage(
          error,
          'Synara could not load your agent notification settings. Try again.'
        ),
      });
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
        /* Focus refresh remains available. */
      });
    return () => {
      disposed = true;
      unsubscribe?.();
      window.removeEventListener('focus', onFocus);
    };
  }, [refresh]);

  /**
   * Every change saves to the account at once, like the rest of Settings. The
   * controls keep the last saved state when a write fails, so a failed save
   * never looks saved.
   */
  const save = async (next: AgentNotificationPreferences) => {
    if (savingRef.current) return;
    const previous = draft;
    savingRef.current = true;
    setSaving(true);
    setMessage(undefined);
    setDraft(next);
    try {
      setDraft(await saveAgentNotificationPreferences(next));
      setMessage({ tone: 'success', text: 'Saved to your account.' });
    } catch (error) {
      setDraft(previous);
      setMessage({
        tone: 'critical',
        text: composerErrorMessage(
          error,
          'Synara could not save your agent notification settings. Try again.'
        ),
      });
    } finally {
      savingRef.current = false;
      setSaving(false);
    }
  };

  return (
    <SettingsSection
      title="Agent notifications"
      description="Changes save to your account and apply on every Synara client."
    >
      <SequenceCard
        className={SequenceCardStyle}
        variant="SurfaceVariant"
        direction="Column"
        gap="400"
      >
        <SettingTile
          title="Agent accounts"
          description="Add each agent’s full Matrix user ID. These rules apply only to the listed agents. Once you add an agent, only listed agents can raise urgent approval alerts, even in muted rooms; with no agents listed, approval alerts follow each room’s notification settings."
        >
          {loading && !draft && <Spinner size="100" />}
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
                      void save({
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
                <Text size="T200" priority="300">
                  No agents selected. All message categories continue to notify.
                </Text>
              )}
              <Box
                as="form"
                gap="200"
                alignItems="End"
                onSubmit={(event) => {
                  event.preventDefault();
                  const id = agentId.trim();
                  if (id && !draft.agentUserIds.includes(id))
                    void save({ ...draft, agentUserIds: [...draft.agentUserIds, id] });
                  setAgentId('');
                }}
              >
                <Box grow="Yes" direction="Column" gap="100">
                  <Text as="label" size="L400" priority="300" htmlFor={agentIdInput}>
                    Agent Matrix user ID
                  </Text>
                  <Input
                    id={agentIdInput}
                    placeholder="@agent:example.org"
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
                  variant="Primary"
                  aria-label={title}
                  value={draft[key]}
                  disabled={saving || loading}
                  onChange={(value) => void save({ ...draft, [key]: value })}
                />
              }
            />
          ))}
        <Text size="T200" priority="300">
          Approval requests from listed agents remain eligible for urgent alerts. Turning a category
          off keeps its messages in the conversation. On iOS, encrypted background alerts may still
          appear as generic activity unless the installed build supports Apple’s Notification
          Filtering capability.
        </Text>
        {saving && (
          <Box gap="200" alignItems="Center" role="status">
            <Spinner size="100" variant="Secondary" />
            <Text size="T200" priority="300">
              Saving…
            </Text>
          </Box>
        )}
        {!saving && message && <InlineMessage tone={message.tone}>{message.text}</InlineMessage>}
      </SequenceCard>
    </SettingsSection>
  );
}
