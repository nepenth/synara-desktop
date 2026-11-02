import React, { useCallback, useEffect, useId, useState } from 'react';
import { Box, Text, Chip, Button, Input, Spinner, Icon, Icons } from 'folds';
import { SequenceCard } from '../../../components/sequence-card';
import { SequenceCardStyle, SettingsQuietControl } from '../styles.css';
import { SettingTile } from '../../../components/setting-tile';
import { AsyncStatus, useAsyncCallback } from '../../../hooks/useAsyncCallback';
import {
  nativeThreepidAddEmail,
  nativeThreepidAddEmailPassword,
  nativeThreepidDelete,
  nativeThreepidRequestEmailToken,
  nativeThreepidSnapshot,
} from './nativeThreepid';
import { SettingsSection } from '../../../components/settings-layout';
import { InlineMessage } from '../../../components/inline-message';

export function ContactInformation() {
  return <NativeContactInformation />;
}

function NativeContactInformation() {
  const [emails, setEmails] = useState<string[]>([]);
  const [emailDraft, setEmailDraft] = useState('');
  const [password, setPassword] = useState('');
  const [needsPassword, setNeedsPassword] = useState(false);
  const [message, setMessage] = useState<{ tone: 'info' | 'success' | 'critical'; text: string }>();
  const emailId = useId();
  const passwordId = useId();

  const refresh = useCallback(async () => {
    setEmails(await nativeThreepidSnapshot());
  }, []);

  useEffect(() => {
    refresh().catch(() => {
      setMessage({ tone: 'critical', text: 'Contact addresses could not be loaded.' });
    });
  }, [refresh]);

  const [deleteState, remove] = useAsyncCallback(
    useCallback(
      async (address: string) => {
        await nativeThreepidDelete(address);
        await refresh();
      },
      [refresh]
    )
  );

  const [addState, addEmail] = useAsyncCallback(
    useCallback(async () => {
      const email = emailDraft.trim();
      if (!email) return;
      await nativeThreepidRequestEmailToken(email);
      const add = await nativeThreepidAddEmail();
      if (add.status === 'authenticationRequired') {
        setNeedsPassword(true);
        setMessage({ tone: 'info', text: 'Enter your account password to confirm this email.' });
        return;
      }
      setEmailDraft('');
      setNeedsPassword(false);
      setMessage({ tone: 'success', text: 'Email attached.' });
      await refresh();
    }, [emailDraft, refresh])
  );

  const [passwordState, confirmPassword] = useAsyncCallback(
    useCallback(async () => {
      const add = await nativeThreepidAddEmailPassword(password);
      if (add.status !== 'ok') {
        setMessage({ tone: 'critical', text: 'Could not confirm this email.' });
        return;
      }
      setPassword('');
      setEmailDraft('');
      setNeedsPassword(false);
      setMessage({ tone: 'success', text: 'Email attached.' });
      await refresh();
    }, [password, refresh])
  );

  const busy =
    deleteState.status === AsyncStatus.Loading ||
    addState.status === AsyncStatus.Loading ||
    passwordState.status === AsyncStatus.Loading;

  return (
    <SettingsSection title="Contact">
      <SequenceCard
        className={SequenceCardStyle}
        variant="SurfaceVariant"
        direction="Column"
        gap="400"
      >
        <SettingTile title="Email" description="Email addresses attached to your account.">
          <Box direction="Column" gap="300">
            <Box wrap="Wrap" gap="200">
              {emails.map((address) => (
                <Chip
                  className={SettingsQuietControl}
                  key={address}
                  as="button"
                  variant="Secondary"
                  fill="None"
                  radii="Pill"
                  after={<Icon src={Icons.Cross} size="100" />}
                  onClick={() => remove(address)}
                  disabled={busy}
                >
                  <Text size="T200">{address}</Text>
                </Chip>
              ))}
            </Box>
            <Box gap="200" alignItems="End">
              <Box grow="Yes" direction="Column" gap="100">
                <Text as="label" size="L400" priority="300" htmlFor={emailId}>
                  Add an email
                </Text>
                <Input
                  id={emailId}
                  value={emailDraft}
                  onChange={(evt) => setEmailDraft(evt.currentTarget.value)}
                  variant="Secondary"
                  radii="300"
                  placeholder="you@example.org"
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
                disabled={busy || !emailDraft.trim()}
                onClick={() => addEmail()}
              >
                {addState.status === AsyncStatus.Loading && (
                  <Spinner variant="Secondary" size="300" />
                )}
                <Text size="B400">Add</Text>
              </Button>
            </Box>
            {needsPassword && (
              <Box gap="200" alignItems="End">
                <Box grow="Yes" direction="Column" gap="100">
                  <Text as="label" size="L400" priority="300" htmlFor={passwordId}>
                    Account password
                  </Text>
                  <Input
                    id={passwordId}
                    type="password"
                    value={password}
                    onChange={(evt) => setPassword(evt.currentTarget.value)}
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
                  disabled={busy || !password}
                  onClick={() => confirmPassword()}
                >
                  {passwordState.status === AsyncStatus.Loading && (
                    <Spinner variant="Secondary" size="300" />
                  )}
                  <Text size="B400">Confirm</Text>
                </Button>
              </Box>
            )}
            {message && <InlineMessage tone={message.tone}>{message.text}</InlineMessage>}
          </Box>
        </SettingTile>
      </SequenceCard>
    </SettingsSection>
  );
}
