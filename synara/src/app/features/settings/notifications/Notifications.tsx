import React, { useCallback, useEffect, useId, useState } from 'react';
import { Box, Text, Button, Chip, Icon, Icons, Input, Spinner, Switch } from 'folds';
import { InlineMessage } from '../../../components/inline-message';
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
    <Box>
      <InlineMessage tone="critical">{message}</InlineMessage>
    </Box>
  );
}

function NativePushRulesEditor() {
  const [snapshot, setSnapshot] = useState<NativePushRulesSnapshot | null>(null);
  const [keyword, setKeyword] = useState('');
  const [loadFailed, setLoadFailed] = useState(false);
  const [writeError, setWriteError] = useState<{ section: PushRuleSection; message: string }>();
  const keywordId = useId();

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

  const modeRow = (
    title: string,
    value: NativePushRuleMode | undefined,
    onChange: (mode: NativePushRuleMode) => void
  ) => (
    <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
      <SettingTile
        title={title}
        after={
          snapshot &&
          value && <NativeModePicker value={value} disabled={busy} onChange={onChange} />
        }
      />
    </SequenceCard>
  );

  return (
    <>
      <SettingsSection
        title="Direct messages"
        description="How much to notify for one-to-one conversations."
      >
        {modeRow('Unencrypted', snapshot?.dm, (mode) => setDefault(false, true, mode))}
        {modeRow('Encrypted', snapshot?.dmEncrypted, (mode) => setDefault(true, true, mode))}
        <SectionError message={sectionError('defaults')} />
      </SettingsSection>

      <SettingsSection title="Rooms" description="How much to notify for group rooms.">
        {modeRow('Unencrypted', snapshot?.group, (mode) => setDefault(false, false, mode))}
        {modeRow('Encrypted', snapshot?.groupEncrypted, (mode) => setDefault(true, false, mode))}
      </SettingsSection>

      <SettingsSection title="Mentions" description="Always notify when a message mentions you.">
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
          >
            <SettingTile
              title={title}
              after={
                snapshot && (
                  <Switch
                    variant="Primary"
                    aria-label={title}
                    value={snapshot.mentions[ruleId]}
                    disabled={busy}
                    onChange={(next) =>
                      save('mentions', `Couldn't change "${title}". Try again.`, () =>
                        nativePushRulesSetMention(ruleId, next)
                      )
                    }
                  />
                )
              }
            />
          </SequenceCard>
        ))}
        <SectionError message={sectionError('mentions')} />
      </SettingsSection>

      <SettingsSection
        title="Keywords"
        description="Notify when a message contains one of these words."
      >
        <SequenceCard
          className={SequenceCardStyle}
          variant="SurfaceVariant"
          direction="Column"
          gap="300"
        >
          <SettingTile>
            <Box
              as="form"
              gap="200"
              alignItems="End"
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
              <Box grow="Yes" direction="Column" gap="100">
                <Text as="label" size="L400" priority="300" htmlFor={keywordId}>
                  Add a keyword
                </Text>
                <Input
                  aria-label="New keyword"
                  id={keywordId}
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
                <Text size="B400">Add</Text>
              </Button>
            </Box>
            {snapshot && snapshot.keywords.length > 0 && (
              <Box wrap="Wrap" gap="200">
                {snapshot.keywords.map((item) => (
                  <Chip
                    key={item}
                    className={SettingsQuietControl}
                    variant="Secondary"
                    fill="None"
                    radii="Pill"
                    after={<Icon src={Icons.Cross} size="100" />}
                    onClick={() =>
                      save('keywords', `Couldn't remove the keyword "${item}". Try again.`, () =>
                        nativePushRulesRemoveKeyword(item)
                      )
                    }
                    disabled={busy}
                    aria-label={`Remove keyword ${item}`}
                  >
                    <Text size="T200">{item}</Text>
                  </Chip>
                ))}
              </Box>
            )}
          </SettingTile>
        </SequenceCard>
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
    </>
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
