import React, {
  ChangeEventHandler,
  FormEventHandler,
  useCallback,
  useEffect,
  useState,
} from 'react';
import { Box, Button, Chip, Icon, Icons, Input, Spinner, Text, color } from 'folds';
import { SequenceCard } from '../../../components/sequence-card';
import { SequenceCardStyle, SettingsQuietControl } from '../styles.css';
import { SettingTile } from '../../../components/setting-tile';
import { AsyncStatus, useAsyncCallback } from '../../../hooks/useAsyncCallback';
import { isUserId } from '../../../utils/matrix';
import { useAlive } from '../../../hooks/useAlive';
import {
  nativeIgnoredUsersIgnore,
  nativeIgnoredUsersSnapshot,
  nativeIgnoredUsersUnignore,
} from './nativeIgnoredUsers';

export function IgnoredUserList() {
  return <NativeIgnoredUserList />;
}

function InlineError({ message }: { message: string }) {
  return (
    <Text size="T200" role="alert" style={{ color: color.Critical.Main }}>
      {message}
    </Text>
  );
}

function NativeIgnoreUserInput({ onIgnore }: { onIgnore: (userId: string) => Promise<void> }) {
  const [userId, setUserId] = useState('');
  const [error, setError] = useState<string>();
  const alive = useAlive();
  const [ignoreState, ignore] = useAsyncCallback(onIgnore);
  const ignoring = ignoreState.status === AsyncStatus.Loading;

  const handleChange: ChangeEventHandler<HTMLInputElement> = (evt) => {
    setUserId(evt.currentTarget.value);
  };

  const handleSubmit: FormEventHandler<HTMLFormElement> = (evt) => {
    evt.preventDefault();
    if (ignoring) return;
    const uId = userId.trim();
    if (!uId) return;
    if (!isUserId(uId)) {
      setError('Enter a full user ID, like @name:example.org.');
      return;
    }
    setError(undefined);
    // The typed ID stays in the field when blocking fails, ready for a retry.
    ignore(uId).then(
      () => {
        if (alive()) setUserId('');
      },
      () => {
        if (alive()) setError(`Couldn't block ${uId}. Try again.`);
      }
    );
  };

  return (
    <Box direction="Column" gap="100">
      <Box as="form" onSubmit={handleSubmit} gap="200" aria-disabled={ignoring}>
        <Box grow="Yes" direction="Column">
          <Input
            required
            name="userIdInput"
            value={userId}
            onChange={handleChange}
            variant="Secondary"
            radii="300"
            readOnly={ignoring}
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
          disabled={ignoring}
        >
          {ignoring && <Spinner variant="Secondary" size="300" />}
          <Text size="B400">Block</Text>
        </Button>
      </Box>
      {error && <InlineError message={error} />}
    </Box>
  );
}

function NativeIgnoredUserList() {
  const [userList, setUserList] = useState<string[]>([]);
  const [loadState, load] = useAsyncCallback(useCallback(() => nativeIgnoredUsersSnapshot(), []));
  const loadFailed = loadState.status === AsyncStatus.Error;

  const reload = useCallback(() => {
    // A failed load is rendered from `loadState`; it must not look like an empty list.
    load().then(setUserList, () => {});
  }, [load]);

  useEffect(() => {
    reload();
  }, [reload]);

  const handleIgnore = useCallback(
    async (userId: string) => {
      await nativeIgnoredUsersIgnore(userId);
      // The block succeeded; a failed refresh only shows the list error.
      setUserList((current) => (current.includes(userId) ? current : [...current, userId]));
      reload();
    },
    [reload]
  );

  const handleUnignore = useCallback(
    async (userId: string) => {
      await nativeIgnoredUsersUnignore(userId);
      setUserList((current) => current.filter((id) => id !== userId));
      reload();
    },
    [reload]
  );

  return (
    <Box direction="Column" gap="100">
      <Box alignItems="Center" justifyContent="SpaceBetween" gap="200">
        <Text size="L400">Blocked Users</Text>
      </Box>
      <SequenceCard
        className={SequenceCardStyle}
        variant="SurfaceVariant"
        direction="Column"
        gap="400"
      >
        <SettingTile
          title="Select User"
          description="Prevent receiving messages or invites from a user by adding their user ID."
        >
          <Box direction="Column" gap="300">
            <NativeIgnoreUserInput onIgnore={handleIgnore} />
            {loadState.status === AsyncStatus.Loading && userList.length === 0 && (
              <Spinner variant="Secondary" size="300" />
            )}
            {loadFailed && (
              <Box alignItems="Center" gap="200">
                <Box grow="Yes">
                  <InlineError message="Couldn't load your blocked users. The list below may be incomplete." />
                </Box>
                <Button
                  className={SettingsQuietControl}
                  size="300"
                  variant="Secondary"
                  fill="Soft"
                  outlined
                  radii="300"
                  onClick={reload}
                >
                  <Text size="B300">Retry</Text>
                </Button>
              </Box>
            )}
            {userList.length > 0 && (
              <Box direction="Inherit" gap="100">
                <Text size="L400">Users</Text>
                <Box wrap="Wrap" gap="200">
                  {userList.map((userId) => (
                    <NativeIgnoredUserChip
                      key={userId}
                      userId={userId}
                      onUnignore={handleUnignore}
                    />
                  ))}
                </Box>
              </Box>
            )}
          </Box>
        </SettingTile>
      </SequenceCard>
    </Box>
  );
}

function NativeIgnoredUserChip({
  userId,
  onUnignore,
}: {
  userId: string;
  onUnignore: (userId: string) => Promise<void>;
}) {
  const [unignoreState, unignore] = useAsyncCallback(
    useCallback(() => onUnignore(userId), [onUnignore, userId])
  );
  const unIgnoring = unignoreState.status === AsyncStatus.Loading;
  return (
    <Box direction="Column" gap="100">
      <Chip
        className={SettingsQuietControl}
        variant="Secondary"
        fill="None"
        radii="Pill"
        after={
          unIgnoring ? (
            <Spinner variant="Secondary" size="100" />
          ) : (
            <Icon src={Icons.Cross} size="100" />
          )
        }
        onClick={() => {
          // The failure is rendered from `unignoreState`.
          unignore().catch(() => {});
        }}
        disabled={unIgnoring}
        aria-label={`Unblock ${userId}`}
      >
        <Text size="T200" truncate>
          {userId}
        </Text>
      </Chip>
      {unignoreState.status === AsyncStatus.Error && (
        <InlineError message={`Couldn't unblock ${userId}. Try again.`} />
      )}
    </Box>
  );
}
