import React, { FormEventHandler, useState } from 'react';
import { Box, Button, Spinner, Switch, Text } from 'folds';
import { SequenceCard } from '../../../components/sequence-card';
import { SequenceCardStyle, SettingsQuietControl } from '../styles.css';
import { SettingTile } from '../../../components/setting-tile';
import { SettingsSection } from '../../../components/settings-layout';
import { PasswordInput } from '../../../components/password-input';
import { useAuthMetadata } from '../../../hooks/useAuthMetadata';
import { openExternalUrl } from '../../../utils/appLinks';
import {
  nativeChangePassword,
  passwordFormProblem,
  PasswordChangeUnsupportedError,
} from './nativePassword';

type Status =
  { kind: 'idle' } | { kind: 'saving' } | { kind: 'done' } | { kind: 'error'; message: string };

export function Password() {
  const authMetadata = useAuthMetadata();
  const accountPage = authMetadata?.account_management_uri ?? authMetadata?.issuer;
  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [logoutDevices, setLogoutDevices] = useState(false);
  const [status, setStatus] = useState<Status>({ kind: 'idle' });
  const [managedElsewhere, setManagedElsewhere] = useState(false);
  const saving = status.kind === 'saving';

  const openAccountPage = () => {
    if (accountPage) void openExternalUrl(accountPage);
  };

  // A server that delegates accounts to OAuth (such as matrix.org) changes
  // passwords on its own account page; the Matrix API cannot.
  if (accountPage) {
    return (
      <SettingsSection title="Password">
        <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
          <SettingTile
            title="Change Password"
            description="Your homeserver manages your password on its account page."
            after={
              <Button
                className={SettingsQuietControl}
                size="300"
                variant="Secondary"
                fill="Soft"
                outlined
                radii="300"
                onClick={openAccountPage}
              >
                <Text size="B300">Open Account Page</Text>
              </Button>
            }
          />
        </SequenceCard>
      </SettingsSection>
    );
  }

  const handleSubmit: FormEventHandler<HTMLFormElement> = async (evt) => {
    evt.preventDefault();
    if (saving) return;
    const problem = passwordFormProblem(currentPassword, newPassword, confirmPassword);
    if (problem) {
      setStatus({ kind: 'error', message: problem });
      return;
    }
    setStatus({ kind: 'saving' });
    try {
      await nativeChangePassword({ currentPassword, newPassword, logoutDevices });
      setCurrentPassword('');
      setNewPassword('');
      setConfirmPassword('');
      setStatus({ kind: 'done' });
    } catch (error) {
      if (error instanceof PasswordChangeUnsupportedError) setManagedElsewhere(true);
      setStatus({
        kind: 'error',
        message: error instanceof Error ? error.message : 'The password could not be changed.',
      });
    }
  };

  return (
    <SettingsSection title="Password">
      <SequenceCard
        className={SequenceCardStyle}
        variant="SurfaceVariant"
        direction="Column"
        gap="400"
      >
        <SettingTile title="Change Password" description="Confirm with your current password.">
          <Box as="form" direction="Column" gap="300" onSubmit={handleSubmit}>
            <PasswordInput
              size="400"
              variant="Secondary"
              radii="300"
              outlined
              autoComplete="current-password"
              placeholder="Current password"
              aria-label="Current password"
              value={currentPassword}
              onChange={(evt) => setCurrentPassword(evt.currentTarget.value)}
              readOnly={saving}
            />
            <PasswordInput
              size="400"
              variant="Secondary"
              radii="300"
              outlined
              autoComplete="new-password"
              placeholder="New password"
              aria-label="New password"
              value={newPassword}
              onChange={(evt) => setNewPassword(evt.currentTarget.value)}
              readOnly={saving}
            />
            <PasswordInput
              size="400"
              variant="Secondary"
              radii="300"
              outlined
              autoComplete="new-password"
              placeholder="Confirm new password"
              aria-label="Confirm new password"
              value={confirmPassword}
              onChange={(evt) => setConfirmPassword(evt.currentTarget.value)}
              readOnly={saving}
            />
            <Box alignItems="Center" justifyContent="SpaceBetween" gap="300">
              <Text size="T300">Sign out of my other devices</Text>
              <Switch
                variant="Primary"
                value={logoutDevices}
                onChange={setLogoutDevices}
                disabled={saving}
                aria-label="Sign out of my other devices"
              />
            </Box>
            <Box alignItems="Center" gap="300">
              <Button
                className={SettingsQuietControl}
                type="submit"
                size="400"
                variant="Secondary"
                fill="Soft"
                outlined
                radii="300"
                disabled={saving}
              >
                {saving && <Spinner variant="Secondary" size="300" />}
                <Text size="B400">Change Password</Text>
              </Button>
              {managedElsewhere && accountPage && (
                <Button
                  size="400"
                  variant="Secondary"
                  fill="None"
                  radii="300"
                  onClick={openAccountPage}
                >
                  <Text size="B400">Open Account Page</Text>
                </Button>
              )}
            </Box>
            {status.kind === 'done' && (
              <Text size="T200" role="status">
                Password changed.
                {logoutDevices ? ' Your other devices were signed out.' : ''}
              </Text>
            )}
            {status.kind === 'error' && (
              <Text size="T200" role="alert">
                {status.message}
              </Text>
            )}
          </Box>
        </SettingTile>
      </SequenceCard>
    </SettingsSection>
  );
}
