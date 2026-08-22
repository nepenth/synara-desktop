import React, { useCallback, useEffect, useState } from 'react';
import { Box, Text, IconButton, Icon, Icons, Scroll, Button, Input, Spinner } from 'folds';
import { Page, PageContent, PageHeader } from '../../../components/page';
import { SystemNotification } from './SystemNotification';
import { AllMessagesNotifications } from './AllMessages';
import { SpecialMessagesNotifications } from './SpecialMessages';
import { KeywordMessagesNotifications } from './KeywordMessages';
import { SequenceCard } from '../../../components/sequence-card';
import { SequenceCardStyle } from '../styles.css';
import { SettingTile } from '../../../components/setting-tile';
import { isNativeMatrixSession } from '../../verification/nativeVerification';
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
          size="300"
          variant={mode === value ? 'Primary' : 'Secondary'}
          fill={mode === value ? 'Solid' : 'Soft'}
          outlined
          radii="300"
          disabled={disabled}
          onClick={() => onChange(mode)}
        >
          <Text size="T300">{MODE_LABEL[mode]}</Text>
        </Button>
      ))}
    </Box>
  );
}

function NativePushRulesEditor() {
  const [snapshot, setSnapshot] = useState<NativePushRulesSnapshot | null>(null);
  const [keyword, setKeyword] = useState('');
  const [message, setMessage] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setSnapshot(await nativePushRulesSnapshot());
  }, []);

  useEffect(() => {
    refresh().catch(() => {
      setMessage('Push rules could not be loaded.');
    });
  }, [refresh]);

  const [writeState, write] = useAsyncCallback(
    useCallback(
      async (task: () => Promise<void>) => {
        await task();
        await refresh();
      },
      [refresh]
    )
  );
  const busy = writeState.status === AsyncStatus.Loading;

  const setDefault = (encrypted: boolean, oneToOne: boolean, mode: NativePushRuleMode) => {
    write(() => nativePushRulesSetDefault(encrypted, oneToOne, mode));
  };

  return (
    <Box direction="Column" gap="700">
      <Box direction="Column" gap="100">
        <Text size="L400">All Messages</Text>
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
      </Box>

      <Box direction="Column" gap="100">
        <Text size="L400">Mentions</Text>
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
                    size="300"
                    variant="Secondary"
                    fill="Soft"
                    outlined
                    radii="300"
                    disabled={busy}
                    onClick={() =>
                      write(() => nativePushRulesSetMention(ruleId, !snapshot.mentions[ruleId]))
                    }
                  >
                    <Text size="T300">{snapshot.mentions[ruleId] ? 'On' : 'Off'}</Text>
                  </Button>
                )
              }
            />
          </SequenceCard>
        ))}
      </Box>

      <Box direction="Column" gap="100">
        <Text size="L400">Keyword Messages</Text>
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
                write(async () => {
                  await nativePushRulesAddKeyword(next);
                  setKeyword('');
                });
              }}
            >
              <Box grow="Yes" direction="Column">
                <Input
                  value={keyword}
                  onChange={(evt) => setKeyword(evt.currentTarget.value)}
                  variant="Secondary"
                  radii="300"
                  readOnly={busy}
                />
              </Box>
              <Button
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
                  onClick={() => write(() => nativePushRulesRemoveKeyword(item))}
                  size="300"
                  radii="Pill"
                  variant="Secondary"
                  disabled={busy}
                >
                  <Icon src={Icons.Cross} size="100" />
                </IconButton>
              }
            />
          </SequenceCard>
        ))}
      </Box>
      {message && <Text size="T200">{message}</Text>}
    </Box>
  );
}

type NotificationsProps = {
  requestClose: () => void;
};
export function Notifications({ requestClose }: NotificationsProps) {
  return (
    <Page>
      <PageHeader outlined={false}>
        <Box grow="Yes" gap="200">
          <Box grow="Yes" alignItems="Center" gap="200">
            <Text size="H3" truncate>
              Notifications
            </Text>
          </Box>
          <Box shrink="No">
            <IconButton onClick={requestClose} variant="Surface">
              <Icon src={Icons.Cross} />
            </IconButton>
          </Box>
        </Box>
      </PageHeader>
      <Box grow="Yes">
        <Scroll hideTrack visibility="Hover">
          <PageContent>
            <Box direction="Column" gap="700">
              <SystemNotification />
              {isNativeMatrixSession() ? (
                <NativePushRulesEditor />
              ) : (
                <>
                  <AllMessagesNotifications />
                  <SpecialMessagesNotifications />
                  <KeywordMessagesNotifications />
                  <Box direction="Column" gap="100">
                    <Text size="L400">Block Messages</Text>
                    <SequenceCard
                      className={SequenceCardStyle}
                      variant="SurfaceVariant"
                      direction="Column"
                      gap="400"
                    >
                      <SettingTile
                        description={
                          'This option has been moved to "Account > Blocked Users" section.'
                        }
                      />
                    </SequenceCard>
                  </Box>
                </>
              )}
            </Box>
          </PageContent>
        </Scroll>
      </Box>
    </Page>
  );
}
