import React, {
  ChangeEventHandler,
  FormEventHandler,
  useCallback,
  useEffect,
  useState,
} from 'react';
import { Box, Button, Chip, Icon, IconButton, Icons, Input, Spinner, Text, config } from 'folds';
import { SequenceCard } from '../../../components/sequence-card';
import { SequenceCardStyle } from '../styles.css';
import { SettingTile } from '../../../components/setting-tile';
import { useMatrixClient } from '../../../hooks/useMatrixClient';
import { AsyncStatus, useAsyncCallback } from '../../../hooks/useAsyncCallback';
import { isUserId } from '../../../utils/matrix';
import { useIgnoredUsers } from '../../../hooks/useIgnoredUsers';
import { useAlive } from '../../../hooks/useAlive';
import { isNativeMatrixSession } from '../../verification/nativeVerification';
import {
  nativeIgnoredUsersIgnore,
  nativeIgnoredUsersSnapshot,
  nativeIgnoredUsersUnignore,
} from './nativeIgnoredUsers';

function IgnoreUserInput({ userList }: { userList: string[] }) {
  const mx = useMatrixClient();
  const [userId, setUserId] = useState<string>('');
  const alive = useAlive();

  const [ignoreState, ignore] = useAsyncCallback(
    useCallback(
      async (uId: string) => {
        await mx.setIgnoredUsers([...userList, uId]);
      },
      [mx, userList]
    )
  );
  const ignoring = ignoreState.status === AsyncStatus.Loading;

  const handleChange: ChangeEventHandler<HTMLInputElement> = (evt) => {
    const uId = evt.currentTarget.value;
    setUserId(uId);
  };

  const handleReset = () => {
    setUserId('');
  };

  const handleSubmit: FormEventHandler<HTMLFormElement> = (evt) => {
    evt.preventDefault();
    if (ignoring) return;

    const target = evt.target as HTMLFormElement | undefined;
    const userIdInput = target?.userIdInput as HTMLInputElement | undefined;
    const uId = userIdInput?.value.trim();
    if (!uId) return;

    if (!isUserId(uId)) return;

    ignore(uId).then(() => {
      if (alive()) {
        setUserId('');
      }
    });
  };

  return (
    <Box as="form" onSubmit={handleSubmit} gap="200" aria-disabled={ignoring}>
      <Box grow="Yes" direction="Column">
        <Input
          required
          name="userIdInput"
          value={userId}
          onChange={handleChange}
          variant="Secondary"
          radii="300"
          style={{ paddingRight: config.space.S200 }}
          readOnly={ignoring}
          after={
            userId &&
            !ignoring && (
              <IconButton
                type="reset"
                onClick={handleReset}
                size="300"
                radii="300"
                variant="Secondary"
              >
                <Icon src={Icons.Cross} size="100" />
              </IconButton>
            )
          }
        />
      </Box>
      <Button
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
  );
}

function IgnoredUserChip({ userId, userList }: { userId: string; userList: string[] }) {
  const mx = useMatrixClient();
  const [unignoreState, unignore] = useAsyncCallback(
    useCallback(
      () => mx.setIgnoredUsers(userList.filter((uId) => uId !== userId)),
      [mx, userId, userList]
    )
  );

  const handleUnignore = () => unignore();

  const unIgnoring = unignoreState.status === AsyncStatus.Loading;
  return (
    <Chip
      variant="Secondary"
      radii="Pill"
      after={
        unIgnoring ? (
          <Spinner variant="Secondary" size="100" />
        ) : (
          <Icon src={Icons.Cross} size="100" />
        )
      }
      onClick={handleUnignore}
      disabled={unIgnoring}
    >
      <Text size="T200" truncate>
        {userId}
      </Text>
    </Chip>
  );
}

export function IgnoredUserList() {
  if (isNativeMatrixSession()) {
    return <NativeIgnoredUserList />;
  }
  return <LegacyIgnoredUserList />;
}

function NativeIgnoreUserInput({ onIgnore }: { onIgnore: (userId: string) => Promise<void> }) {
  const [userId, setUserId] = useState('');
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
    if (!uId || !isUserId(uId)) return;
    ignore(uId).then(() => {
      if (alive()) setUserId('');
    });
  };

  return (
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
  );
}

function NativeIgnoredUserList() {
  const [userList, setUserList] = useState<string[]>([]);
  const [loadState, load] = useAsyncCallback(
    useCallback(() => nativeIgnoredUsersSnapshot(), [])
  );

  useEffect(() => {
    load().then((ids) => {
      if (ids) setUserList(ids);
    });
  }, [load]);

  const handleIgnore = useCallback(async (userId: string) => {
    await nativeIgnoredUsersIgnore(userId);
    setUserList(await nativeIgnoredUsersSnapshot());
  }, []);

  const handleUnignore = useCallback(async (userId: string) => {
    await nativeIgnoredUsersUnignore(userId);
    setUserList(await nativeIgnoredUsersSnapshot());
  }, []);

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
    <Chip
      variant="Secondary"
      radii="Pill"
      after={
        unIgnoring ? (
          <Spinner variant="Secondary" size="100" />
        ) : (
          <Icon src={Icons.Cross} size="100" />
        )
      }
      onClick={() => unignore()}
      disabled={unIgnoring}
    >
      <Text size="T200" truncate>
        {userId}
      </Text>
    </Chip>
  );
}

function LegacyIgnoredUserList() {
  const ignoredUsers = useIgnoredUsers();

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
          description="Prevent receiving messages or invites from user by adding their userId."
        >
          <Box direction="Column" gap="300">
            <IgnoreUserInput userList={ignoredUsers} />
            {ignoredUsers.length > 0 && (
              <Box direction="Inherit" gap="100">
                <Text size="L400">Users</Text>
                <Box wrap="Wrap" gap="200">
                  {ignoredUsers.map((userId) => (
                    <IgnoredUserChip key={userId} userId={userId} userList={ignoredUsers} />
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
