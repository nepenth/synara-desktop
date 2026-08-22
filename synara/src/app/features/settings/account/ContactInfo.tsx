import React, { useCallback, useEffect, useState } from 'react';
import { Box, Text, Chip, Button, Input, Spinner, Icon, Icons } from 'folds';
import { SequenceCard } from '../../../components/sequence-card';
import { SequenceCardStyle } from '../styles.css';
import { SettingTile } from '../../../components/setting-tile';
import { useMatrixClient } from '../../../hooks/useMatrixClient';
import { AsyncStatus, useAsyncCallback } from '../../../hooks/useAsyncCallback';
import { isNativeMatrixSession } from '../../verification/nativeVerification';
import {
  nativeThreepidAddEmail,
  nativeThreepidAddEmailPassword,
  nativeThreepidDelete,
  nativeThreepidRequestEmailToken,
  nativeThreepidSnapshot,
} from './nativeThreepid';

export function ContactInformation() {
  if (isNativeMatrixSession()) {
    return <NativeContactInformation />;
  }
  return <LegacyContactInformation />;
}

function NativeContactInformation() {
  const [emails, setEmails] = useState<string[]>([]);
  const [emailDraft, setEmailDraft] = useState('');
  const [password, setPassword] = useState('');
  const [needsPassword, setNeedsPassword] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setEmails(await nativeThreepidSnapshot());
  }, []);

  useEffect(() => {
    refresh().catch(() => {
      setMessage('Contact addresses could not be loaded.');
    });
  }, [refresh]);

  const [deleteState, remove] = useAsyncCallback(
    useCallback(async (address: string) => {
      await nativeThreepidDelete(address);
      await refresh();
    }, [refresh])
  );

  const [addState, addEmail] = useAsyncCallback(
    useCallback(async () => {
      const email = emailDraft.trim();
      if (!email) return;
      await nativeThreepidRequestEmailToken(email);
      const add = await nativeThreepidAddEmail();
      if (add.status === 'authenticationRequired') {
        setNeedsPassword(true);
        setMessage('Enter your account password to confirm this email.');
        return;
      }
      setEmailDraft('');
      setNeedsPassword(false);
      setMessage('Email attached.');
      await refresh();
    }, [emailDraft, refresh])
  );

  const [passwordState, confirmPassword] = useAsyncCallback(
    useCallback(async () => {
      const add = await nativeThreepidAddEmailPassword(password);
      if (add.status !== 'ok') {
        setMessage('Could not confirm this email.');
        return;
      }
      setPassword('');
      setEmailDraft('');
      setNeedsPassword(false);
      setMessage('Email attached.');
      await refresh();
    }, [password, refresh])
  );

  const busy =
    deleteState.status === AsyncStatus.Loading ||
    addState.status === AsyncStatus.Loading ||
    passwordState.status === AsyncStatus.Loading;

  return (
    <Box direction="Column" gap="100">
      <Text size="L400">Contact Information</Text>
      <SequenceCard
        className={SequenceCardStyle}
        variant="SurfaceVariant"
        direction="Column"
        gap="400"
      >
        <SettingTile title="Email Address" description="Email address attached to your account.">
          <Box direction="Column" gap="300">
            <Box wrap="Wrap" gap="200">
              {emails.map((address) => (
                <Chip
                  key={address}
                  as="button"
                  variant="Secondary"
                  radii="Pill"
                  after={<Icon src={Icons.Cross} size="100" />}
                  onClick={() => remove(address)}
                  disabled={busy}
                >
                  <Text size="T200">{address}</Text>
                </Chip>
              ))}
            </Box>
            <Box gap="200">
              <Box grow="Yes" direction="Column">
                <Input
                  value={emailDraft}
                  onChange={(evt) => setEmailDraft(evt.currentTarget.value)}
                  variant="Secondary"
                  radii="300"
                  placeholder="you@example.org"
                  readOnly={busy}
                />
              </Box>
              <Button
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
              <Box gap="200">
                <Box grow="Yes" direction="Column">
                  <Input
                    type="password"
                    value={password}
                    onChange={(evt) => setPassword(evt.currentTarget.value)}
                    variant="Secondary"
                    radii="300"
                    placeholder="Account password"
                    readOnly={busy}
                  />
                </Box>
                <Button
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
            {message && (
              <Text size="T200">
                {message}
              </Text>
            )}
          </Box>
        </SettingTile>
      </SequenceCard>
    </Box>
  );
}

function LegacyContactInformation() {
  const mx = useMatrixClient();
  const [threePIdsState, loadThreePIds] = useAsyncCallback(
    useCallback(() => mx.getThreePids(), [mx])
  );
  const threePIds =
    threePIdsState.status === AsyncStatus.Success
      ? threePIdsState.data?.threepids ?? []
      : undefined;

  const emailIds = threePIds?.filter((id) => id.medium === 'email');

  useEffect(() => {
    loadThreePIds();
  }, [loadThreePIds]);

  return (
    <Box direction="Column" gap="100">
      <Text size="L400">Contact Information</Text>
      <SequenceCard
        className={SequenceCardStyle}
        variant="SurfaceVariant"
        direction="Column"
        gap="400"
      >
        <SettingTile title="Email Address" description="Email address attached to your account.">
          <Box>
            {emailIds?.map((email) => (
              <Chip key={email.address} as="span" variant="Secondary" radii="Pill">
                <Text size="T200">{email.address}</Text>
              </Chip>
            ))}
          </Box>
          {/* <Input defaultValue="" variant="Secondary" radii="300" /> */}
        </SettingTile>
      </SequenceCard>
    </Box>
  );
}
