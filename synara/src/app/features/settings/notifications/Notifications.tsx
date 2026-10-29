import React, { useCallback, useEffect, useState } from 'react';
import { Box, Text, IconButton, Icon, Icons, Button, Input, Spinner, color } from 'folds';
import { SystemNotification } from './SystemNotification';
import { AgentNotifications } from './AgentNotifications';
import { SequenceCard } from '../../../components/sequence-card';
import { SequenceCardStyle, SettingsQuietControl } from '../styles.css';
import { SettingTile } from '../../../components/setting-tile';
import { AsyncStatus, useAsyncCallback } from '../../../hooks/useAsyncCallback';
import {
  nativePushRulesAddKeyword,
  nativePushRulesRemoveKeyword,
  nativePushRulesSetDefault,
  nativePushRulesSetMention,
  nativePushRulesSnapshot,
  type NativePushRuleMode,
  type NativePushRulesSnapshot,
} from './nativePushRules';
import { SettingsPage, SettingsSection } from '../../../components/settings-layout';

const MODE_LABEL: Record<NativePushRuleMode, string> = {
  all: 'All messages',
  mentions: 'Mentions and keywords',
  mute: 'Off',
};

function NativeModePicker({
  value,
  onChange,
  disabled,
}: {
  value: NativePushRuleMode;
  onChange: (mode: NativePushRuleMode) => void;
  disabled?: boolean;
}) {
  return (
    <Box gap="100">
      {(Object.keys(MODE_LABEL) as NativePushRuleMode[]).map((mode) => (
        <Button
          key={mode}
          className={SettingsQuietControl}
          size="300"
          variant="Secondary"
          fill={mode === value ? 'Soft' : 'None'}
          outlined
          radii="300"
          aria-pressed={mode === value}
          before={
            <Icon
              size="100"
              src={Icons.Check}
              aria-hidden
              style={{ visibility: mode === value ? 'visible' : 'hidden' }}
            />
          }
          disabled={disabled}
          onClick={() => onChange(mode)}
        >
          <Text size="T300">{MODE_LABEL[mode]}</Text>
        </Button>
      ))}
    </Box>
  );
}

type PushRuleSection = 'defaults' | 'mentions' | 'keywords';

function SectionError({ message }: { message?: string }) {
  if (!message) return null;
  return (
    <Text size="T200" role="alert" style={{ color: color.Critical.Main }}>
      {message}
    </Text>
  );
}

function NativePushRulesEditor() {
  const [snapshot, setSnapshot] = useState<NativePushRulesSnapshot | null>(null);
  const [keyword, setKeyword] = useState('');
  const [loadFailed, setLoadFailed] = useState(false);
  const [writeError, setWriteError] = useState<{ section: PushRuleSection; message: string }>();

  const refresh = useCallback(async () => {
    setSnapshot(await nativePushRulesSnapshot());
    setLoadFailed(false);
  }, []);

  const load = useCallback(() => {
    refresh().catch(() => setLoadFailed(true));
  }, [refresh]);

  useEffect(() => {
    load();
  }, [load]);

  const [writeState, write] = useAsyncCallback(
    useCallback(
      async (task: () => Promise<void>) => {
        await task();
        // The write succeeded; a failed readback is a load error, not a save error.
        await refresh().catch(() => setLoadFailed(true));
      },
      [refresh]
    )
  );
  const busy = writeState.status === AsyncStatus.Loading;

  /**
   * Save one change and show a failure in the section it came from. The
   * controls keep showing the last loaded state, so a failed write never
   * looks saved.
   */
  const save = (section: PushRuleSection, failure: string, task: () => Promise<void>) => {
    setWriteError(undefined);
    write(task).catch(() => setWriteError({ section, message: failure }));
  };
  const sectionError = (section: PushRuleSection) =>
    writeError?.section === section ? writeError.message : undefined;

  const setDefault = (encrypted: boolean, oneToOne: boolean, mode: NativePushRuleMode) => {
    save('defaults', "Couldn't change this notification setting. Try again.", () =>
      nativePushRulesSetDefault(encrypted, oneToOne, mode)
    );
  };

  return (
    <Box direction="Column" gap="700">
      <SettingsSection title="All Messages">
        <SequenceCard
          className={SequenceCardStyle}
          variant="SurfaceVariant"
          direction="Column"
          gap="400"
        >
          <SettingTile
            title="1-to-1 Chats"
            after={
              snapshot && (
                <NativeModePicker
                  value={snapshot.dm}
                  disabled={busy}
                  onChange={(mode) => setDefault(false, true, mode)}
                />
              )
            }
          />
        </SequenceCard>
        <SequenceCard
          className={SequenceCardStyle}
          variant="SurfaceVariant"
          direction="Column"
          gap="400"
        >
          <SettingTile
            title="1-to-1 Chats (Encrypted)"
            after={
              snapshot && (
                <NativeModePicker
                  value={snapshot.dmEncrypted}
                  disabled={busy}
                  onChange={(mode) => setDefault(true, true, mode)}
                />
              )
            }
          />
        </SequenceCard>
        <SequenceCard
          className={SequenceCardStyle}
          variant="SurfaceVariant"
          direction="Column"
          gap="400"
        >
          <SettingTile
            title="Rooms"
            after={
              snapshot && (
                <NativeModePicker
                  value={snapshot.group}
                  disabled={busy}
                  onChange={(mode) => setDefault(false, false, mode)}
                />
              )
            }
          />
        </SequenceCard>
        <SequenceCard
          className={SequenceCardStyle}
          variant="SurfaceVariant"
          direction="Column"
          gap="400"
        >
          <SettingTile
            title="Rooms (Encrypted)"
            after={
              snapshot && (
                <NativeModePicker
                  value={snapshot.groupEncrypted}
                  disabled={busy}
                  onChange={(mode) => setDefault(true, false, mode)}
                />
              )
            }
          />
        </SequenceCard>
        <SectionError message={sectionError('defaults')} />
      </SettingsSection>

      <SettingsSection title="Mentions">
        {[
          ['userMention', 'Mentions of your user ID'] as const,
          ['displayName', 'Contains your display name'] as const,
          ['userName', 'Contains your username'] as const,
          ['roomMention', 'Mentions of @room'] as const,
          ['atRoom', 'Contains @room'] as const,
        ].map(([ruleId, title]) => (
          <SequenceCard
            key={ruleId}
            className={SequenceCardStyle}
            variant="SurfaceVariant"
            direction="Column"
            gap="400"
          >
            <SettingTile
              title={title}
              after={
                snapshot && (
                  <Button
                    className={SettingsQuietControl}
                    size="300"
                    variant="Secondary"
                    fill={snapshot.mentions[ruleId] ? 'Soft' : 'None'}
                    outlined
                    radii="300"
                    aria-pressed={snapshot.mentions[ruleId]}
                    before={
                      <Icon
                        size="100"
                        src={Icons.Check}
                        aria-hidden
                        style={{ visibility: snapshot.mentions[ruleId] ? 'visible' : 'hidden' }}
                      />
                    }
                    disabled={busy}
                    onClick={() =>
                      save('mentions', `Couldn't change "${title}". Try again.`, () =>
                        nativePushRulesSetMention(ruleId, !snapshot.mentions[ruleId])
                      )
                    }
                  >
                    <Text size="T300">{snapshot.mentions[ruleId] ? 'On' : 'Off'}</Text>
                  </Button>
                )
              }
            />
          </SequenceCard>
        ))}
        <SectionError message={sectionError('mentions')} />
      </SettingsSection>

      <SettingsSection title="Keyword Messages">
        <SequenceCard
          className={SequenceCardStyle}
          variant="SurfaceVariant"
          direction="Column"
          gap="400"
        >
          <SettingTile
            title="Select Keyword"
            description="Notify when a message contains this keyword."
          >
            <Box
              as="form"
              gap="200"
              onSubmit={(evt) => {
                evt.preventDefault();
                const next = keyword.trim();
                if (!next) return;
                // The keyword stays in the field when saving fails.
                save('keywords', `Couldn't save the keyword "${next}". Try again.`, async () => {
                  await nativePushRulesAddKeyword(next);
                  setKeyword('');
                });
              }}
            >
              <Box grow="Yes" direction="Column">
                <Input
                  aria-label="New keyword"
                  value={keyword}
                  onChange={(evt) => setKeyword(evt.currentTarget.value)}
                  variant="Secondary"
                  radii="300"
                  readOnly={busy}
                />
              </Box>
              <Button
                className={SettingsQuietControl}
                size="400"
                variant="Secondary"
                fill="Soft"
                outlined
                radii="300"
                type="submit"
                disabled={busy || !keyword.trim()}
              >
                {busy && <Spinner variant="Secondary" size="300" />}
                <Text size="B400">Save</Text>
              </Button>
            </Box>
          </SettingTile>
        </SequenceCard>
        {snapshot?.keywords.map((item) => (
          <SequenceCard
            key={item}
            className={SequenceCardStyle}
            variant="SurfaceVariant"
            direction="Column"
            gap="400"
          >
            <SettingTile
              title={`"${item}"`}
              before={
                <IconButton
                  className={SettingsQuietControl}
                  onClick={() =>
                    save('keywords', `Couldn't remove the keyword "${item}". Try again.`, () =>
                      nativePushRulesRemoveKeyword(item)
                    )
                  }
                  aria-label={`Remove keyword ${item}`}
                  size="300"
                  radii="Pill"
                  variant="Secondary"
                  fill="None"
                  disabled={busy}
                >
                  <Icon src={Icons.Cross} size="100" />
                </IconButton>
              }
            />
          </SequenceCard>
        ))}
        <SectionError message={sectionError('keywords')} />
      </SettingsSection>
      {loadFailed && (
        <Box alignItems="Center" gap="200">
          <Box grow="Yes">
            <SectionError message="Couldn't load your notification rules. Retry to see the current settings." />
          </Box>
          <Button
            className={SettingsQuietControl}
            size="300"
            variant="Secondary"
            fill="Soft"
            outlined
            radii="300"
            onClick={load}
          >
            <Text size="B300">Retry</Text>
          </Button>
        </Box>
      )}
    </Box>
  );
}

type NotificationsProps = {
  requestClose: () => void;
};
export function Notifications({ requestClose }: NotificationsProps) {
  return (
    <SettingsPage title="Notifications" requestClose={requestClose}>
      <SystemNotification />
      <AgentNotifications />
      <NativePushRulesEditor />
    </SettingsPage>
  );
}
